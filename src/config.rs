
//! read configuration from a file

use crate::errors::Error;

#[derive(serde::Deserialize)]
pub struct Config {
    pub user: String,
    pub account: String,
    pub url: String,
    pub private_key: String,
    pub private_key_path: String,
    pub jwt_token: String,
}

pub(crate) fn read_config(path: &str) -> Result<Config, Error> {
    let config = if path == "ENV" {
        Config {
            user: std::env::var("SNOWFLAKE_USERNAME")
                .map_err(|_| Error::Config("Missing SNOWFLAKE_USERNAME env var".to_string()))?,
            account: std::env::var("SNOWFLAKE_ACCOUNT")
                .map_err(|_| Error::Config("Missing SNOWFLAKE_ACCOUNT env var".to_string()))?,
            url: std::env::var("SNOWFLAKE_URL")
                .map_err(|_| Error::Config("Missing SNOWFLAKE_URL env var".to_string()))?,
            private_key: "".to_string(),
            private_key_path: "".to_string(),
            jwt_token: std::env::var("SNOWFLAKE_JWT_TOKEN").map_err(|_| {
                Error::Config("Missing SNOWFLAKE_JWT_TOKEN env var".to_string())
            })?,
        }
    } else {
        let contents = std::fs::read_to_string(path)?;
        serde_json::from_str(&contents)?
    };
    Ok(config)
}