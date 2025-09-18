use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use pkcs8::DecodePrivateKey;
use pkcs8::der::Decode;
use reqwest::Client;
use rsa::pkcs1::EncodeRsaPrivateKey;
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    channel::StreamingIngestChannel,
    config::Config,
    errors::Error,
    request_context::RequestDispatchContext,
    retry::{JitterStrategy, OperationKind, RetryPlan},
    telemetry::refresh::RefreshTelemetry,
    token::{RefreshPolicy, TokenEnvelope, TokenSnapshot},
};

fn snapshot_from_raw(token: String, ttl_secs: u64, scoped: bool) -> TokenSnapshot {
    let issued = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    TokenSnapshot {
        value: token,
        issued_at: issued,
        expires_at: issued + ttl_secs,
        scoped,
    }
}

fn generate_assertion(audience: &str, cfg: &crate::config::Config) -> Result<TokenSnapshot, Error> {
    // Test helper: allow bypass via special prefix
    if let Some(ref raw) = cfg.private_key {
        let prefix = "TEST://assertion:";
        if let Some(rest) = raw.strip_prefix(prefix) {
            // No iat/exp context in bypass; pick small window forcing refresh paths in tests if needed
            let ttl = cfg.jwt_exp_secs.unwrap_or(3600);
            return Ok(snapshot_from_raw(rest.to_string(), ttl, false));
        }
    }
    let iss = cfg.user.clone();
    let sub = cfg.user.clone();
    let aud = audience.to_string();
    let iat_u = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| Error::Config(format!("Time error: {e}")))?
        .as_secs() as i64;
    let exp_secs = cfg.jwt_exp_secs.unwrap_or(3600) as i64;
    let exp_i = iat_u + exp_secs;
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
        iat: iat_u as usize,
        exp: exp_i as usize,
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
    Ok(TokenSnapshot {
        value: assertion,
        issued_at: iat_u as u64,
        expires_at: exp_i as u64,
        scoped: false,
    })
}

fn build_retry_plan(cfg: &Config) -> Result<RetryPlan, Error> {
    let max_attempts = cfg.retry_max_attempts.unwrap_or(4);
    if max_attempts == 0 {
        return Err(Error::Config(
            "Retry max attempts must be at least 1".into(),
        ));
    }
    let initial_delay = Duration::from_millis(cfg.retry_initial_delay_ms.unwrap_or(200));
    let multiplier = cfg.retry_multiplier.unwrap_or(1.8);
    if multiplier < 1.0 {
        return Err(Error::Config("Retry multiplier must be >= 1.0".into()));
    }
    let max_delay = Duration::from_millis(cfg.retry_max_delay_ms.unwrap_or(5_000));
    let jitter = match cfg.retry_jitter.as_deref() {
        Some(value) => JitterStrategy::from_str(value)?,
        None => JitterStrategy::Full,
    };
    Ok(RetryPlan::new(
        max_attempts,
        initial_delay,
        multiplier,
        max_delay,
        jitter,
    ))
}

fn build_refresh_policy(cfg: &Config, ttl: Duration) -> Result<RefreshPolicy, Error> {
    let threshold = cfg
        .refresh_threshold_secs
        .map(Duration::from_secs)
        .unwrap_or_else(|| RefreshPolicy::derive_threshold(ttl));
    let min_cooldown = Duration::from_secs(cfg.refresh_min_cooldown_secs.unwrap_or(300));
    let max_skew = Duration::from_secs(cfg.refresh_max_skew_secs.unwrap_or(30));
    RefreshPolicy::new(threshold, min_cooldown, max_skew)
}

#[derive(serde::Deserialize)]
struct ScopedTokenResponse {
    access_token: String,
    expires_in: Option<u64>,
}

#[derive(Clone)]
pub struct StreamingIngestClient<R> {
    _marker: PhantomData<R>,
    pub db_name: String,
    pub schema_name: String,
    pub pipe_name: String,
    pub account: String,
    control_host: String,
    auth_token_type: String,
    http_client: Client,
    control_context: RequestDispatchContext,
    ingest_context: Option<RequestDispatchContext>,
    pub ingest_host: Option<String>,
    pub scoped_token: Option<String>,
    config: Config,
    retry_plan: RetryPlan,
    clock_skew: Duration,
    scoped_token_state: Arc<RwLock<Option<TokenSnapshot>>>,
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
        let account = config.account.clone();
        let http_client = Client::new();
        let retry_plan = build_retry_plan(&config)?;

        let jwt_ttl = config.jwt_exp_secs.unwrap_or(3600);
        let control_snapshot = if config.jwt_token.is_empty() {
            // Generate a control-plane JWT locally from the provided private key
            // Audience is the control host (no /oauth2/token endpoint exists for keypair JWT)
            generate_assertion(&control_host, &config)?
        } else {
            snapshot_from_raw(config.jwt_token.clone(), jwt_ttl, false)
        };
        let control_envelope = TokenEnvelope::from_snapshot(control_snapshot.clone())?;
        let token_ttl =
            Duration::from_secs(control_snapshot.expires_at - control_snapshot.issued_at);
        let refresh_policy = build_refresh_policy(&config, token_ttl)?;
        let clock_skew = refresh_policy.max_skew;
        let control_context = RequestDispatchContext::build(
            http_client.clone(),
            retry_plan.clone(),
            control_envelope,
            refresh_policy.clone(),
            clock_skew,
        );

