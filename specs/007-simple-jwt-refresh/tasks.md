# Tasks: Simple JWT Refresh Logic

**Input**: Design documents from `/specs/007-simple-jwt-refresh/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/

## Phase 3.1: Setup
- [ ] T001 Update runtime dependencies for randomized back-off by adding `rand` (and required features) to `[dependencies]` in `Cargo.toml`; keep dev-dependency usage intact.

## Phase 3.2: Tests First (TDD)
- [ ] T002 Add a unit test in `src/client/crypto/tests.rs` asserting that configs with `jwt_exp_secs` below 30 clamp to 30 seconds and emit a single warning.
- [ ] T003 Add a unit test in `src/client/crypto/tests.rs` verifying that a JWT refresh is triggered when remaining TTL drops below the safety margin (token value changes, timestamps advance).
- [ ] T004 [P] Extend `tests/unit/jwt.rs` with a test that building a `StreamingIngestClient` using `Config::from_values` plus `jwt_token` logs a deprecation warning.
- [ ] T005 [P] Create `tests/retry_401_success.rs` covering the 401→200 scenario: mock Snowflake to return 401 then 200, assert warning log, single retry, and success response.
- [ ] T006 [P] Create `tests/retry_401_failure.rs` verifying that two consecutive 401 responses lead to exactly one retry and an `Error::Auth` result.
- [ ] T007 [P] Create `tests/retry_429_backoff.rs` validating the 1–3 s randomized delay before retrying a 429 response (use deterministic time control to keep the test fast).

## Phase 3.3: Core Implementation (only after tests fail)
- [ ] T008 Implement configuration clamping logic in `src/config.rs`, returning adjusted `jwt_exp_secs`, enforcing minimum/maximum bounds, and emitting a `tracing::warn!` that includes supplied vs. effective values.
- [ ] T009 Introduce a `JwtContext` manager in `src/client/crypto.rs` (plus field additions in `src/client/mod.rs`) to cache the current token, track expiry, and expose `ensure_valid_jwt`/`needs_refresh` helpers using the configured safety margin.
- [ ] T010 Refactor `StreamingIngestClient::new` in `src/client/impls.rs` to initialize and store the new JWT context, to prefer private-key-based generation, and to log a deprecation warning when `jwt_token` is supplied.
- [ ] T011 Add a reusable `send_with_jwt` helper in `src/client/impls.rs` that acquires a fresh JWT when needed, attaches it to control-plane requests, and retries exactly once after a 401 using the refreshed token.
- [ ] T012 Extend the same helper in `src/client/impls.rs` to treat 429 responses with a randomized 1–3 s `tokio::time::sleep` back-off before retrying (propagate errors after the retry).

## Phase 3.4: Integration
- [ ] T013 Update control-plane methods (`discover_ingest_host`, `get_scoped_token`) and ingest operations in `src/client/impls.rs` to route through `send_with_jwt`, ensuring JWT refresh/back-off behavior is used consistently.
- [ ] T014 Ensure clamp/refresh state is reflected in logs by wiring `was_clamped`/margin metadata into `tracing` events inside `src/client/crypto.rs` and `src/client/impls.rs`.

## Phase 3.5: Polish
- [ ] T015 [P] Refresh `README.md` (and any referenced examples) to document automatic JWT refresh, 401 retry, 429 back-off, and deprecation of user-supplied JWTs.
- [ ] T016 [P] Update `examples/example.rs` (or add a focused usage snippet) to demonstrate long-running usage with refresh margin configuration and documented log outputs.
- [ ] T017 [P] Run `cargo fmt`, `cargo clippy --all-targets`, and the full test suite (`cargo test`) to verify the feature end-to-end.

## Dependencies
- T002 → T008 (clamp behavior must be implemented after the failing test exists).
- T003 → T009 (refresh threshold test drives context implementation).
- T004 → T010 (deprecation warning test forces constructor changes).
- T005, T006 → T011 (401 retry logic satisfies both scenarios).
- T007 → T012 (back-off handling must satisfy 429 test).
- T011 → T013 (helper must exist before routing all requests through it).
- T008, T009 → T014 (logging can reference clamp/context state only after they exist).
- All implementation tasks (T008–T014) → T015–T017 (docs and verification come last).

## Parallel Execution Example
```
# After setup completes, kick off independent test authoring tasks in parallel:
Task: "T004 Extend tests/unit/jwt.rs with deprecation warning coverage"
Task: "T005 Create tests/retry_401_success.rs for 401 recovery"
Task: "T006 Create tests/retry_401_failure.rs for repeated 401 handling"
Task: "T007 Create tests/retry_429_backoff.rs validating randomized delay"

# Final polish commands can also run together on fresh builds:
Task: "T015 Refresh README.md with auto-refresh guidance"
Task: "T016 Update examples/example.rs with refresh configuration"
Task: "T017 Run cargo fmt, cargo clippy --all-targets, and cargo test"
```

## Validation Checklist
- [ ] All contract scenarios have corresponding tests (T002–T007).
- [ ] Each entity from the data model maps to implementation work (JwtContext → T009, RetryPolicy → T011/T012).
- [ ] Tests precede implementation in task order.
- [ ] [P] tasks touch distinct files or are tooling-only steps.
- [ ] Every task specifies concrete file paths or commands.
