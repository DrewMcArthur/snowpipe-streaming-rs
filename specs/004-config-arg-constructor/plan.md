# Implementation Plan: Config Argument Constructor

**Branch**: `[004-config-arg-constructor]` | **Date**: 2025-09-16 | **Spec**: specs/004-config-arg-constructor/spec.md

## Summary
Replace the `ConfigLocation`-driven constructor with a direct, explicit configuration API. Introduce a public `Config` type that contains all necessary fields and provides three ergonomic constructors:
- `Config::from_values(...)` — build from explicit values (preferred in apps/services)
- `Config::from_file(path)` — load JSON from disk (explicit, testable)
- `Config::from_env()` — load from environment (explicit opt-in)

Backwards compatibility is not required. Deprecate AWS secret loading and any non-essential I/O in library internals.

## Technical Context
- Language: Rust 2024
- Key modules impacted: `src/config.rs`, `src/client.rs`, `tests/*`, `README.md`
- Current features to preserve: programmatic JWT (`/oauth2/token`), encrypted PKCS#8 PEM decryption, timeouts/warnings, URL handling

## Design
- Public `Config` struct (in `src/config.rs`) with complete fields used by the client today:
  - `user`, `account`, `url`
  - auth material: `jwt_token` (optional), `private_key`/`private_key_path` (optional), `private_key_passphrase` (optional), `jwt_exp_secs` (optional)
- Associated constructors:
  - `Config::from_values(user, account, url, jwt_token, private_key, private_key_path, private_key_passphrase, jwt_exp_secs)` — builder-like with sane defaults or implement `ConfigBuilder` for ergonomics
  - `Config::from_file(path: impl AsRef<Path>) -> Result<Config, Error>` — JSON via serde
  - `Config::from_env() -> Result<Config, Error>` — explicit env opt-in
- Streamlined `StreamingIngestClient` constructor:
  - `StreamingIngestClient::new(client_name, db, schema, pipe, config: Config)` — no internal file/env reads
- Deprecations:
  - Mark AWS secret loading (`read_config_from_secret`) as `#[deprecated(note = "Use Config::* constructors; AWS secret loading removed")]`
  - Optionally feature-gate or remove AWS deps in a follow-up breaking change

## API Sketch
```rust
pub struct Config { /* fields */ }
impl Config {
    pub fn from_values(...) -> Self { ... }
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> { ... }
    pub fn from_env() -> Result<Self, Error> { ... }
}

impl<R: Serialize + Clone> StreamingIngestClient<R> {
    pub async fn new(client_name: &str, db: &str, schema: &str, pipe: &str, config: Config) -> Result<Self, Error> { ... }
}
```

## Migration Notes / Best Practices
- Prefer `from_values` in long-lived services where configuration is already parsed/validated
- Use `from_file`/`from_env` only at app boundaries; avoid mixing I/O into library internals
- Keep `Error` actionable (no panics). Maintain informative logs (without secrets)
- Consider a `ConfigBuilder` for readability and defaults (future enhancement)
- Consider removing AWS dependencies entirely in a breaking-change release (or feature-gate)

## Tasks
1) Expose public `Config` type and associated constructors (values/file/env)
2) Replace `ConfigLocation`-based client constructor with `Config` variant
3) Deprecate/disable AWS Secret loading; add deprecation attributes and README note
4) Update all tests to use the new API; remove location-based helpers from tests
5) Update README: authentication + configuration (examples for all three constructors)
6) Clippy/fmt/tests; ensure integration tests remain fast (pre-generated assertion in integration)

## Tests
- Unit tests: `Config::from_values`, `Config::from_file`, `Config::from_env` (success + failure per missing fields)
- Integration tests: reuse existing wiremock scenarios; construct `StreamingIngestClient` via `Config::from_values` and `Config::from_file`
- Ensure encrypted PEM + OAuth2 flow still passes using the new API

## Acceptance
- New constructor is the default path; programmatic JWT/encrypted PEM work unchanged
- No internal I/O in client constructor; all I/O via explicit `Config::*`
- CI green (clippy/fmt/tests);
- README updated with usage and migration guidance
