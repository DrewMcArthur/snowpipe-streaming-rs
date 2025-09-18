# Quickstart – JWT Automatic Refresh & Central Retry

## 1. Provide Refresh/Retry Configuration
The client automatically refreshes keypair JWTs when the remaining lifetime is at or below the lesser of 120 seconds or 20% of the original TTL. Override defaults via env vars or `config.json`:

```bash
export SNOWFLAKE_ACCOUNT=your_account
export SNOWFLAKE_USERNAME=service_user
export SNOWFLAKE_URL=https://xy12345.us-east-1.snowflakecomputing.com
export SNOWFLAKE_PRIVATE_KEY_PATH=$HOME/.secrets/snowflake_private_key.pem
export SNOWPIPE_REFRESH_THRESHOLD_SECONDS=120   # optional (defaults to min(120s, ttl*0.2))
export SNOWPIPE_REFRESH_MIN_COOLDOWN_SECONDS=300 # optional (defaults to 300)
export SNOWPIPE_REFRESH_MAX_SKEW_SECONDS=30      # optional (defaults to 30)
export SNOWPIPE_RETRY_MAX_ATTEMPTS=4             # optional (defaults to 4)
export SNOWPIPE_RETRY_INITIAL_DELAY_MS=200       # optional (defaults to 200)
export SNOWPIPE_RETRY_MULTIPLIER=1.8             # optional (defaults to 1.8)
export SNOWPIPE_RETRY_MAX_DELAY_MS=5000          # optional (defaults to 5000)
export SNOWPIPE_RETRY_JITTER=full                # optional (full|decorrelated)
```

Equivalent JSON snippet:
```json
{
  "user": "service_user",
  "account": "your_account",
  "url": "https://xy12345.us-east-1.snowflakecomputing.com",
  "private_key_path": "~/.secrets/snowflake_private_key.pem",
  "refresh_threshold_secs": 120,
  "refresh_min_cooldown_secs": 300,
  "refresh_max_skew_secs": 30,
  "retry_max_attempts": 4,
  "retry_initial_delay_ms": 200,
  "retry_multiplier": 1.8,
  "retry_max_delay_ms": 5000,
  "retry_jitter": "full"
}
```

## 2. Exercise the Guarded Control Flow
Generate a client using `Config::from_file` or `Config::from_env` and invoke the control-plane operations. The guard automatically refreshes the JWT before discovery or scoped-token calls when the TTL window is breached.

```rust
use snowpipe_streaming::{Config, StreamingIngestClient};

#[derive(serde::Serialize, Clone)]
struct Row { id: u64 }

#[tokio::main]
async fn main() -> Result<(), snowpipe_streaming::Error> {
    let cfg = Config::from_env()?;
    let client = StreamingIngestClient::<Row>::new(
        "svc-client", "DB", "SCHEMA", "PIPE", cfg,
    ).await?;
    let mut channel = client.open_channel("demo").await?;
    channel.append_row(&Row { id: 1 }).await?;
    channel.close().await?;
    Ok(())
}
```

When the token is close to expiry you will see structured tracing similar to:
```
refresh.start attempt_id=...
refresh.success attempt_id=...
```
Failures emit `refresh.failure` along with the propagated error.

## 3. Run the Automated Tests
Integration tests rely on `wiremock` to bind ephemeral ports. Run them on a host that allows local sockets:
```bash
cargo test
```
Contract and unit suites verifying the refresh policy and retry logic are included automatically. If your environment forbids binding local ports (e.g., some CI sandboxes), run `cargo test --tests` locally instead.

## 4. Known Follow-ups
Ingest operations currently retain their previous retry orchestration; the control-plane flow is fully guarded. Follow-up work will extend the shared context and telemetry to ingest requests before enabling forced refresh on channel failures.
