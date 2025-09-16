Quickstart: Running Tests with Mocked HTTP Server

Prerequisites
- Rust toolchain installed (`cargo`, `rustc`)

Steps
- Run all tests: `cargo test`
- Integration tests will start a local HTTP server automatically; no real Snowflake credentials are required.

Notes
- If tests introduce a base URL decision (absolute vs host), production remains unchanged (HTTPS default) while tests pass `http://127.0.0.1:<port>` via discovery.
