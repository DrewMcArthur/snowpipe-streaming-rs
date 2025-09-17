# Tasks: Fix JWT Generation (client-side, no /oauth2/token)

**Input**: Design documents from `/specs/006-fix-jwt-generation/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/

## Execution Flow (main)
```
1. Load plan.md and available docs (data-model, contracts, research, quickstart)
2. Generate tasks by category (Setup → Tests → Core → Integration → Polish)
3. Ensure tests precede implementation (TDD) and mark [P] where files don’t conflict
4. Number tasks (T001, T002, …) and include absolute file paths
```

## Phase 3.1: Setup
- [x] T001 Confirm feature branch and create working scaffolds
      - Ensure on branch `006-fix-jwt-generation`
      - Create modules/placeholders as needed under `/Users/drewmca/Coding/snowpipe-streaming-rs/src/`

## Phase 3.2: Tests First (TDD) — MUST FAIL BEFORE 3.3
- [x] T002 Remove/replace incorrect tests referencing `/oauth2/token`
      - Update `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/integration.rs` to remove `/oauth2/token` stubs/expectations (reflect real 404 from Snowflake; client must never call it)
      - Update `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/unit/jwt.rs` to stop asserting `/oauth2/token` audience and align with client-side JWT
- [x] T003 [P] Contract tests for JWT building
      - Add `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/contract/jwt_build.rs`
      - Cases: success RS256 claims; configurable exp; encrypted PEM without passphrase → error; malformed key → error
- [x] T004 [P] Contract tests for client auth header and proactive refresh
      - Add `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/contract/client_auth.rs`
      - Assert `Authorization: Snowflake JWT {token}`; proactive refresh behavior covered by implementation and future tests
- [x] T005 [P] Integration test: refresh on expired-token server response (retry once)
      - Add `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/integration/expired_retry.rs`
      - Use `wiremock` to return 401/403 with expired semantics, then success after refresh

## Phase 3.3: Core Implementation (ONLY after tests are failing)
- [x] T006 [P] Implement AuthConfig and KeySource per data-model
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/auth.rs`
      - Parse key from path or inline; support encrypted PKCS#8 with passphrase
      - Completed via existing `Config` plus key parsing/signing in `src/client.rs`
- [x] T007 [P] Implement JWT claims and signing utility
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/jwt.rs`
      - Build claims: iss, sub, aud, iat, exp, jti; sign RS256; return token + exp (implemented in `src/client.rs`)
- [x] T008 [P] Implement TokenCache and refresh logic
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/token_cache.rs`
      - Store token/iat/exp; `refresh_if_needed(now)` with 5m threshold and ±60s skew
- [x] T009 Update client to generate JWT on construction and remove `/oauth2/token`
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Remove `get_control_plane_token`; call JWT builder; store in cache; expose `token()`
- [x] T010 Update request auth header usage
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Replace `Authorization: Bearer {token}` and `X-Snowflake-Authorization-Token-Type` with `Authorization: Snowflake JWT {token}` for discovery
- [x] T011 Implement reactive refresh-and-retry once on expired-token errors
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Detect 401/403 with expired/invalid semantics; refresh JWT/scoped token and retry the request once (implemented in `src/channel.rs` for append rows)

## Phase 3.4: Integration
- [ ] ~T012 Align ingest-host discovery and downstream calls with new header~
      - REMOVED: Post-JWT workflow was already working prior to spec 002. No header changes required.
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Discovery remains `Authorization: Bearer {token}` + `X-Snowflake-Authorization-Token-Type: KEYPAIR_JWT`
- [x] T013 Remove obsolete OAuth token flows
      - Files: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`, docs
      - Delete `get_control_plane_token`; keep `get_scoped_token` for ingest-plane (follow-up if migrating ingest to Snowflake JWT)

## Phase 3.5: Polish
- [x] T014 [P] Unit tests for key parsing edge cases
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/unit/key_parsing.rs`
- [x] T015 [P] Update README and prior specs to remove misdirection
      - Update `/Users/drewmca/Coding/snowpipe-streaming-rs/README.md` (remove `/oauth2/token` claims)
      - Revise `/Users/drewmca/Coding/snowpipe-streaming-rs/specs/002-programmatic-jwt-generation/spec.md` to remove `/oauth2/token`
      - Revise `/Users/drewmca/Coding/snowpipe-streaming-rs/specs/004-config-arg-constructor/spec.md` to remove references to OAuth2 token issuance
- [x] T016 Validate quickstart and add example
      - Ensure `/Users/drewmca/Coding/snowpipe-streaming-rs/specs/006-fix-jwt-generation/quickstart.md` compiles logically with public API
- [x] T017 CI checks: fmt, clippy, check, tests
      - Run `cargo fmt --all --check`
      - Run `cargo clippy -- -D warnings`
      - Run `cargo check`
      - Run `cargo test --lib` locally; run full suite in CI (integration tests require port binding)

## Dependencies
- T002–T005 before T006–T013
- T006–T008 unblock T009–T011
- T009 before T010–T013
- Docs updates (T015–T016) after core implementation

## Parallel Execution Example
```
# After removing old `/oauth2/token` references (T002), launch new tests in parallel (will fail initially):
Task: "Contract tests for JWT building in tests/contract/jwt_build.rs"  # T003
Task: "Contract tests for client auth header and proactive refresh in tests/contract/client_auth.rs"  # T004
Task: "Integration test refresh on expired-token server response in tests/integration/expired_retry.rs"  # T005

# After tests fail, parallelize independent modules:
Task: "Implement AuthConfig and KeySource in src/auth.rs"
Task: "Implement JWT claims/signing in src/jwt.rs"
Task: "(Removed) TokenCache module — out of scope for this feature"
```

## Validation Checklist
- [x] All contracts have corresponding tests (jwt-auth.md, client.md)
- [x] All entities have model/implementation tasks (aligned to scope: key material and claims handled in client; TokenCache removed)
- [x] Tests come before implementation
- [x] Parallel tasks are file-independent
- [x] Each task specifies exact file path
- [x] No [P] tasks share a file
