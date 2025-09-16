use reqwest::StatusCode;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Http(reqwest::Error),
    IngestHostDiscovery(StatusCode, String),
    DataTooLarge(usize, usize),
    JwtError(std::process::Output),
    Config(String),
    Timeout(std::time::Duration),
    Key(String),
    JwtSign(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err)
    }
}

impl From<std::process::Output> for Error {
    fn from(output: std::process::Output) -> Self {
        Error::JwtError(output)
    }
}
