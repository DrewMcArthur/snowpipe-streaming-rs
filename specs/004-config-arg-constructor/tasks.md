
# Tasks: Config Argument Constructor

**Input**: Design documents from `/specs/004-config-arg-constructor/`

## Execution Flow
```
1) Add public Config API
2) Replace client constructor to take Config
3) Deprecate AWS secrets loading
4) Update tests to use new API
5) Update README usage
6) Clippy/fmt/tests
```

## Setup
- [x] T001 Expose public `Config` in `src/config.rs` with complete fields (user, account, url, jwt_token, private_key, private_key_path, private_key_passphrase, jwt_exp_secs)
- [x] T002 Implement `Config::from_values(...)` (preferred), `Config::from_file(path)`, `Config::from_env()`

## Implementation
- [x] T003 Change `StreamingIngestClient::new` to accept `Config` directly and remove internal I/O
- [x] T004 Add `#[deprecated]` on AWS secret loader; route callers to `Config::*`
- [x] T005 Remove/feature-gate AWS-related code paths from tests (no usage)

## Tests First (TDD)
- [x] T006 Unit: `Config::from_values` constructs minimal and full variants (defaults/optionals)
- [x] T007 Unit: `Config::from_file` success/failure (missing fields/file)
- [x] T008 Unit: `Config::from_env` success/failure; ensure actionable errors
- [x] T009 Integration: Construct client via `Config::from_values` and validate OAuth2 + discovery + scoped token (wiremock)
- [x] T010 Integration: Construct client via `Config::from_file` for same flow

## Polish
- [x] T011 Update README with examples for all three Config constructors; note deprecation of AWS secret loading
- [x] T012 Run `cargo clippy --all-targets -- -D warnings` and fix lints
- [x] T013 Run `cargo fmt --all -- --check` and address formatting
- [x] T014 Ensure integration tests remain fast (pre-generated assertion), unit tests retain decrypt/sign coverage

## Validation Checklist
- [x] New constructor uses Config directly (no file/env I/O inside client)
- [x] `Config::from_values|from_file|from_env` behave correctly with clear errors
- [x] OAuth2/encrypted PEM paths unchanged via new API
- [x] README documents new usage; AWS secret loading deprecated
- [x] Clippy/fmt/tests green
