use std::marker::PhantomData;

use reqwest::Client;
use serde::Serialize;

use crate::channel::StreamingIngestChannel;

#[derive(Clone)]
pub struct StreamingIngestClient<R> {
    _marker: PhantomData<R>,
    pub db_name: String,
    pub schema_name: String,
    pub pipe_name: String,
    pub account: String,
    control_host: String,
    jwt: Option<String>,
    pub ingest_host: Option<String>,
    pub scoped_token: Option<String>,
}

impl<R: Serialize+Clone> StreamingIngestClient<R> {
    pub async fn new(
        client_name: &str,
        db_name: &str,
        schema_name: &str,
        pipe_name: &str,
        profile_json: &str,
    ) -> Self {
        let account = "ACCOUNT".to_string(); // from profile.json
        let control_host = format!("{account}.snowflakecomputing.com");
        let mut client = StreamingIngestClient {
            _marker: PhantomData,
            db_name: db_name.to_string(),
            schema_name: schema_name.to_string(),
            pipe_name: pipe_name.to_string(),
            account,
            control_host, 
            jwt: None,
            ingest_host: None,
            scoped_token: None,
        };
        client.discover_ingest_host().await.expect("Failed to discover ingest host");
        client.get_scoped_token().await.expect("Failed to get scoped token");
        client
    }

    async fn discover_ingest_host(&mut self) -> Result<(), reqwest::Error> {
        let control_host = self.control_host.as_str(); 
        let url = format!("https://{control_host}/v2/streaming/hostname");
        // TODO: pick up from where left off, read JWT from snowsql and private key
        // https://github.com/sfc-gh-chathomas/snowpipe-streaming-examples/tree/main/REST#step-1-discover-ingest-host
        let client = Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.jwt.as_ref().expect("JWT not set")))
            .header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT")
            .send()
            .await?;

        let ingest_host = resp.text().await?;
        self.ingest_host = Some(ingest_host);
        Ok(())
    }

    async fn get_scoped_token(&mut self) -> Result<(), reqwest::Error> {
        let control_host = self.control_host.as_str();
        let url = format!("https://{control_host}/oauth/token");
        let client = Client::new();
        let resp = client.post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Authorization", format!("Bearer {}", self.jwt.as_ref().expect("JWT not set")))
            .body(format!("grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&scope={}", self.ingest_host.as_ref().expect("Ingest host not set")))
            .send()
            .await?;
        self.scoped_token = Some(resp.text().await?);
        Ok(())
    }

    pub async fn open_channel(&self, channel_name: &str) -> Result<StreamingIngestChannel<R>, reqwest::Error> {
        let ingest_host = self.ingest_host.as_ref().expect("Ingest host not set");
        let db = self.db_name.as_str();
        let schema = self.schema_name.as_str();
        let pipe = self.pipe_name.as_str();

        let client = Client::new();
        let url = format!("https://{ingest_host}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel_name}");

        let resp = client.put(&url)
            .header("Authorization", format!("Bearer {}", self.scoped_token.as_ref().expect("Scoped token not set")))
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await?
            .json()
            .await?;

        Ok(StreamingIngestChannel::from_response(self, resp, channel_name))
    }

    pub fn close(&self) {}
}
