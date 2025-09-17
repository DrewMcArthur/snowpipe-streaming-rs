# Quickstart: Client-Side JWT Auth

Overview
- Generate Snowflake JWT locally from a private key (path or inline)
- Cache for up to 1 hour; auto-refresh before expiry
- No calls to `/oauth2/token`

Usage (Rust)
```rust
use snowpipe_streaming::{Config, StreamingIngestClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::from_values(
        "user_name",             // login name
        "my_account",            // account identifier
        "https://my-account-host",
        None,                     // jwt_token (None => programmatic)
        None,                     // private_key (string)
        Some("/path/to/rsa_key.pem".into()), // private_key_path
        None,                     // private_key_passphrase
        Some(3600),               // jwt_exp_secs
    );

    let mut client = StreamingIngestClient::<serde_json::Value>::new(
        "svc-client", "DB", "SCHEMA", "PIPE", cfg,
    ).await?; // Generates JWT at construction

    // Use the client; JWT auto-attached in Authorization header
    // client.insert_rows(...).await?;

    Ok(())
}
```

Inline key (PEM string)
```rust
let cfg = Config::from_values(
    "user_name",
    "my_account",
    "https://my-account-host",
    None,
    Some(include_str!("./dev_rsa_key.pem").into()), // or read from env/secret manager
    None,
    Some("secret".into()), // if encrypted PEM
    Some(3600),
);
```

Refresh Behavior
- Proactive: refresh 5 minutes before expiration
- Reactive: on 401/403 with explicit expired/invalid token semantics, refresh and retry once

Notes
- Ensure private key never leaves process memory
- Encrypted PEM requires passphrase; unencrypted PEM supported without passphrase
