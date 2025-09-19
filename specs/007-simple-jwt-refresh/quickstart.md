# Quickstart: Simple JWT Refresh Logic

## Prerequisites
- Rust toolchain (1.79 or newer) with `cargo`.  
- Snowflake key pair credentials (private key + account/user identifiers) configured via the library’s standard config sources.

## Configure
1. Provide private key material only; avoid supplying raw JWTs (deprecated).  
2. Minimal configuration example (keep only the essentials):
   ```toml
   user = "MY_USER"
   account = "MY_ACCOUNT"
   url = "https://my-account-host"
   private_key_path = "/path/to/private_key.pem"
   ```
3. Optional behaviors—such as custom JWT lifetimes, refresh margins, or retry overrides—can be added later via `jwt_exp_secs`, `jwt_refresh_margin_secs`, and `retry_on_unauthorized` if needed.  
4. Inspect logs on startup to confirm whether clamping occurred.

## Run Example (after implementation)
1. Build: `cargo build`.  
2. Execute example pipeline: `cargo run --example example --features unstable-example`.  
3. Watch logs for entries such as `refreshing JWT (margin hit)`, `retrying request after 401`, and `received 429; sleeping 2s before retry`.

## Validate Behavior
1. Unit tests: `cargo test client::crypto::tests::clamp_and_refresh`.  
2. Integration tests (wiremock): `cargo test client::tests::retry_behaviors`.  
3. Manual check: Temporarily set `jwt_exp_secs = 35` to observe clamping warning and automatic refresh under one minute.

## Troubleshooting
- Repeated 401s after retry indicate invalid credentials; verify private key and Snowflake grants.  
- 429 responses trigger an automatic 2s back-off; persistent throttling suggests upstream rate limits.  
- Ensure system clock accuracy; significant skew may cause premature refresh attempts.
