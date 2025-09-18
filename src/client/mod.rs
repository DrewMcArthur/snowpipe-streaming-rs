use std::marker::PhantomData;

mod crypto;
mod impls;

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
