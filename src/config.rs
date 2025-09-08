/// read configuration from a file

use crate::errors::Error;

#[derive(serde::Deserialize)]
pub struct Config {
    pub user: String,
    pub account: String,
    pub url: String,
    pub private_key: String,
    pub jwt_token: String,
}

pub(crate) fn read_config(path: &str) -> Result<Config, Error> {
    let contents = std::fs::read_to_string(path)?;
    let config = serde_json::from_str(&contents)?;
    Ok(config)
}