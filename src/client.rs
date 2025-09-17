use std::marker::PhantomData;

use pkcs8::DecodePrivateKey;
use pkcs8::der::Decode;
use reqwest::Client;
use rsa::pkcs1::EncodeRsaPrivateKey;
use serde::Serialize;
use tracing::{error, info};

use crate::{channel::StreamingIngestChannel, config::Config, errors::Error};

fn generate_assertion(url: &str, cfg: &crate::config::Config) -> Result<String, Error> {
    let iss = cfg.user.clone();
    let sub = cfg.user.clone();
    let aud = url.to_string();
    let iat = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Config(format!("Time error: {e}")))?
        .as_secs() as usize;
    let exp_secs = cfg.jwt_exp_secs.unwrap_or(3600) as usize;
    let exp = iat + exp_secs;
    let jti = uuid::Uuid::new_v4().to_string();

    #[derive(serde::Serialize)]
    struct Claims {
        iss: String,
        sub: String,
        aud: String,
        iat: usize,
        exp: usize,
        jti: String,
    }
    let claims = Claims {
        iss,
        sub,
        aud,
        iat,
        exp,
        jti,
    };

    let pem_bytes = if let Some(ref key) = cfg.private_key {
        key.as_bytes().to_vec()
    } else if let Some(ref path) = cfg.private_key_path {
        std::fs::read(path).map_err(|e| Error::Key(format!("Failed reading private key: {e}")))?
    } else {
        return Err(Error::Config(
            "Missing private key for JWT generation: set SNOWFLAKE_PRIVATE_KEY or private_key/private_key_path in config"
                .into(),
        ));
    };
    // Parse PEM and handle encrypted PKCS#8
    let enc_key = match pem::parse_many(&pem_bytes) {
        Ok(blocks) if !blocks.is_empty() => {
            let block = &blocks[0];
            match block.tag() {
                "ENCRYPTED PRIVATE KEY" => {
                    let pass = cfg.private_key_passphrase.as_deref().ok_or_else(|| {
                        Error::Key("Encrypted private key provided but no passphrase set".into())
                    })?;
                    let info = pkcs8::EncryptedPrivateKeyInfo::from_der(block.contents())
                        .map_err(|e| Error::Key(format!("Encrypted PKCS#8 parse error: {e}")))?;
                    let der = info
                        .decrypt(pass)
                        .map_err(|e| Error::Key(format!("PKCS#8 decryption failed: {e}")))?;
                    let rsa = rsa::RsaPrivateKey::from_pkcs8_der(der.as_bytes())
                        .map_err(|e| Error::Key(format!("PKCS#8 to RSA parse failed: {e}")))?;
                    let pkcs1 = rsa
                        .to_pkcs1_der()
                        .map_err(|e| Error::Key(format!("PKCS#1 DER encode failed: {e}")))?;
                    jsonwebtoken::EncodingKey::from_rsa_der(pkcs1.as_bytes())
                }
                "PRIVATE KEY" => {
                    let rsa = rsa::RsaPrivateKey::from_pkcs8_der(block.contents())
                        .map_err(|e| Error::Key(format!("PKCS#8 parse failed: {e}")))?;
                    let pkcs1 = rsa
                        .to_pkcs1_der()
                        .map_err(|e| Error::Key(format!("PKCS#1 DER encode failed: {e}")))?;
                    jsonwebtoken::EncodingKey::from_rsa_der(pkcs1.as_bytes())
                }
                _ => jsonwebtoken::EncodingKey::from_rsa_pem(&pem_bytes)
                    .map_err(|e| Error::Key(format!("Invalid RSA private key: {e}")))?,
            }
        }
        _ => jsonwebtoken::EncodingKey::from_rsa_pem(&pem_bytes)
            .map_err(|e| Error::Key(format!("Invalid RSA private key: {e}")))?,
    };
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    let assertion = jsonwebtoken::encode(&header, &claims, &enc_key)
        .map_err(|e| Error::JwtSign(format!("JWT signing failed: {e}")))?;
    Ok(assertion)
}

#[derive(Clone)]
pub struct StreamingIngestClient<R> {
    _marker: PhantomData<R>,
    pub db_name: String,
    pub schema_name: String,
    pub pipe_name: String,
    pub account: String,
    control_host: String,
    jwt_token: String,
    auth_token_type: String,
    pub ingest_host: Option<String>,
    pub scoped_token: Option<String>,
}

