use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::Serialize;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::AuthTokenState;
use crate::{
    StreamingIngestClient, channel::StreamingIngestChannel, client::crypto::JwtContext,
    config::Config, errors::Error,
};

const USER_AGENT: &str = "snowpipe-streaming-rust-sdk/0.1.0";
const DEFAULT_REFRESH_MARGIN_SECS: u64 = 30;
const BACKOFF_DELAY_SECS: u64 = 2;

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

        let refresh_margin_secs = config
            .jwt_refresh_margin_secs
            .unwrap_or(DEFAULT_REFRESH_MARGIN_SECS);

        let auth_state = if let Some(token) = config.jwt_token.clone().filter(|t| !t.is_empty()) {
            warn!(
                "jwt_token configuration is deprecated; supply a private key so the library can refresh automatically"
            );
            AuthTokenState::Provided { token }
        } else {
            let ctx = JwtContext::new(&config, refresh_margin_secs)?;
            AuthTokenState::Managed(Arc::new(Mutex::new(ctx)))
        };

        let account = config.account.clone();
        let retry_on_unauthorized = config.retry_on_unauthorized.unwrap_or(true);
        let http_client = Client::new();

        let mut client = StreamingIngestClient {
            _marker: std::marker::PhantomData,
            db_name: db_name.to_string(),
            schema_name: schema_name.to_string(),
            pipe_name: pipe_name.to_string(),
            account,
            control_host,
            auth_state,
            auth_config: config,
            retry_on_unauthorized,
            backoff_delay: Duration::from_secs(BACKOFF_DELAY_SECS),
            http_client,
            auth_token_type: String::from("KEYPAIR_JWT"),
            ingest_host: None,
            scoped_token: Arc::new(Mutex::new(None)),
        };
        client.discover_ingest_host().await?;
        client.get_scoped_token().await?;
        Ok(client)
    }

    // Removed get_control_plane_token; JWT is generated locally during construction.

    async fn discover_ingest_host(&mut self) -> Result<(), Error> {
        let url = format!("{}/v2/streaming/hostname", self.control_host);
        let auth_type = self.auth_token_type.clone();
        let response = self
            .send_with_jwt(move |client, token| {
                client
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("X-Snowflake-Authorization-Token-Type", auth_type.as_str())
                    .header("User-Agent", USER_AGENT)
            })
            .await?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
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
        let scope = self
            .ingest_host
            .as_ref()
            .expect("Ingest host not set before requesting scoped token")
            .to_string();
        let url = format!("{}/oauth/token", self.control_host);
        let body = format!(
            "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&scope={}",
            scope
        );

        let response = self
            .send_with_jwt(move |client, token| {
                client
                    .post(&url)
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .header("Authorization", format!("Bearer {}", token))
                    .header("User-Agent", USER_AGENT)
                    .body(body.clone())
            })
            .await?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if status.is_success() {
            info!("scoped token acquired (len={})", text.len());
            *self.scoped_token.lock().await = Some(text);
            Ok(())
        } else {
            error!(
                "scoped token request failed: status={} body='{}'",
                status, text
            );
            Err(Error::Http(status, text))
        }
    }

    async fn ensure_valid_jwt(&self) -> Result<String, Error> {
        match &self.auth_state {
            AuthTokenState::Managed(ctx) => {
                let mut guard = ctx.lock().await;
                guard.ensure_valid(&self.auth_config)
            }
            AuthTokenState::Provided { token } => Ok(token.clone()),
        }
    }

    async fn invalidate_jwt(&self) {
        if let AuthTokenState::Managed(ctx) = &self.auth_state {
            let mut guard = ctx.lock().await;
            guard.invalidate();
        }
    }

    async fn send_with_jwt<F>(&self, builder: F) -> Result<reqwest::Response, Error>
    where
        F: Fn(&Client, &str) -> reqwest::RequestBuilder,
    {
        let mut unauthorized_retry = false;
        let mut rate_limit_retry = false;

        loop {
            let token = self.ensure_valid_jwt().await?;
            let response = builder(&self.http_client, &token).send().await?;
            let status = response.status();

            if status == StatusCode::UNAUTHORIZED {
                let body = response.text().await.unwrap_or_default();
                if self.retry_on_unauthorized && !unauthorized_retry {
                    warn!("received 401 from Snowflake; refreshing JWT and retrying");
                    self.invalidate_jwt().await;
                    unauthorized_retry = true;
                    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
                    continue;
                }
                warn!("received 401 from Snowflake after retry; surfacing authentication failure");
                return Err(Error::Auth(format!("401 Unauthorized: {}", body)));
            }

            if status == StatusCode::TOO_MANY_REQUESTS {
                if !rate_limit_retry {
                    warn!(
                        "received 429 TOO MANY REQUESTS; sleeping {} seconds before retry",
                        self.backoff_delay.as_secs()
                    );
                    sleep(self.backoff_delay).await;
                    rate_limit_retry = true;
                    continue;
                }
                let body = response.text().await.unwrap_or_default();
                return Err(Error::Http(status, body));
            }

            return Ok(response);
        }
    }

    pub(crate) async fn send_with_scoped_token<F>(
        &mut self,
        builder: F,
    ) -> Result<reqwest::Response, Error>
    where
        F: Fn(&Client, &str) -> reqwest::RequestBuilder,
    {
        if self.scoped_token.lock().await.is_none() {
            self.get_scoped_token().await?;
        }

        let mut unauthorized_retry = false;
        let mut rate_limit_retry = false;

        loop {
            let token = {
                let guard = self.scoped_token.lock().await;
                guard
                    .clone()
                    .expect("scoped token should be available before request")
            };

            let response = builder(&self.http_client, &token).send().await?;
            let status = response.status();

            if status == StatusCode::UNAUTHORIZED {
                let body = response.text().await.unwrap_or_default();
                if !unauthorized_retry {
                    warn!("scoped token rejected with 401; refreshing scoped token and retrying");
                    self.get_scoped_token().await?;
                    unauthorized_retry = true;
                    continue;
                }
                warn!("scoped token rejected again after retry; surfacing authentication failure");
                return Err(Error::Auth(format!("Scoped token unauthorized: {}", body)));
            }

            if status == StatusCode::TOO_MANY_REQUESTS {
                if !rate_limit_retry {
                    warn!(
                        "received 429 from ingest endpoint; sleeping {} seconds before retry",
                        self.backoff_delay.as_secs()
                    );
                    sleep(self.backoff_delay).await;
                    rate_limit_retry = true;
                    continue;
                }
                let body = response.text().await.unwrap_or_default();
                return Err(Error::Http(status, body));
            }

            return Ok(response);
        }
    }

    pub async fn open_channel(
        &mut self,
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

        let url = format!(
            "{}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel_name}",
            base
        );

        let response = self
            .send_with_scoped_token(|client, scoped| {
                client
                    .put(&url)
                    .header("Authorization", format!("Bearer {}", scoped))
                    .header("Content-Type", "application/json")
                    .header("User-Agent", USER_AGENT)
                    .body("{}")
            })
            .await?;

        let resp = response.error_for_status()?.json().await?;

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
