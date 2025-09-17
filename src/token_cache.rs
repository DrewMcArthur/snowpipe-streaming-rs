#[derive(Clone)]
#[allow(dead_code)]
pub struct TokenCache {
    pub token: String,
    pub iat: i64,
    pub exp: i64,
}

impl TokenCache {
    pub fn new(token: String, iat: i64, exp: i64) -> Self {
        Self { token, iat, exp }
    }

    pub fn needs_refresh(&self, now: i64, proactive_secs: i64) -> bool {
        now >= (self.exp - proactive_secs)
    }
}
