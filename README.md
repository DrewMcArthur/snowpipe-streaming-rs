# Snowpipe Streaming Rust SDK

**WARNING: DEFINITELY NOT PRODUCTION READY**

This library provides a Rust client to stream data to Snowflake via the Snowpipe Streaming API. It mirrors the Python SDK’s ergonomics where reasonable, implemented against the public REST documentation. The official Python and Java SDKs aren’t open source, so this implementation follows the REST guide and observed behavior.

## References
- [Python SDK Guide](https://docs.snowflake.com/en/user-guide/snowpipe-streaming-high-performance-getting-started)
- [Python Example Usage](https://gist.github.com/sfc-gh-chathomas/a7b06bb46907bead737954d53b3a8495#file-example-py/)
- [REST Guide](https://docs.snowflake.com/en/user-guide/snowpipe-streaming-high-performance-rest-tutorial)


## Installation

This crate is not published yet. Add it via a path (local checkout) or a Git URL:

```
[dependencies]
# Local path
snowpipe-streaming = { path = "." }

# Or Git (replace with your fork/URL)
# snowpipe-streaming = { git = "https://github.com/<you>/snowpipe-streaming-rs", rev = "<sha>" }
```

Minimum supported Rust: stable toolchain compatible with edition declared in `Cargo.toml`.

## Quickstart

```
use snowpipe_streaming::{Config, StreamingIngestClient};

#[derive(serde::Serialize, Clone)]
struct Row { id: u64 }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // config.json example below
  let cfg = Config::from_file("config.json")?;
  let client = StreamingIngestClient::<Row>::new(
    "svc-client", "DB", "SCHEMA", "PIPE", cfg,
  ).await?;

  let mut ch = client.open_channel("ch").await?;
  ch.append_row(&Row { id: 1 }).await?;
  // Or batch
  let rows = vec![Row { id: 2 }, Row { id: 3 }];
  ch.append_rows_iter(rows).await?;
  ch.close().await?;
  Ok(())
}
```

Example `config.json`:
```
{
  "user": "MY_USER",
  "account": "MY_ACCOUNT",
  "url": "https://my-account-host",
  "jwt_token": "<optional, see Auth modes>",
  "private_key": "<optional PEM, or use private_key_path>",
  "private_key_path": "/path/to/private_key.pem",
  "private_key_passphrase": "<optional for encrypted keys>",
  "jwt_exp_secs": 60
}
```

## Testing
- Run all tests: `cargo test`.
- Integration tests use a local mocked HTTP server (wiremock) to emulate Snowflake endpoints; they do not require network or real credentials.
- Tests use a per-test JSON config file to avoid process-wide env races.

Example test setup (simplified):
```
use snowpipe_streaming::{Config, StreamingIngestClient};

// Write a minimal config file for the test
let cfg_path = "target/test-config.json";
std::fs::write(cfg_path, r#"{
  "user": "user",
  "account": "acct",
  "url": "http://127.0.0.1:12345", // wiremock server URI
  "jwt_token": "jwt"
}"#)?;

// Construct client using file config
let client = StreamingIngestClient::<YourRow>::new(
    "test-client", "db", "schema", "pipe", Config::from_file(cfg_path)?
).await?;
```

## Usage: Authentication

Two options are supported:

- Pre-supplied JWT: Provide `SNOWFLAKE_JWT_TOKEN` (or `jwt_token` in config). The client uses `KEYPAIR_JWT` for control-plane calls.
- Programmatic OAuth2 token generation: Provide a private key (string or file path). The client generates a control-plane access token via the Snowflake `/oauth2/token` endpoint.

Config fields (JSON file or env):
- `user` (`SNOWFLAKE_USERNAME`) – Snowflake user identifier
- `account` (`SNOWFLAKE_ACCOUNT`) – Snowflake account identifier
- `url` (`SNOWFLAKE_URL`) – Control-plane base URL
- `jwt_token` (`SNOWFLAKE_JWT_TOKEN`) – Optional; omit to enable programmatic token generation
- `private_key` (`SNOWFLAKE_PRIVATE_KEY`) – Optional PEM-encoded private key string
- `private_key_path` (`SNOWFLAKE_PRIVATE_KEY_PATH`) – Optional path to private key PEM file
- `private_key_passphrase` (`SNOWFLAKE_PRIVATE_KEY_PASSPHRASE`) – Passphrase for encrypted PKCS#8 private keys
- `jwt_exp_secs` (`SNOWFLAKE_JWT_EXP_SECS`) – Optional JWT client assertion lifetime (default 60s)

Example (programmatic):
```
{
  "user": "MY_USER",
  "account": "MY_ACCOUNT",
  "url": "https://my-account-host",
  "private_key_path": "/path/to/private_key.pem",
}
```
```
use snowpipe_streaming::{Config, StreamingIngestClient};

#[derive(serde::Serialize, Clone)]
struct Row { id: u64 }

# async fn run() -> Result<(), snowpipe_streaming::Error> {
let client = StreamingIngestClient::<Row>::new(
  "svc-client",
  "DB",
  "SCHEMA",
  "PIPE",
  Config::from_file("config.json")?,
).await?;
let mut ch = client.open_channel("ch").await?;
ch.append_row(&Row{ id: 1 }).await?;
ch.close().await?;
# Ok(())
# }
```

Close semantics:
- `StreamingIngestChannel::close()` polls until Snowflake reports commits for all appended rows.
- Warnings emit every minute after the first, and by default it times out after 5 minutes with `Error::Timeout`.
- You can override the timeout with `close_with_timeout(std::time::Duration::from_secs(30))`.

## Batching and limits
- `append_row(&T)` appends a single row.
- `append_rows_iter<I>(I)` accepts any `IntoIterator<Item = T>` and batches requests up to 16MB per HTTP call.
- Requests larger than 16MB fail with `Error::DataTooLarge(actual, max)`; adjust batch size or row size accordingly.

## Errors and logging
- Common errors: HTTP failures, invalid/missing configuration, private key parsing/decryption issues, request too large.
- Enable logs with `tracing_subscriber` in tests/examples to observe discovery, token acquisition, and ingestion progress.

## Examples
- A minimal example is available at `examples/example.rs` (requires the `unstable-example` feature).
- Integration test flows in `tests/integration.rs` demonstrate discovery, token paths, open/append/status/close.

## Compatibility
- Requires a Rust toolchain that supports the edition declared in `Cargo.toml` (2024 edition).
- Tested on recent stable Rust on macOS/Linux.

## Contributing
- PRs and issues are welcome. Keep changes minimal and focused.
- Run `cargo test` before submitting. Include/revise examples and docs as needed.

## Security
- Do not commit secrets (JWTs, private keys, passphrases). Use env vars or secure secret stores.
- Prefer `private_key_path` over embedding PEM strings in files checked into source control.

## Roadmap
- Buffered client for adaptive batching
- Builder-style client and clearer type-state for construction
- Documentation polish and doctests
