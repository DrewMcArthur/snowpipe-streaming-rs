# Contract: Client

Constructors
- Client::new(auth: AuthConfig) -> Result<Client, Error>
  - Behavior: builds and caches JWT during construction
- AuthConfig::with_key_path(account: &str, login_name: &str, path: &str) -> Self
- AuthConfig::with_inline_key(account: &str, login_name: &str, pem: &str) -> Self
- AuthConfig::with_passphrase(pass: Option<&str>) -> Self
- AuthConfig::with_expiry_secs(secs: u64) -> Self
- AuthConfig::with_audience(aud: &str) -> Self

Behavior
- token() -> &str
  - Returns current cached JWT (refreshes if within 5m of exp)
- execute(request) -> Response
  - Attaches Authorization header; on expired-token error, refresh and retry once

Errors
- ConfigError: missing fields, invalid expiry
- AuthError: key parsing/signing/claims
- HttpError: request/response errors (pass-through)
