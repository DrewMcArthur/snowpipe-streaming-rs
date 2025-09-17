# Contract: JWT Auth

Operations
- build_jwt(auth: AuthConfig) -> (token: String, claims: JwtClaims)
  - Pre: valid key_source; if key encrypted then passphrase is provided
  - Post: token signed RS256; exp = iat + expiry_secs (<= 3600)
- refresh_if_needed(cache: &mut Option<TokenCache>, now: i64, expiry_secs: u64) -> bool
  - Pre: cache may be None
  - Post: returns true if refreshed; ensures cache present and not within 5m of exp
- handle_expired_error(status: u16, body_snippet: &str) -> bool
  - Post: returns true if error consistent with expired/invalid JWT and should trigger refresh+retry

Headers
- Authorization: "Snowflake JWT {token}"

Errors
- InvalidKey: encrypted key without passphrase; malformed PEM/DER; unsupported algorithm
- ClaimError: invalid audience/issuer/subject; exp <= iat
- SigningError: crypto failure
