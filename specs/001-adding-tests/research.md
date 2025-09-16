## Research: Testing Strategy for snowpipe-streaming

Decisions
- Mocking HTTP: Use `wiremock` crate to stub `reqwest` calls with async support.
- Base URL handling: Allow discovery to return absolute base URL (with scheme). If `contains("://")`, use as-is; else prefix `https://`. Keeps prod default secure while enabling HTTP in tests.
- Test layout: Integration tests under `tests/` spinning a mock server; unit tests inline with `#[cfg(test)]` in modules.

Rationale
- `wiremock` integrates well with `reqwest` and async `tokio` runtime, avoiding custom hyper/axum servers and TLS complexity.
- Minimal surface-area change (URL decision) isolates test-only behavior without conditional compilation flags.

Alternatives Considered
- `httpmock`: similar capabilities; `wiremock` has broader examples and matcher ergonomics.
- Local HTTPS with self-signed cert: increases complexity (cert generation, TLS config, trust store) for little value in library tests.

Open Questions (resolved)
- Token endpoints return plain text today; tests will mirror this for simplicity.
- Channel status structure is nested; provide sample fixtures to reduce duplication.
