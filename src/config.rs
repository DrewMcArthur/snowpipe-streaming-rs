//! read configuration from a file

use aws_config::BehaviorVersion;

use crate::errors::Error;

pub enum ConfigLocation {
    File(String),
    Env,
    Secret,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
pub struct Config {
    pub user: String,
    pub account: String,
    pub url: String,
    #[serde(default)]
    pub jwt_token: String,
    pub private_key: Option<String>,
    pub private_key_path: Option<String>,
    pub private_key_passphrase: Option<String>,
    pub jwt_exp_secs: Option<u64>,
}

pub(crate) async fn read_config(loc: ConfigLocation) -> Result<Config, Error> {
    let config = match loc {
        ConfigLocation::File(path) => {
            let contents = std::fs::read_to_string(path)?;
            serde_json::from_str(&contents)?
        }
        ConfigLocation::Env => read_config_from_env()?,
        ConfigLocation::Secret => read_config_from_secret().await?,
    };
    Ok(config)
}

fn read_config_from_env() -> Result<Config, Error> {
    Ok(Config {
        user: std::env::var("SNOWFLAKE_USERNAME")
            .map_err(|_| Error::Config("Missing SNOWFLAKE_USERNAME env var".to_string()))?,
        account: std::env::var("SNOWFLAKE_ACCOUNT")
            .map_err(|_| Error::Config("Missing SNOWFLAKE_ACCOUNT env var".to_string()))?,
        url: std::env::var("SNOWFLAKE_URL")
            .map_err(|_| Error::Config("Missing SNOWFLAKE_URL env var".to_string()))?,
        private_key: std::env::var("SNOWFLAKE_PRIVATE_KEY").ok(),
        private_key_path: std::env::var("SNOWFLAKE_PRIVATE_KEY_PATH").ok(),
        private_key_passphrase: std::env::var("SNOWFLAKE_PRIVATE_KEY_PASSPHRASE").ok(),
        jwt_exp_secs: std::env::var("SNOWFLAKE_JWT_EXP_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok()),
        jwt_token: std::env::var("SNOWFLAKE_JWT_TOKEN")
            .map_err(|_| Error::Config("Missing SNOWFLAKE_JWT_TOKEN env var".to_string()))?,
    })
}

async fn read_config_from_secret() -> Result<Config, Error> {
    let secret_arn = std::env::var("SNOWFLAKE_CONFIG_SECRET_ARN")
        .map_err(|_| Error::Config("Missing PROFILE_CONFIG_SECRET_ARN env var".to_string()))?;
    let client = aws_sdk_secretsmanager::Client::new(
        &aws_config::load_defaults(BehaviorVersion::latest()).await,
    );
    let resp = client
        .get_secret_value()
        .secret_id(secret_arn)
        .send()
        .await
        .map_err(|e| Error::Config(format!("Failed to get secret: {}", e)))?;
    let secret = match resp.secret_string() {
        Some(s) => Ok(s),
        None => Err(Error::Config(
            "Failed to get secret string, returned None".to_string(),
        )),
    }?;
    let config: Config = serde_json::from_str(secret)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn env_success() {
        let _g = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("SNOWFLAKE_USERNAME", "user");
            std::env::set_var("SNOWFLAKE_ACCOUNT", "acct");
            std::env::set_var("SNOWFLAKE_URL", "https://example");
            std::env::set_var("SNOWFLAKE_JWT_TOKEN", "jwt");
        }
        let cfg = read_config_from_env().expect("env config");
        assert_eq!(cfg.user, "user");
        assert_eq!(cfg.account, "acct");
        assert_eq!(cfg.url, "https://example");
        assert_eq!(cfg.jwt_token, "jwt");
    }

    #[test]
    fn env_missing_vars() {
        let _g = ENV_LOCK.lock().unwrap();
        // Clear vars and test each missing case
        unsafe {
            std::env::remove_var("SNOWFLAKE_USERNAME");
            std::env::remove_var("SNOWFLAKE_ACCOUNT");
            std::env::remove_var("SNOWFLAKE_URL");
            std::env::remove_var("SNOWFLAKE_JWT_TOKEN");
        }
        assert!(matches!(read_config_from_env(), Err(Error::Config(_))));
    }
}
