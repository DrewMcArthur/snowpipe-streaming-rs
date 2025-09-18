use std::marker::PhantomData;

use base64::Engine;
use pkcs8::DecodePrivateKey;
use reqwest::Client;
use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey, EncodeRsaPublicKey};
use serde::Serialize;
use tracing::{error, info};

use crate::{channel::StreamingIngestChannel, config::Config, errors::Error};

/// Returns base64 encoded fingerprint of a public key derived from the RSA key.
fn compute_fingerprint(key: &rsa::RsaPublicKey) -> Result<String, Error> {
    let pkcs1 = key
        .to_pkcs1_der()
        .map_err(|e| Error::Key(format!("PKCS#1 DER encode failed: {e}")))?;
    let fingerprint = {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(pkcs1.as_bytes());
        let b64 = base64::engine::general_purpose::STANDARD.encode(hash);
        format!("SHA256:{}", b64)
    };
    Ok(fingerprint)
}

fn load_rsa_private_key_from_pem(
    pem_str: &str,
    passphrase: Option<&str>,
) -> Result<rsa::RsaPrivateKey, Error> {
    if let Ok(blocks) = pem::parse_many(pem_str.as_bytes()) {
        for block in &blocks {
            match block.tag() {
                "ENCRYPTED PRIVATE KEY" => {
                    let pass = passphrase.ok_or_else(|| {
                        Error::Key("Encrypted private key provided but no passphrase set".into())
                    })?;
                    return rsa::RsaPrivateKey::from_pkcs8_encrypted_der(block.contents(), pass)
                        .map_err(|e| Error::Key(format!("PKCS#8 decryption failed: {e}")));
                }
                "PRIVATE KEY" => {
                    return rsa::RsaPrivateKey::from_pkcs8_der(block.contents())
                        .map_err(|e| Error::Key(format!("PKCS#8 parse failed: {e}")));
                }
                "RSA PRIVATE KEY" => {
                    return rsa::RsaPrivateKey::from_pkcs1_der(block.contents())
                        .map_err(|e| Error::Key(format!("PKCS#1 parse failed: {e}")));
                }
                _ => continue,
            }
        }
    }

    if let Some(pass) = passphrase
        && let Ok(key) = rsa::RsaPrivateKey::from_pkcs8_encrypted_pem(pem_str, pass)
    {
        return Ok(key);
    }
    if let Ok(key) = rsa::RsaPrivateKey::from_pkcs8_pem(pem_str) {
        return Ok(key);
    }
    if let Ok(key) = rsa::RsaPrivateKey::from_pkcs1_pem(pem_str) {
        return Ok(key);
    }

    Err(Error::Key(
        "Invalid RSA private key: unsupported format or incorrect passphrase".into(),
    ))
}

