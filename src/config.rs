
//! read configuration from a file

use crate::errors::Error;

pub enum ConfigLocation {
    File(String),
    Env,
    Secret
}

#[derive(serde::Deserialize)]
pub struct Config {
    pub user: String,
    pub account: String,
    pub url: String,
    pub private_key: String,
    pub private_key_path: String,
    pub jwt_token: String,
}

pub(crate) async fn read_config(loc: ConfigLocation) -> Result<Config, Error> {
    let config = match loc {
        ConfigLocation::File(path) => {
            let contents = std::fs::read_to_string(path)?;
            serde_json::from_str(&contents)?
        },
        ConfigLocation::Env => read_config_from_env()?,
        ConfigLocation::Secret => read_config_from_secret().await?,
    };
    Ok(config)
}

fn read_config_from_env()-> Result<Config, Error> {
        Ok(Config {
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
        })
}

async fn read_config_from_secret()-> Result<Config, Error> {
    let secret_arn = std::env::var("PROFILE_CONFIG_SECRET_ARN")
        .map_err(|_| Error::Config("Missing PROFILE_CONFIG_SECRET_ARN env var".to_string()))?;
    let region = std::env::var("AWS_REGION")
        .map_err(|_| Error::Config("Missing AWS_REGION env var".to_string()))?;
    let client = aws_sdk_secretsmanager::Client::new(&aws_config::load_from_env().region(region).await);
    let resp = client
        .get_secret_value()
        .secret_id(secret_arn)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Failed to get secret: {}", e)))?;
    let secret = resp.secret_string().map_err(|e| {
        Error::Config(format!("Failed to get secret string: {}", e))
    })?;
    let config: Config = serde_json::from_str(&secret).map_err(|e| {
        Error::Config(format!("Failed to parse secret JSON: {}", e))
    })?;
    Ok(config)
}