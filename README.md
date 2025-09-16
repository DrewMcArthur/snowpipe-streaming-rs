# Snowpipe Streaming Rust SDK

**WARNING: DEFINITELY NOT PRODUCTION READY**

This library is meant to replicate the python api interface, to allow users to stream data to snowflake via the snowpipe streaming API.  This is a naive implementation, since the Python and Java SDKs are not open source, so I'm guessing based off the REST guide and the external visibility of the Python SDK.

## References
- [Python SDK Guide](https://docs.snowflake.com/en/user-guide/snowpipe-streaming-high-performance-getting-started)
- [Python Example Usage](https://gist.github.com/sfc-gh-chathomas/a7b06bb46907bead737954d53b3a8495#file-example-py/)
- [REST Guide](https://docs.snowflake.com/en/user-guide/snowpipe-streaming-high-performance-rest-tutorial)


## TODO
- check flows and make sure it all works, get example working
- add `append_rows` function, accepting a `Vec<RowType>`
- create a `BufferedClient`, which batches `append_row` requests, to speed up / minimize HTTP throughput
- documentation! doctests!
- mock http server? 
- change how client is created, use a builder or the model that has different structs for different stages so that errors are less possible, like what's described [here](https://blog.systems.ethz.ch/blog/2018/a-hammer-you-can-only-hold-by-the-handle.html)
- hide aws behind feature flag

## Testing
- Run all tests: `cargo test`.
- Integration tests use a local mocked HTTP server (wiremock) to emulate Snowflake endpoints; they do not require network or real credentials.
- Configuration for tests is passed via a JSON file using `ConfigLocation::File` to avoid process‑wide env races.

Example test setup (simplified):
```
use snowpipe_streaming::{ConfigLocation, StreamingIngestClient};

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
    "test-client", "db", "schema", "pipe", ConfigLocation::File(cfg_path.into())
).await?;
```

Close semantics:
- `StreamingIngestChannel::close()` polls until Snowflake reports commits for all appended rows.
- Warnings emit every minute after the first, and by default it times out after 5 minutes with `Error::Timeout`.
- You can override the timeout with `close_with_timeout(std::time::Duration::from_secs(30))`.
