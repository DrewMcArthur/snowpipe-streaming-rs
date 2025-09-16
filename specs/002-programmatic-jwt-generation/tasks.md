# Tasks: Programmatic JWT Generation

**Input**: Design documents from `/specs/002-programmatic-jwt-generation/`
**Prerequisites**: plan.md (required), research.md, contracts/

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → Extract: flow (/oauth2/token), claims, config additions
2. Load contracts/: derive request/response shapes
3. Generate tasks: Setup → Tests (TDD) → Core → Polish
4. Apply task rules: [P] for parallel (different files), tests before impl
5. Number tasks sequentially (T001, T002...)
6. Define dependencies and parallel examples
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Phase 3.1: Setup
- [x] T001 Add dependencies in `Cargo.toml` (prod): `jsonwebtoken` (RS256), `pem`
- [x] T002 [P] Extend config in `src/config.rs` with fields:
      - `private_key_passphrase: Option<String>`
      - `jwt_exp_secs: Option<u64>` (default 60)
      - Env support: `SNOWFLAKE_PRIVATE_KEY[_PATH]`, `SNOWFLAKE_PRIVATE_KEY_PASSPHRASE`, `SNOWFLAKE_JWT_EXP_SECS`
- [x] T003 [P] Add new error variants in `src/errors.rs`: `Key(String)`, `JwtSign(String)`

## Phase 3.2: Tests First (TDD)
- [x] T004 Integration: oauth2 token flow in `tests/integration.rs`
      - Stub `POST /oauth2/token` → JSON `{access_token, token_type, expires_in}`
      - Config: no `jwt_token`; provide `private_key` (PEM string)
      - Assert client initializes (discovers host, obtains scoped token) using generated control-plane token
- [x] T005 [P] Unit: key parsing + client assertion claims in `tests/unit/jwt.rs`
      - Load test PEM; generate client assertion; decode (without verify) and assert header `alg=RS256`, `aud` matches `/oauth2/token`, `exp > iat`
- [x] T006 [P] Unit: config reading of new fields in `src/config.rs` (env + file)
- [x] T007 [P] Integration: failure cases in `tests/integration.rs`
      - Missing key → actionable error
      - `/oauth2/token` returns 400 → `Error::Http`

## Phase 3.3: Core Implementation
- [x] T008 Implement key parsing + JWT builder in `src/client.rs` (or `src/auth.rs`)
      - Build client assertion with claims (`iss`/`sub`, `aud`, `iat`, `exp`, `jti`)
      - Sign with RS256 using `private_key` (or file)
- [x] T009 Add `get_control_plane_token` in `src/client.rs`
      - POST `{control}/oauth2/token` (x-www-form-urlencoded)
      - Body: `grant_type`, `client_assertion_type`, `client_assertion`
      - Set `self.jwt_token = access_token`
- [x] T010 Wire into `StreamingIngestClient::new` in `src/client.rs`
      - If `config.jwt_token` missing, generate control-plane token using T008+T009
      - Continue existing `discover_ingest_host` and `get_scoped_token`
- [x] T011 [P] Improve logs and errors along the new path (trace token lengths, not values)

## Phase 3.4: Polish
- [x] T012 [P] Update `README.md` usage with programmatic JWT example (private key path) and config fields
- [x] T013 Run `cargo clippy --all-targets -- -D warnings` and fix any lints
- [x] T014 Run `cargo fmt --all -- --check` and address formatting

## Dependencies
- T004–T007 (tests) before T008–T011 (implementation)
- T008 before T009; T009 before T010
- Docs/lint/format (T012–T014) last

## Parallel Example
```
# After setup, write tests in parallel:
T004 Integration: oauth2 token flow
T005 Unit: jwt claims
T006 Unit: config extensions
T007 Integration: error cases
```

## Validation Checklist
- [ ] Integration test initializes client via `/oauth2/token` without pre-supplied `jwt_token`
- [ ] Unit tests cover parsing/signing/claims and config paths
- [ ] Errors are actionable for missing/invalid key or HTTP failures
- [ ] Clippy and fmt clean
- [ ] README documents new usage clearly