        let mut client = StreamingIngestClient {
            _marker: PhantomData,
            db_name: db_name.to_string(),
            schema_name: schema_name.to_string(),
            pipe_name: pipe_name.to_string(),
            account,
            control_host,
            auth_token_type: String::from("KEYPAIR_JWT"),
            http_client,
            control_context,
            ingest_context: None,
            ingest_host: None,
            scoped_token: None,
            config,
            retry_plan,
            clock_skew,
            scoped_token_state: Arc::new(RwLock::new(None)),
        };
        client.discover_ingest_host().await?;
        client.get_scoped_token().await?;
        Ok(client)
    }

    // Removed get_control_plane_token; JWT is generated locally during construction.

    async fn discover_ingest_host(&mut self) -> Result<(), Error> {
        let control_host = self.control_host.clone();
        let url = format!("{control_host}/v2/streaming/hostname");
        let guard = self.control_context.guard();
        let retry = self.control_context.retry();
        let http = self.http_client.clone();
        let auth_type = self.auth_token_type.clone();
        let config = self.config.clone();

        let (body, _) = retry
            .execute(OperationKind::DiscoverHost, move |attempt| {
                let guard = guard.clone();
                let http = http.clone();
                let url = url.clone();
                let auth_type = auth_type.clone();
                let config = config.clone();
                let control_host = control_host.clone();
                async move {
                    let telemetry =
                        RefreshTelemetry::new(format!("control.discover_ingest_host#{}", attempt));
                    let snapshot = guard
                        .ensure_fresh(
                            attempt > 1,
                            || async { generate_assertion(&control_host, &config) },
                            &telemetry,
                        )
                        .await?;
                    let token_value = snapshot.value.clone();
                    let resp = http
                        .get(&url)
                        .header("Authorization", format!("Bearer {}", token_value))
                        .header("X-Snowflake-Authorization-Token-Type", auth_type.as_str())
                        .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
                        .send()
                        .await?;
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    if status.is_success() {
                        Ok(body)
                    } else {
                        Err(Error::IngestHostDiscovery(status, body))
                    }
                }
            })
            .await?;

        info!("discover ingest host ok: host='{}'", body);
        self.ingest_host = Some(body);
        Ok(())
    }

    pub(crate) async fn get_scoped_token(&mut self) -> Result<(), Error> {
        let control_host = self.control_host.clone();
        let url = format!("{control_host}/oauth/token");
        let scope = self.ingest_host.clone().expect("Ingest host not set");
        let guard = self.control_context.guard();
        let retry = self.control_context.retry();
        let http = self.http_client.clone();
        let config = self.config.clone();

        let (snapshot, _) = retry
            .execute(OperationKind::GetScopedToken, move |attempt| {
                let guard = guard.clone();
                let http = http.clone();
                let url = url.clone();
                let scope = scope.clone();
                let config = config.clone();
                let control_host = control_host.clone();
                async move {
                    let telemetry =
                        RefreshTelemetry::new(format!("control.get_scoped_token#{}", attempt));
                    let snapshot = guard
                        .ensure_fresh(
                            attempt > 1,
                            || async { generate_assertion(&control_host, &config) },
                            &telemetry,
                        )
                        .await?;
                    let token_value = snapshot.value.clone();
                    let resp = http
                        .post(&url)
                        .header("Content-Type", "application/x-www-form-urlencoded")
                        .header("Authorization", format!("Bearer {}", token_value))
                        .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
                        .body(format!(
                            "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&scope={}",
                            scope
                        ))
                        .send()
                        .await?;
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    if !status.is_success() {
                        return Err(Error::Http(status, body));
                    }
                    let parsed = serde_json::from_str::<ScopedTokenResponse>(&body).ok();
                    let (token_value, expires_in) = if let Some(p) = parsed {
                        (p.access_token, p.expires_in.unwrap_or(3600))
                    } else {
                        (body, config.jwt_exp_secs.unwrap_or(3600))
                    };
                    let ttl = expires_in.max(60);
                    let snapshot = snapshot_from_raw(token_value, ttl, true);
                    Ok(snapshot)
                }
            })
            .await?;

        info!(
            "scoped token acquired (ttl={}s)",
            snapshot.expires_at - snapshot.issued_at
        );

        {
            let mut guard = self.scoped_token_state.write().await;
            *guard = Some(snapshot.clone());
        }
        self.scoped_token = Some(snapshot.value.clone());

        let ttl = snapshot.expires_at - snapshot.issued_at;
        let ingest_policy = build_refresh_policy(&self.config, Duration::from_secs(ttl))?;
        let ingest_envelope = TokenEnvelope::from_snapshot(snapshot.clone())?;
        self.ingest_context = Some(RequestDispatchContext::build(
            self.http_client.clone(),
            self.retry_plan.clone(),
            ingest_envelope,
            ingest_policy,
            self.clock_skew,
        ));

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