fn generate_assertion(cfg: &Config) -> Result<String, Error> {
    // Test helper: allow bypass via special prefix
    let private_key = cfg.private_key()?;
    let prefix = "TEST://assertion:";
    if let Some(rest) = private_key.strip_prefix(prefix) {
        // No iat/exp context in bypass; pick small window forcing refresh paths in tests if needed
        return Ok(rest.to_string());
    }
    let name = match cfg.login.as_ref() {
        Some(login) => login,
        None => &cfg.user,
    };
    let rsa_key =
        load_rsa_private_key_from_pem(&private_key, cfg.private_key_passphrase.as_deref())?;
    let fingerprint = match cfg.public_key_fp.as_ref() {
        Some(fp) => fp.clone(),
        None => compute_fingerprint(&rsa_key.to_public_key())?,
    };
    let account_norm = cfg.account.to_uppercase().replace('.', "-");
    let user_norm = name.to_uppercase();
    let sub = format!("{}.{}", account_norm, user_norm);
    let iss = format!("{}.{}", sub, fingerprint);
    let iat = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Config(format!("Time error: {e}")))?
        .as_secs();
    let exp_secs = cfg.jwt_exp_secs.unwrap_or(3600);
    if exp_secs == 0 || exp_secs > 3600 {
        return Err(Error::Config(format!(
            "jwt_exp_secs must be between 1 and 3600 seconds (got {})",
            exp_secs
        )));
    }
    let exp = iat + exp_secs;

    #[derive(serde::Serialize)]
    struct Claims {
        iss: String,
        sub: String,
        iat: u64,
        exp: u64,
    }
    let claims = Claims { iss, sub, iat, exp };

    let pkcs1 = rsa_key
        .to_pkcs1_der()
        .map_err(|e| Error::Key(format!("PKCS#1 DER encode failed: {e}")))?;
    let enc_key = jsonwebtoken::EncodingKey::from_rsa_der(pkcs1.as_bytes());
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
        let jwt_token = match config.jwt_token.as_ref() {
            Some(token) if !token.is_empty() => token.clone(),
            _ => generate_assertion(&config)?,
        };
        let account = config.account.clone();
        let mut client = StreamingIngestClient {
            _marker: PhantomData,
            db_name: db_name.to_string(),
            schema_name: schema_name.to_string(),
            pipe_name: pipe_name.to_string(),
            account,
            control_host,
            jwt_token,
            auth_token_type: String::from("KEYPAIR_JWT"),
            ingest_host: None,
            scoped_token: None,
        };
        client.discover_ingest_host().await?;
        client.get_scoped_token().await?;
        Ok(client)
    }

    // Removed get_control_plane_token; JWT is generated locally during construction.

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

    pub(crate) async fn get_scoped_token(&mut self) -> Result<(), Error> {
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

#[cfg(test)]
mod tests {

    // tests/generate_assertion_tests.rs
    use std::time::{SystemTime, UNIX_EPOCH};

    use base64::Engine;
    use base64::engine::general_purpose::{self, STANDARD, URL_SAFE_NO_PAD};
    use pkcs8::{DecodePublicKey, EncodePrivateKey};
    use rand::thread_rng;
    use rsa::{RsaPrivateKey, RsaPublicKey};
    use serde_json::Value;

    use super::generate_assertion;
    use crate::Config;
    use crate::client::compute_fingerprint;

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn decode_jwt_payload(jwt: &str) -> Value {
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must have 3 segments");
        let payload_b64 = parts[1];
        let bytes = URL_SAFE_NO_PAD
            .decode(payload_b64.as_bytes())
            .expect("payload must be valid base64url (no padding)");
        serde_json::from_slice::<Value>(&bytes).expect("payload must be valid JSON")
    }

    /// A minimal PKCS#8 private key PEM for tests. You can replace with your own fixture.
    /// This is intended only to exercise signing; the test does not verify the signature.
    const TEST_PKCS8_PRIVKEY_PEM: &str = include_str!("../tests/fixtures/id_rsa.pem");

    /// Happy path: builds a JWT whose claims match Snowflake’s expectations.
    /// We assert:
    /// - `iss` contains UPPERCASE <ACCOUNT>.<USER>.SHA256:<fingerprint>
    /// - `sub` equals UPPERCASE <ACCOUNT>.<USER>
    /// - `exp - iat == jwt_exp_secs` and <= 3600
    /// - `iat` ≈ now (allow small skew)
    #[test]
    fn generates_snowflake_style_jwt_claims() {
        // Arrange
        // Example inputs you expect your library to normalize:
        let account = "xy12345.us-east-1"; // many users pass this form
        let user = "alice";
        // In unit tests we don't recompute the fingerprint; we pass the value we expect to see in `iss`.
        // Ensure it starts with "SHA256:".
        let exp_secs = 900u64;

        let cfg = Config {
            user: user.to_string(),
            login: None,
            account: account.to_string(),
            url: "https://xy12345.us-east-1.snowflakecomputing.com".to_string(),
            jwt_token: None,
            private_key: Some(TEST_PKCS8_PRIVKEY_PEM.to_string()),
            private_key_path: None,
            private_key_passphrase: None,
            public_key_fp: None,
            jwt_exp_secs: Some(exp_secs),
        };

        let t0 = now_secs();

        // Act
        let jwt = generate_assertion(&cfg).expect("should generate a JWT");
        let payload = decode_jwt_payload(&jwt);

        // Assert structure
        let iss = payload
            .get("iss")
            .and_then(|v| v.as_str())
            .expect("iss must exist");
        let sub = payload
            .get("sub")
            .and_then(|v| v.as_str())
            .expect("sub must exist");
        let iat = payload
            .get("iat")
            .and_then(|v| v.as_u64())
            .expect("iat must be u64");
        let exp = payload
            .get("exp")
            .and_then(|v| v.as_u64())
            .expect("exp must be u64");

        // Account & user should be uppercased in both iss/sub.
        let want_account_uc = account.to_uppercase().replace('.', "-"); // common normalization
        let want_user_uc = user.to_uppercase();

        // `sub` is "<ACCOUNT>.<USER>"
        assert!(
            sub == format!("{}.{}", want_account_uc, want_user_uc)
                || sub == format!("{}.{}", account.to_uppercase(), want_user_uc),
            "sub should be '<ACCOUNT>.<USER>' uppercased (with possible '.'→'-' normalization); got '{}'",
            sub
        );

        // `iss` is "<ACCOUNT>.<USER>.SHA256:<fp>"
        assert!(
            iss.contains("SHA256"),
            "iss must contain the SHA256 fingerprint; got '{}'",
            iss
        );
        assert!(
            iss.starts_with(&format!("{}.", sub)),
            "iss should start with '<ACCOUNT>.<USER>.' and then fingerprint; got '{}'",
            iss
        );

        // Time math: exp - iat == jwt_exp_secs and <= 3600
        assert_eq!(
            exp.saturating_sub(iat),
            exp_secs,
            "exp - iat must equal jwt_exp_secs"
        );
        assert!(
            exp_secs <= 3600,
            "jwt_exp_secs should be <= 3600 for Snowflake key-pair auth"
        );

        // iat should be close to now (allow generous skew for CI)
        let t1 = now_secs();
        assert!(
            iat >= t0.saturating_sub(30) && iat <= t1.saturating_add(30),
            "iat should be near 'now' (±30s); got {}, window [{}, {}]",
            iat,
            t0.saturating_sub(30),
            t1.saturating_add(30)
        );
    }

    fn to_encrypted_pem(body: &str) -> String {
        let mut out = String::with_capacity(body.len() + body.len() / 64 + 64);
        out.push_str("-----BEGIN ENCRYPTED PRIVATE KEY-----\n");
        for chunk in body.as_bytes().chunks(64) {
            out.push_str(std::str::from_utf8(chunk).expect("valid base64 chunk"));
            out.push('\n');
        }
        out.push_str("-----END ENCRYPTED PRIVATE KEY-----\n");
        out
    }

    #[test]
    fn encrypted_pkcs8_with_passphrase_parses() {
        const PASSPHRASE: &str = "test-pass";
        let mut rng = thread_rng();
        let rsa = RsaPrivateKey::new(&mut rng, 2048).expect("keygen");
        let encrypted = rsa
            .to_pkcs8_encrypted_der(&mut rng, PASSPHRASE)
            .expect("encrypt");
        let pem_body = STANDARD.encode(encrypted.as_bytes());
        let pem = to_encrypted_pem(&pem_body);

        let cfg = Config::from_values(
            "user",
            None,
            "acct",
            "https://example",
            None,
            Some(pem),
            None,
            Some(PASSPHRASE.to_string()),
            None,
            Some(60),
        );

        generate_assertion(&cfg).expect("should generate assertion with encrypted key");
    }

    #[test]
    fn correctly_generates_fingerprint() {
        let b64 = "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2RmwUycPmCSycr6WgS/NXcffCs6U025B+rT2zQDl1UWeKcSIh1TSdh7aHTyMuDaWcu3u+3+93L443D2nXJntZvcg8JV08a/QN+bI3RGdVabGL74ewqn3fuGleWYsIz3oLhse6zwbrhLGdVsD3ADOIl/nAmjOnalyuJ0fUjPgxLwRACEV5WIchVqrkG3wxRJCsj+ze8HrFMMsZ2rEtZb5XwoUiw5gbuvFhrU1y6b821Efe/ajI7h+h8qIIXcqTWSFZj93dmqWl8jUU9GkRouSVD8PrHUu0LMRNNsJ/ZC5e0u6mjVc47PyTKTUn+2q0ySoyWLRkyF0SWzqD4WI12gzIQIDAQAB";
        let der = general_purpose::STANDARD.decode(b64).unwrap();
        let pubkey = RsaPublicKey::from_public_key_der(&der).unwrap();
        let fp = "SHA256:xZx8qqibbh7x0CTGVPZNf3z463BMMn7vIoIxSUJQ/Bc=";
        assert_eq!(compute_fingerprint(&pubkey).unwrap(), fp);
    }
}
