# Quickstart: Simple JWT Refresh Logic

## Prerequisites
- Rust toolchain (1.79 or newer) with `cargo`.  
- Snowflake key pair credentials (private key + account/user identifiers) configured via the library’s standard config sources.

## Configure
1. Provide private key material only; avoid supplying raw JWTs (deprecated).  
2. Optional configuration in `config.toml`:
   ```toml
   jwt_exp_secs = 120   # will be clamped to [30, 3600] with a warning if outside range
   jwt_refresh_margin_secs = 45
   retry_on_unauthorized = true
   ```
3. Inspect logs on startup to confirm whether clamping occurred.

## Run Example (after implementation)
1. Build: `cargo build`.  
2. Execute example pipeline: `cargo run --example example --features unstable-example`.  
3. Watch logs for entries such as `refreshing JWT (margin hit)` and `retrying request after 401`.

## Validate Behavior
1. Unit tests: `cargo test client::crypto::tests::clamp_and_refresh`.  
2. Integration tests (wiremock): `cargo test client::tests::retry_behaviors`.  
3. Manual check: Temporarily set `jwt_exp_secs = 35` to observe clamping warning and automatic refresh under one minute.

## Troubleshooting
- Repeated 401s after retry indicate invalid credentials; verify private key and Snowflake grants.  
- 429 responses trigger automatic 1–3s back-off; persistent throttling suggests upstream rate limits.  
- Ensure system clock accuracy; significant skew may cause premature refresh attempts.
