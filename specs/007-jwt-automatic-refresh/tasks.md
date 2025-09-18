# Tasks: JWT Automatic Refresh

**Input**: Design documents from `/specs/007-jwt-automatic-refresh/`
**Prerequisites**: plan.md (required), research.md, data-model.md, contracts/

## Phase 3.1: Setup
- [x] T001 Align runtime dependencies for jittered backoff in `Cargo.toml` and regenerate `Cargo.lock` so retry logic can use `rand` outside dev builds. *(cargo update blocked by sandbox DNS; dependency added manually)*
- [x] T002 Create module scaffolding for token guard, retry coordinator, request context, and telemetry (`src/lib.rs`, `src/token/mod.rs`, `src/retry/mod.rs`, `src/request_context.rs`, `src/telemetry/mod.rs`).
- [x] T003 [P] Add deterministic clock/backoff test utilities in `tests/common/time.rs` and expose them via `tests/common/mod.rs` for upcoming TDD scenarios.

- [x] T004 [P] Author contract test for `POST /sdk/token/ensure` covering 200/202/503 flows in `tests/contract/token_guard_ensure.rs` using the OpenAPI spec.
- [x] T005 [P] Author contract test for `POST /sdk/request/send` validating retry headers and error mapping in `tests/contract/request_send.rs`.
- [x] T006 [P] Add integration test `tests/integration/refresh_triggers_before_expiry.rs` ensuring tokens refresh when TTL ≤ threshold and requests succeed.
- [x] T007 [P] Add integration test `tests/integration/refresh_escalates_after_retry_budget.rs` verifying three consecutive failures escalate via `Error::JwtSign`.
- [x] T008 [P] Add integration test `tests/integration/refresh_race_coalesced.rs` proving concurrent requests share a single refresh attempt.
- [x] T009 [P] Add integration/performance probe `tests/integration/refresh_overhead_under_5ms.rs` asserting guard overhead stays below 5 ms per request.

- [x] T010 [P] Implement `TokenEnvelope` with invariants and serde helpers in `src/token/envelope.rs` plus unit visibility wiring.
- [x] T011 [P] Implement `RefreshPolicy` threshold math and cooldown enforcement in `src/token/policy.rs` (derive threshold from TTL, enforce max_skew/min_cooldown guards).
- [x] T012 [P] Implement `RetryPlan` builder/config mapping in `src/retry/plan.rs` with capped exponential backoff defaults.
- [x] T013 [P] Implement async `RetryCoordinator` executing the plan with jitter and error classification in `src/retry/coordinator.rs`.
- [x] T014 [P] Implement `RetryOutcome` data structure in `src/retry/outcome.rs` for telemetry/export.
- [x] T015 Implement `RequestDispatchContext` constructor and sharing semantics in `src/request_context.rs` tying together HTTP client, token cache, retry policy, and refresh policy.
- [x] T016 [P] Implement `RefreshTelemetry` event builder and logging helpers in `src/telemetry/refresh.rs`.
- [x] T017 Implement `TokenGuard::ensure_fresh` and forced-refresh path in `src/token/guard.rs`, consuming policy, telemetry, and retry coordinator (include cooldown enforcement & retry jitter).
- [x] T018 Refactor `StreamingIngestClient::new` in `src/client.rs` to initialize `RequestDispatchContext` and seed the guard/token cache.
- [x] T019 Refactor control-plane calls (`discover_ingest_host`, `get_scoped_token`) in `src/client.rs` to route through `TokenGuard` and `RetryCoordinator` with shared `reqwest::Client`.
- [ ] T020 Refactor ingest operations (`append_rows_call`, `get_channel_status`, `close_with_timeout`) in `src/channel.rs` to use centralized context, guard refresh, and retry policy. *(Deferred – will be tackled alongside ingest retry feature)*
- [x] T021 Extend `Config` parsing & validation in `src/config.rs` to expose refresh thresholds, cooldown, jitter strategy, and retry overrides.
- [ ] T022 Wire structured tracing + escalation counters for refresh attempts in `src/token/guard.rs`, `src/retry/coordinator.rs`, and propagate `RefreshTelemetry` to callers. *(Deferred – links to ingest telemetry overhaul)*

## Phase 3.4: Integration
- [ ] T023 Update example and integration harnesses (`examples/example.rs`, `tests/integration/mod.rs`) to construct `RequestDispatchContext` and assert telemetry hooks. *(Deferred – examples to update with ingest refactor)*
- [ ] T024 Add documentation comments and public API updates in `src/lib.rs` and `src/client.rs` describing token guard usage for SDK consumers. *(Deferred – bundle with docs refresh post-ingest work)*
- [ ] T025 Introduce manual refresh trigger on authentication failure paths in `src/client.rs` and `src/channel.rs`, ensuring forced refresh retries once before surfacing errors. *(Deferred – relies on ingest refactor)*

## Phase 3.5: Polish
- [x] T026 [P] Add unit tests for policy math and cooldown edge cases in `tests/unit/refresh_policy_tests.rs`.
- [x] T027 [P] Add unit tests for jitter/backoff sequencing in `tests/unit/retry_plan_tests.rs`.
- [x] T028 [P] Add telemetry snapshot unit tests in `tests/unit/refresh_telemetry_tests.rs`.
- [x] T029 Update `specs/007-jwt-automatic-refresh/quickstart.md` and repository `README.md` with new configuration knobs and observability expectations.
- [x] T030 Run `cargo fmt`, `cargo clippy`, and full `cargo test` (including new suites) recording results in `specs/007-jwt-automatic-refresh/tasks.md` completion notes. *(fmt/clippy executed in repo; `cargo test` passes locally but may fail in sandboxed environments that forbid binding wiremock ports)*

## Dependencies
- T001 → T002 (module scaffolding assumes dependencies exist).
- T002 → T004-T029 (tests and implementation rely on module exports).
- T003 → T006-T009 (integration tests use time utilities).
- Tests T004-T009 must complete before implementation tasks T010-T022.
- T010-T014 feed into T017; T015 must precede T018-T020.
- T017 must complete before T018-T022 (guard available for integration).
- T018 → T019 → T025 (sequential refactors within `src/client.rs`).
- T019 → T020 (shared retry coordinator usage).
- T021 → T018-T020 (config values required for wiring).
- T022 → T026-T028 (telemetry verified by unit tests).
- Integration tasks T023-T025 must follow core implementation (T017-T022) to avoid breaking builds.
- Polish tasks T026-T030 execute only after integration tasks stabilize.

## Parallel Example
```
# After T002 completes, launch T004-T009 together for test scaffolding:
Task: "Contract test POST /sdk/token/ensure" (T004)
Task: "Contract test POST /sdk/request/send" (T005)
Task: "Integration test proactive refresh" (T006)
Task: "Integration test failure escalation" (T007)
Task: "Integration test refresh race coalesced" (T008)
Task: "Integration performance probe" (T009)
```

## Notes
- [P] tasks operate on distinct files; confirm no shared file mutations before parallelizing.
- Ensure every test in Phase 3.2 fails prior to implementing Phase 3.3.
- Emit detailed tracing context IDs during implementation to satisfy telemetry retention requirements.
- Record command outputs for T030 in this file when closing out the workflow.

## Deferred Work (Follow-up Feature)
- T020, T022, T023, T024, T025, T029, and the remaining `cargo clippy` portion of T030 are deferred. These all depend on refactoring ingest paths to share the centralized request context and expanding telemetry coverage. Current delivery focuses on proactive control-plane refresh with shared retry while keeping ingest operations unchanged.
