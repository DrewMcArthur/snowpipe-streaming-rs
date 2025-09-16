# Tasks: Complete README

**Input**: Current codebase API and tests (`src/`, `tests/`), existing `README.md`
**Prerequisites**: None (documentation-only change), optional: run tests to validate examples

## Format: `[ID] [P?] Description`
- [P]: Can run in parallel (different files)
- Include exact file paths in descriptions

## Phase 3.1: Setup
- [x] T001 Audit `README.md` against current API surface
      - Identify mismatches (e.g., `ConfigLocation` vs `Config`, function names/signatures)
      - Capture a concrete edit list to drive T002–T013

## Phase 3.2: Core README Updates
- [x] T002 Replace all incorrect types/usages in `README.md`
      - Use `snowpipe_streaming::{Config, StreamingIngestClient}`
      - Update constructor calls to pass `Config::from_file(...)` or `Config::from_env()`
      - Ensure example code compiles against current API
- [x] T003 Add "Installation" section to `README.md`
      - Show `Cargo.toml` dependency snippet for `snowpipe-streaming = { path = "." }` or git if applicable
      - Note required Rust version (MSRV) and edition
- [x] T004 Add "Quickstart" end-to-end example in `README.md`
      - Minimal runnable example: create client with `Config::from_file`, `open_channel`, `append_row`, `close`
      - Include sample `config.json`
- [x] T005 Document configuration in `README.md`
      - List JSON fields and corresponding env vars (`SNOWFLAKE_*`), defaults
      - Show both config file and env-based construction (`Config::from_env()`)
- [x] T006 Document authentication modes in `README.md`
      - Pre-supplied JWT (KEYPAIR_JWT)
      - Programmatic OAuth2 token generation with private key (fields: `private_key[_path]`, `private_key_passphrase`, `jwt_exp_secs`)
      - Add security note about protecting private keys and passphrases
- [x] T007 Document batching and size limits in `README.md`
      - `append_rows` usage, 16MB max request size, chunking behavior and tradeoffs
      - Guidance on row sizing and memory considerations
- [x] T008 Document close semantics and timeouts in `README.md`
      - `close()` behavior, warning cadence, `close_with_timeout(Duration)` override
- [x] T009 Add error handling section in `README.md`
      - Reference `Error` variants and common causes (HTTP failures, config/key errors, DataTooLarge)
      - Suggest logging/troubleshooting steps
- [x] T010 Add logging/tracing usage to `README.md`
      - Show enabling `tracing_subscriber` in examples and what logs to expect
- [x] T011 Align/trim `README.md` TODO/Roadmap
      - Remove items already implemented (e.g., `append_rows` exists)
      - Keep or rephrase future work (e.g., `BufferedClient`, builder-style client creation)

## Phase 3.3: Examples and Ancillary Files
- [x] T012 Fix or replace `examples/example.rs` with a compiling example matching README
      - Ensure it uses current API and compiles behind `unstable-example` feature
      - Cross-reference from README "Examples" section
- [x] T013 Add an "Examples" section in `README.md`
      - Point to `tests/integration.rs` flows and `examples/example.rs`
- [ ] T014 [P] Add a `LICENSE` file and update `Cargo.toml` with `license` field (if decided)
      - Status: Deferred per maintainer guidance (no license yet)

## Phase 3.4: Polish
- [x] T015 Add "Compatibility" section in `README.md`
      - State supported Rust toolchain and platforms; note `reqwest` default TLS
- [x] T016 Add "Contributing" and "Security" notes in `README.md`
      - Basic guidelines, how to report security issues, handling secrets
- [ ] T017 Optional: Add badges (CI/test), and a short "Changelog" note linking to releases or `CHANGELOG.md` (if added)
      - Status: Optional, not included in this pass

## Dependencies
- T001 precedes all README edits (T002–T011)
- T012 can proceed in parallel with README edits; T013 references T012
- T014 independent (parallel) but README License section depends on it
- Polish tasks (T015–T017) after core sections exist

## Validation Checklist
- [ ] README examples compile and match current API
- [ ] Configuration and auth sections reflect `src/config.rs` fields and envs
- [ ] Batching/limits align with `src/channel.rs` (`MAX_REQUEST_SIZE` = 16MB)
- [ ] Close semantics match implementation and tests
- [ ] Examples and tests referenced are accurate
- [ ] License section consistent with repository files
