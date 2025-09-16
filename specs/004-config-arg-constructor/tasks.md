
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
- [ ] T001 Expose public `Config` in `src/config.rs` with complete fields (user, account, url, jwt_token, private_key, private_key_path, private_key_passphrase, jwt_exp_secs)
- [ ] T002 Implement `Config::from_values(...)` (preferred), `Config::from_file(path)`, `Config::from_env()`

## Implementation
- [ ] T003 Change `StreamingIngestClient::new` to accept `Config` directly and remove internal I/O
- [ ] T004 Add `#[deprecated]` on AWS secret loader; route callers to `Config::*`
- [ ] T005 Remove/feature-gate AWS-related code paths from tests (no usage)

## Tests First (TDD)
- [ ] T006 Unit: `Config::from_values` constructs minimal and full variants (defaults/optionals)
- [ ] T007 Unit: `Config::from_file` success/failure (missing fields/file)
- [ ] T008 Unit: `Config::from_env` success/failure; ensure actionable errors
- [ ] T009 Integration: Construct client via `Config::from_values` and validate OAuth2 + discovery + scoped token (wiremock)
- [ ] T010 Integration: Construct client via `Config::from_file` for same flow

## Polish
- [ ] T011 Update README with examples for all three Config constructors; note deprecation of AWS secret loading
- [ ] T012 Run `cargo clippy --all-targets -- -D warnings` and fix lints
- [ ] T013 Run `cargo fmt --all -- --check` and address formatting
- [ ] T014 Ensure integration tests remain fast (pre-generated assertion), unit tests retain decrypt/sign coverage

## Validation Checklist
- [ ] New constructor uses Config directly (no file/env I/O inside client)
- [ ] `Config::from_values|from_file|from_env` behave correctly with clear errors
- [ ] OAuth2/encrypted PEM paths unchanged via new API
- [ ] README documents new usage; AWS secret loading deprecated
- [ ] Clippy/fmt/tests green
