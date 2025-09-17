# Quickstart: Client-Side JWT Auth

Overview
- Generate Snowflake JWT locally from a private key (path or inline)
- Cache for up to 1 hour; auto-refresh before expiry
- No calls to `/oauth2/token`

Usage (Rust)
```rust
use snowpipe_streaming::{Client, AuthConfig, KeySource};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let auth = AuthConfig::with_key_path(
        "my_account",            // account identifier
        "user_name",             // login name
        "/path/to/rsa_key.pem",  // private key path
    )
    .with_audience("snowflake")
    .with_expiry_secs(3600);

    let mut client = Client::new(auth)?; // Generates JWT at construction

    // Use the client; JWT auto-attached in Authorization header
    // client.insert_rows(...).await?;

    Ok(())
}
```

Inline key (PEM string)
```rust
let auth = AuthConfig::with_inline_key(
    "my_account",
    "user_name",
    include_str!("./dev_rsa_key.pem"), // or read from env/secret manager
)
.with_passphrase(Some("secret")); // if encrypted PEM
```

Refresh Behavior
- Proactive: refresh 5 minutes before expiration
- Reactive: on 401/403 with explicit expired/invalid token semantics, refresh and retry once

Notes
- Ensure private key never leaves process memory
- Encrypted PEM requires passphrase; unencrypted PEM supported without passphrase
