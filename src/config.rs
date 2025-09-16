//! read configuration from a file

use aws_config::BehaviorVersion;

use crate::errors::Error;

#[allow(dead_code)]
pub enum ConfigLocation {
    File(String),
    Env,
    Secret,
}

#[derive(serde::Deserialize, Clone)]
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

impl Config {
    #[allow(clippy::too_many_arguments)]
    pub fn from_values(
        user: impl Into<String>,
        account: impl Into<String>,
        url: impl Into<String>,
        jwt_token: Option<String>,
        private_key: Option<String>,
        private_key_path: Option<String>,
        private_key_passphrase: Option<String>,
        jwt_exp_secs: Option<u64>,
    ) -> Self {
        Self {
            user: user.into(),
            account: account.into(),
            url: url.into(),
            jwt_token: jwt_token.unwrap_or_default(),
            private_key,
            private_key_path,
            private_key_passphrase,
            jwt_exp_secs,
        }
    }

    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        let contents = std::fs::read_to_string(path).map_err(Error::Io)?;
        let cfg = serde_json::from_str(&contents).map_err(Error::Json)?;
        Ok(cfg)
    }

    pub fn from_env() -> Result<Self, Error> {
        read_config_from_env()
    }
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
        jwt_token: std::env::var("SNOWFLAKE_JWT_TOKEN").unwrap_or_default(),
    })
}

#[allow(dead_code)]
#[deprecated(note = "Use Config::from_values/from_file/from_env; AWS secret loading deprecated")]
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
