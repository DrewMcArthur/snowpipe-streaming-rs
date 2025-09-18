//! Configuration for the client

use crate::errors::Error;

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
    #[serde(default)]
    pub refresh_threshold_secs: Option<u64>,
    #[serde(default)]
    pub refresh_min_cooldown_secs: Option<u64>,
    #[serde(default)]
    pub refresh_max_skew_secs: Option<u64>,
    #[serde(default)]
    pub retry_max_attempts: Option<u8>,
    #[serde(default)]
    pub retry_initial_delay_ms: Option<u64>,
    #[serde(default)]
    pub retry_multiplier: Option<f32>,
    #[serde(default)]
    pub retry_max_delay_ms: Option<u64>,
    #[serde(default)]
    pub retry_jitter: Option<String>,
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
            refresh_threshold_secs: None,
            refresh_min_cooldown_secs: None,
            refresh_max_skew_secs: None,
            retry_max_attempts: None,
            retry_initial_delay_ms: None,
            retry_multiplier: None,
            retry_max_delay_ms: None,
            retry_jitter: None,
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
        refresh_threshold_secs: env_u64("SNOWPIPE_REFRESH_THRESHOLD_SECONDS"),
        refresh_min_cooldown_secs: env_u64("SNOWPIPE_REFRESH_MIN_COOLDOWN_SECONDS"),
        refresh_max_skew_secs: env_u64("SNOWPIPE_REFRESH_MAX_SKEW_SECONDS"),
        retry_max_attempts: std::env::var("SNOWPIPE_RETRY_MAX_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse::<u8>().ok()),
        retry_initial_delay_ms: env_u64("SNOWPIPE_RETRY_INITIAL_DELAY_MS"),
        retry_multiplier: std::env::var("SNOWPIPE_RETRY_MULTIPLIER")
            .ok()
            .and_then(|s| s.parse::<f32>().ok()),
        retry_max_delay_ms: env_u64("SNOWPIPE_RETRY_MAX_DELAY_MS"),
        retry_jitter: std::env::var("SNOWPIPE_RETRY_JITTER").ok(),
    })
}

fn env_u64(var: &str) -> Option<u64> {
    std::env::var(var).ok().and_then(|s| s.parse::<u64>().ok())
}

// AWS secret loading removed; prefer loading in app code and deserializing into Config.

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