impl<R: Serialize + Clone> StreamingIngestClient<R> {
    /// Create a new StreamingIngestClient
    /// # Arguments
    /// * `client_name` - A name for the client
    /// * `db_name` - The name of the database
    /// * `schema_name` - The name of the schema
    /// * `pipe_name` - The name of the pipe
    /// * `config` - Explicit configuration (`Config`), typically loaded via `Config::from_file` or `Config::from_env`.
    /// # ENV Vars (when using `Config::from_env`)
    /// * `SNOWFLAKE_JWT_TOKEN` - Optional pre-supplied JWT for KEYPAIR_JWT auth
    /// * `SNOWFLAKE_ACCOUNT` - Snowflake account name
    /// * `SNOWFLAKE_USERNAME` - Snowflake username
    /// * `SNOWFLAKE_URL` - Snowflake control-plane base URL
    pub async fn new(
        _client_name: &str,
        db_name: &str,
        schema_name: &str,
        pipe_name: &str,
        config: Config,
    ) -> Result<Self, Error> {
        let control_host = if config.url.starts_with("http") {
            config.url.clone()
        } else {
            format!("https://{}", config.url)
        };
        // Validate control host is a proper URL before performing any network calls
        let _ = reqwest::Url::parse(&control_host).map_err(|e| {
            Error::Config(format!(
                "Invalid control host URL '{}': {}",
                control_host, e
            ))
        })?;
        let jwt_token = config.jwt_token.clone();
        let account = config.account.clone();
        let mut client = StreamingIngestClient {
            _marker: PhantomData,
            db_name: db_name.to_string(),
            schema_name: schema_name.to_string(),
            pipe_name: pipe_name.to_string(),
            account,
            control_host,
            jwt_token: String::new(),
            auth_token_type: String::from("KEYPAIR_JWT"),
            ingest_host: None,
            scoped_token: None,
        };
        if jwt_token.is_empty() {
            client.get_control_plane_token(&config).await?;
        } else {
            client.jwt_token = jwt_token;
            client.auth_token_type = String::from("KEYPAIR_JWT");
        }
        client.discover_ingest_host().await?;
        client.get_scoped_token().await?;
        Ok(client)
    }

    async fn get_control_plane_token(&mut self, cfg: &crate::config::Config) -> Result<(), Error> {
        let control_host = self.control_host.as_str();
        let url = format!("{control_host}/oauth2/token");
        // Build client assertion JWT
        let assertion = if let Some(ref raw) = cfg.private_key {
            let prefix = "TEST://assertion:";
            if let Some(rest) = raw.strip_prefix(prefix) {
                rest.to_string()
            } else {
                generate_assertion(&url, cfg)?
            }
        } else {
            generate_assertion(&url, cfg)?
        };

        let body = format!(
            "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&client_assertion_type=urn:ietf:params:oauth:client-assertion-type:jwt-bearer&client_assertion={}",
            urlencoding::Encoded::new(assertion)
        );
        let client = Client::new();
        let resp = client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
            .body(body)
            .send()
            .await?;

        let status = resp.status();
        if ! status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Http(status, body));
        }

        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct TokenResp {
            access_token: String,
            token_type: Option<String>,
            expires_in: Option<i64>,
        }
        let tr: TokenResp = resp.json().await?;
        info!(
            "control-plane token acquired (len={})",
            tr.access_token.len()
        );
        self.jwt_token = tr.access_token;
        self.auth_token_type = String::from("OAUTH");
        Ok(())
    }

    async fn discover_ingest_host(&mut self) -> Result<(), Error> {
        let control_host = self.control_host.as_str();
        let url = format!("{control_host}/v2/streaming/hostname");
        // Control-plane discovery per REST guide:
        // https://github.com/sfc-gh-chathomas/snowpipe-streaming-examples/tree/main/REST#step-1-discover-ingest-host
        let client = Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .header(
                "X-Snowflake-Authorization-Token-Type",
                self.auth_token_type.as_str(),
            )
            .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.is_success() {
            info!("discover ingest host ok: host='{}'", body);
            self.ingest_host = Some(body);
            Ok(())
        } else {
            error!(
                "discover ingest host failed: status={} body='{}'",
                status, body
            );
            Err(Error::IngestHostDiscovery(status, body))
        }
    }

    async fn get_scoped_token(&mut self) -> Result<(), Error> {
        // remove protocol from str
        let control_host = self.control_host.as_str();
        let url = format!("{control_host}/oauth/token");
        let client = Client::new();
        let resp = client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
            .body(format!(
                "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&scope={}",
                self.ingest_host.as_ref().expect("Ingest host not set")
            ))
            .send()
            .await?
            .error_for_status()?;
        let tok = resp.text().await?;
        info!("scoped token acquired (len={})", tok.len());
        self.scoped_token = Some(tok);
        Ok(())
    }

    pub async fn open_channel(
        &self,
        channel_name: &str,
    ) -> Result<StreamingIngestChannel<R>, Error> {
        let ingest_host = self.ingest_host.as_ref().expect("Ingest host not set");
        let base = if ingest_host.contains("://") {
            ingest_host.trim_end_matches('/').to_string()
        } else {
            format!("https://{}", ingest_host)
        };
        let db = self.db_name.as_str();
        let schema = self.schema_name.as_str();
        let pipe = self.pipe_name.as_str();

        let client = Client::new();
        let url = format!(
            "{}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel_name}",
            base
        );

        let resp = client
            .put(&url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    self.scoped_token.as_ref().expect("Scoped token not set")
                ),
            )
            .header("Content-Type", "application/json")
            .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
            .body("{}")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        info!(
            "channel opened: name='{}' db='{}' schema='{}' pipe='{}'",
            channel_name, self.db_name, self.schema_name, self.pipe_name
        );

        Ok(StreamingIngestChannel::from_response(
            self,
            resp,
            channel_name,
        ))
    }

    pub fn close(&self) {}
}

// Legacy snowsql-based JWT generation was explored but not used; programmatic
// OAuth2 control-plane token acquisition is implemented instead.
