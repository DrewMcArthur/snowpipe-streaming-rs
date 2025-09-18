use reqwest::StatusCode;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Json(serde_json::Error),
    Http(reqwest::StatusCode, String),
    Reqwest(reqwest::Error),
    IngestHostDiscovery(StatusCode, String),
    DataTooLarge(usize, usize),
    JwtError(std::process::Output),
    Config(String),
    Timeout(std::time::Duration),
    Key(String),
    JwtSign(String),
    Utf8Error(std::string::FromUtf8Error),
    Auth(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Error::Utf8Error(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Reqwest(err)
    }
}

impl From<pkcs8::Error> for Error {
    fn from(err: pkcs8::Error) -> Self {
        Error::Key(format!("PKCS#8 error: {}", err))
    }
}

impl From<rsa::pkcs1::Error> for Error {
    fn from(err: rsa::pkcs1::Error) -> Self {
        Error::Key(format!("PKCS#1 error: {}", err))
    }
}

impl From<std::process::Output> for Error {
    fn from(output: std::process::Output) -> Self {
        Error::JwtError(output)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Json(e) => write!(f, "JSON error: {}", e),
            Error::Http(e, msg) => write!(f, "HTTP error: {} {}", e, msg),
            Error::Reqwest(e) => write!(f, "Reqwest error: {}", e),
            Error::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
            Error::IngestHostDiscovery(code, body) => {
                write!(
                    f,
                    "Ingest host discovery failed: status={} body={}",
                    code, body
                )
            }
            Error::DataTooLarge(actual, max) => {
                write!(
                    f,
                    "Data too large: actual={} bytes > max={} bytes",
                    actual, max
                )
            }
            Error::JwtError(_) => write!(f, "JWT generation process failed"),
            Error::Config(msg) => write!(f, "Config error: {}", msg),
            Error::Timeout(d) => write!(f, "Operation timed out after {:?}", d),
            Error::Key(msg) => write!(f, "Key error: {}", msg),
            Error::JwtSign(msg) => write!(f, "JWT signing error: {}", msg),
            Error::Auth(msg) => write!(f, "Authentication failed: {}", msg),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Json(e) => Some(e),
            Error::Reqwest(e) => Some(e),
            _ => None,
        }
    }
}
