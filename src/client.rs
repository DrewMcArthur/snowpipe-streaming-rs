use std::marker::PhantomData;

use reqwest::Client;
use serde::Serialize;

use crate::{channel::StreamingIngestChannel, config::read_config, errors::Error};

#[derive(Clone)]
pub struct StreamingIngestClient<R> {
    _marker: PhantomData<R>,
    pub db_name: String,
    pub schema_name: String,
    pub pipe_name: String,
    pub account: String,
    control_host: String,
    jwt_token: String,
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
    /// * `profile_json` - Location to read profile config from.  Use "ENV" to read from environment variables
    /// # ENV Vars
    /// * `SNOWFLAKE_JWT_TOKEN` - The JWT token to use for authentication.  This can be generated using the `snowsql` command line tool
    /// * `SNOWFLAKE_ACCOUNT` - The Snowflake account name
    /// * `SNOWFLAKE_USERNAME` - The Snowflake username
    /// * `SNOWFLAKE_URL` - The URL of the Snowflake account
    pub async fn new(
        _client_name: &str,
        db_name: &str,
        schema_name: &str,
        pipe_name: &str,
        profile_json: &str,
    ) -> Result<Self, Error> {
        let config = read_config(profile_json)?;
        let control_host = if config.url.starts_with("http") {
            config.url
        } else {
            format!("https://{}", config.url)
        };
        let jwt_token =
            std::env::var("SNOWFLAKE_JWT_TOKEN").expect("Set SNOWFLAKE_JWT_TOKEN env var with the following command:\n\n$ export SNOWFLAKE_JWT_TOKEN=$(snowsql -a ACCOUNT -u USERNAME@MATHEMATICA-MPR.COM --private-key-path ./private_key.p8 --generate-jwt)");
        let account = config.account;
        // TODO: validate control host is a valid URL
        let mut client = StreamingIngestClient {
            _marker: PhantomData,
            db_name: db_name.to_string(),
            schema_name: schema_name.to_string(),
            pipe_name: pipe_name.to_string(),
            account,
            control_host,
            jwt_token,
            ingest_host: None,
            scoped_token: None,
        };
        client
            .discover_ingest_host()
            .await
            .expect("Failed to discover ingest host");
        client
            .get_scoped_token()
            .await
            .expect("Failed to get scoped token");
        Ok(client)
    }

    async fn discover_ingest_host(&mut self) -> Result<(), Error> {
        let control_host = self.control_host.as_str();
        let url = format!("{control_host}/v2/streaming/hostname");
        // TODO: pick up from where left off, read JWT from snowsql and private key
        // https://github.com/sfc-gh-chathomas/snowpipe-streaming-examples/tree/main/REST#step-1-discover-ingest-host
        let client = Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.jwt_token))
            .header("X-Snowflake-Authorization-Token-Type", "KEYPAIR_JWT")
            .send()
            .await?
            .error_for_status()?;

        let ingest_host = resp.text().await?;
        self.ingest_host = Some(ingest_host);
        Ok(())
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
            .body(format!(
                "grant_type=urn:ietf:params:oauth:grant-type:jwt-bearer&scope={}",
                self.ingest_host.as_ref().expect("Ingest host not set")
            ))
            .send()
            .await?
            .error_for_status()?;
        self.scoped_token = Some(resp.text().await?);
        Ok(())
    }

    pub async fn open_channel(
        &self,
        channel_name: &str,
    ) -> Result<StreamingIngestChannel<R>, Error> {
        let ingest_host = self.ingest_host.as_ref().expect("Ingest host not set");
        let db = self.db_name.as_str();
        let schema = self.schema_name.as_str();
        let pipe = self.pipe_name.as_str();

        let client = Client::new();
        let url = format!(
            "https://{ingest_host}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel_name}"
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
            .body("{}")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(StreamingIngestChannel::from_response(
            self,
            resp,
            channel_name,
        ))
    }

    pub fn close(&self) {}
}

// async fn fetch_jwt(account: &str, username: &str, private_key_path: &str) -> Result<String, Error> {
//     unimplemented!("This doesn't work due to snowsql's stdout handling");
// let mut child  = Command::new("snowsql")
//     .args(&[
//         "-a",
//         account,
//         "-u",
//         username,
//         "--private-key-path",
//         private_key_path,
//         "--generate-jwt",
//     ])
//     .stdin(Stdio::piped()).spawn()?;

// if let Some(_) = child.stdin.take() { }

// let output = child.wait_with_output()?;

// if !output.status.success() {
//     return Err(Error::from(output));
// }

// let jwt_token = String::from_utf8_lossy(&output.stdout).trim().to_string();
// Ok(jwt_token)
// }
