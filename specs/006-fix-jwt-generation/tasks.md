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
- [ ] T001 Confirm feature branch and create working scaffolds
      - Ensure on branch `006-fix-jwt-generation`
      - Create modules/placeholders as needed under `/Users/drewmca/Coding/snowpipe-streaming-rs/src/`

## Phase 3.2: Tests First (TDD) — MUST FAIL BEFORE 3.3
- [ ] T002 Remove/replace incorrect tests referencing `/oauth2/token`
      - Update `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/integration.rs` to remove `/oauth2/token` stubs/expectations (reflect real 404 from Snowflake; client must never call it)
      - Update `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/unit/jwt.rs` to stop asserting `/oauth2/token` audience and align with client-side JWT
- [ ] T003 [P] Contract tests for JWT building
      - Add `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/contract/jwt_build.rs`
      - Cases: success RS256 claims; exp=iAT+<=3600; encrypted PEM without passphrase → error; malformed key → error
- [ ] T004 [P] Contract tests for client auth header and proactive refresh
      - Add `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/contract/client_auth.rs`
      - Assert `Authorization: Snowflake JWT {token}` and refresh when within 5m of exp
- [ ] T005 [P] Integration test: refresh on expired-token server response (retry once)
      - Add `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/integration/expired_retry.rs`
      - Use `wiremock` to return 401/403 with expired semantics, then success after refresh

## Phase 3.3: Core Implementation (ONLY after tests are failing)
- [ ] T006 [P] Implement AuthConfig and KeySource per data-model
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/auth.rs`
      - Parse key from path or inline; support encrypted PKCS#8 with passphrase
- [ ] T007 [P] Implement JWT claims and signing utility
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/jwt.rs`
      - Build claims: iss, sub, aud, iat, exp, jti; sign RS256; return token + exp
- [ ] T008 [P] Implement TokenCache and refresh logic
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/token_cache.rs`
      - Store token/iat/exp; `refresh_if_needed(now)` with 5m threshold and ±60s skew
- [ ] T009 Update client to generate JWT on construction and remove `/oauth2/token`
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Remove `get_control_plane_token`; call JWT builder; store in cache; expose `token()`
- [ ] T010 Update request auth header usage
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Replace `Authorization: Bearer {token}` and `X-Snowflake-Authorization-Token-Type` with `Authorization: Snowflake JWT {token}`
- [ ] T011 Implement reactive refresh-and-retry once on expired-token errors
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Detect 401/403 with expired/invalid semantics; refresh JWT and retry the request once

## Phase 3.4: Integration
- [ ] T012 Align ingest-host discovery and downstream calls with new header
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`
      - Ensure all control/data-plane calls use `Authorization: Snowflake JWT {token}`
- [ ] T013 Remove obsolete OAuth token flows
      - Files: `/Users/drewmca/Coding/snowpipe-streaming-rs/src/client.rs`, docs
      - Delete `get_control_plane_token`; review `get_scoped_token` and remove if not needed

## Phase 3.5: Polish
- [ ] T014 [P] Unit tests for key parsing edge cases
      - File: `/Users/drewmca/Coding/snowpipe-streaming-rs/tests/unit/key_parsing.rs`
- [ ] T015 [P] Update README and prior specs to remove misdirection
      - Update `/Users/drewmca/Coding/snowpipe-streaming-rs/README.md` (remove `/oauth2/token` claims)
      - Revise `/Users/drewmca/Coding/snowpipe-streaming-rs/specs/002-programmatic-jwt-generation/spec.md` to remove `/oauth2/token`
      - Revise `/Users/drewmca/Coding/snowpipe-streaming-rs/specs/004-config-arg-constructor/spec.md` to remove references to OAuth2 token issuance
- [ ] T016 Validate quickstart and add example
      - Ensure `/Users/drewmca/Coding/snowpipe-streaming-rs/specs/006-fix-jwt-generation/quickstart.md` compiles logically with public API

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
Task: "Implement TokenCache in src/token_cache.rs"
```

## Validation Checklist
- [ ] All contracts have corresponding tests (jwt-auth.md, client.md)
- [ ] All entities have model/implementation tasks (AuthConfig, KeySource, JwtClaims, TokenCache, Client)
- [ ] Tests come before implementation
- [ ] Parallel tasks are file-independent
- [ ] Each task specifies exact file path
- [ ] No [P] tasks share a file
