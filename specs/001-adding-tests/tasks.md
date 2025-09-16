# Tasks: Add Tests and Mocked HTTP Server

**Input**: Design documents from `/specs/001-adding-tests/`
**Prerequisites**: plan.md (required), research.md, contracts/

## Execution Flow (main)
```
1. Load plan.md from feature directory
   → If not found: ERROR "No implementation plan found"
   → Extract: tech stack, libraries, structure
2. Load optional design documents:
   → contracts/: Each file → contract test task
   → research.md: Extract decisions → setup tasks
3. Generate tasks by category:
   → Setup, Tests (contract/integration/unit), Core fixes, Polish
4. Apply task rules:
   → Different files = mark [P] for parallel
   → Same file = sequential (no [P])
   → Tests before implementation (TDD)
5. Number tasks sequentially (T001, T002...)
6. Define dependencies and parallel examples
```

## Format: `[ID] [P?] Description`
- **[P]**: Can run in parallel (different files, no dependencies)
- Include exact file paths in descriptions

## Phase 3.1: Setup
- [x] T001 Add dev-dependencies in `Cargo.toml`: `wiremock` (mock HTTP); optional `tracing-subscriber` for test logs
- [x] T002 [P] Create test fixtures under `tests/fixtures/` from contracts:
      - `open_channel_response.json`, `append_rows_response.json`, `channel_status_response.json`

## Phase 3.2: Tests First (TDD) ⚠️ MUST COMPLETE BEFORE 3.3
- [x] T003 [P] Integration: discovery and token flow in `tests/integration.rs`
      - Start `wiremock::MockServer`
      - Stub `GET /v2/streaming/hostname` → body=`<server.uri()>`
      - Stub `POST /oauth/token` → body=`scoped-token`
      - Set config via file to avoid env races in tests
      - Assert `StreamingIngestClient::new(..., ConfigLocation::Env)` sets `ingest_host` and `scoped_token`
- [x] T004 [P] Integration: open→append(1)→status→close in `tests/integration.rs`
      - Stub `PUT /v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch`
        → JSON from `tests/fixtures/open_channel_response.json`
      - Stub `POST /v2/streaming/data/databases/db/schemas/schema/pipes/pipe/channels/ch/rows?...`
        → JSON from `tests/fixtures/append_rows_response.json`
      - Stub `POST /v2/streaming/databases/db/schemas/schema/pipes/pipe:bulk-channel-status`
        → JSON from `tests/fixtures/channel_status_response.json`
      - Stub `DELETE /v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch` → 200
      - Assert close completes without error
- [x] T005 [P] Integration: batched `append_rows` across size threshold in `tests/integration.rs`
      - Generate rows large enough to require multiple POSTs; assert number of calls via `wiremock` expectations
- [x] T006 [P] Integration: HTTP error mapping in `tests/integration.rs`
      - Stub rows POST to return 413; assert `Error::Http` (via `reqwest::Error`) bubbles
- [x] T007 Unit: MAX_REQUEST_SIZE enforcement in `src/channel.rs`
      - Construct payload >16MB and assert `Error::DataTooLarge(_, 16777216)`
- [x] T008 Unit: chunk size never zero in `src/channel.rs`
      - Use a very large serialized row so `(2*row_size+1) > MAX_REQUEST_SIZE` → computed chunk size 0 → ensure code will be fixed; test should initially fail due to panic
- [x] T009 Unit: config from env in `src/config.rs`
      - Success path with all vars set; failure path for each missing var returns `Error::Config(..)`
- [x] T010 Unit: types serde in `src/types.rs`
      - Parse fixtures into `OpenChannelResponse`, `AppendRowsResponse`, `ChannelStatus`

## Phase 3.3: Core Implementation (ONLY after tests are failing)
- [x] T011 Ingest URL handling in `src/channel.rs`
      - Build base URL using `self.client.ingest_host`: if contains `"://"` use as-is; else prefix `https://`
      - Apply to open, append, status, and delete endpoints
- [x] T012 Control URL handling (verify) in `src/client.rs`
      - Already supports scheme; add comment/tests if needed (no behavior change)
- [x] T013 Fix chunk size calculation in `src/channel.rs`
      - Ensure `let chunk_size = max(1, MAX_REQUEST_SIZE / (2*row_size + 1))`
      - Handle `row_size == 0` gracefully
- [x] T014 Minor: tighten error contexts and keep `tracing` messages consistent for tests

- [x] T017 Add close warnings and timeout in `src/channel.rs` (1m warn, 5m default; `close_with_timeout` with argument)
- [x] T018 Adjust integration test status mock to return commit token >= last_pushed so `close()` completes

## Phase 3.4: Polish
- [x] T015 [P] Add `tests` logging init using `tracing-subscriber` (optional) in `tests/integration.rs`
- [x] T016 [P] Update `README.md` with a short “Testing” section describing mock server usage and `close_with_timeout`

## New Tests
- [x] T019 Add explicit chunk size guard test: two rows each > 8MB serialized should produce exactly two POSTs to rows endpoint

## CI & Quality
- [x] T020 Update GitHub Actions to run check, clippy, fmt, and tests on push to main
- [x] T021 Fix clippy lints to `-D warnings`
- [x] T022 Ensure `cargo fmt --check` passes

## Dependencies
- T003–T010 (tests) must be written and failing before T011–T014
- T011 unblocks T003–T006 to pass
- T013 unblocks T008
- Documentation (T016) after tests stabilize

## Parallel Example
```
# After setup, write tests in parallel:
T003 Integration: discovery+token in tests/integration.rs
T007 Unit: MAX_REQUEST_SIZE in src/channel.rs
T009 Unit: config env in src/config.rs
T010 Unit: types serde in src/types.rs
```

## Validation Checklist
- [ ] All contract endpoints have corresponding mock + test
- [ ] Tests precede implementation changes
- [ ] Parallel tasks avoid same-file edits
- [ ] Each task includes exact file path
- [ ] Integration tests do not call external services
