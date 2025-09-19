use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::Config;
use crate::client::crypto::JwtContext;
use reqwest::Client;
use std::time::Duration;

pub(crate) mod crypto;
mod impls;

#[derive(Clone)]
pub struct StreamingIngestClient<R> {
    _marker: PhantomData<R>,
    pub db_name: String,
    pub schema_name: String,
    pub pipe_name: String,
    pub account: String,
    control_host: String,
    auth_state: AuthTokenState,
    auth_config: Config,
    retry_on_unauthorized: bool,
    backoff_delay: Duration,
    http_client: Client,
    auth_token_type: String,
    pub ingest_host: Option<String>,
    pub scoped_token: Arc<Mutex<Option<String>>>,
}

#[derive(Clone)]
enum AuthTokenState {
    Managed(Arc<Mutex<JwtContext>>),
    Provided { token: String },
}
