 # Implementation Plan: Add Tests and Mocked HTTP Server
 
 **Branch**: `[001-adding-tests]` | **Date**: 2025-09-16 | **Spec**: /Users/drewmca/Coding/snowpipe-streaming-rs/specs/001-adding-tests/spec.md
 **Input**: Feature specification from `/specs/001-adding-tests/spec.md`
 
 ## Execution Flow (/plan command scope)
 ```
 1. Load feature spec from Input path
    → If not found: ERROR "No feature spec at {path}"
 2. Fill Technical Context (scan for NEEDS CLARIFICATION)
    → Detect Project Type from context (single Rust library)
    → Set Structure Decision based on project type
 3. Evaluate Constitution Check section below
    → If violations exist: Document in Complexity Tracking
    → If no justification possible: ERROR "Simplify approach first"
    → Update Progress Tracking: Initial Constitution Check
 4. Execute Phase 0 → research.md
    → Resolve: test framework + HTTP mocking approach; HTTPS constraint in URLs
 5. Execute Phase 1 → contracts, quickstart.md
 6. Re-evaluate Constitution Check
    → If new violations: Refactor design, return to Phase 1
    → Update Progress Tracking: Post-Design Constitution Check
 7. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
 8. STOP - Ready for /tasks command
 ```
 
 ## Summary
 Add comprehensive unit and integration tests for the existing Rust library to prevent regressions. Integration tests will run against a local mocked HTTP server that emulates Snowflake Streaming Ingest endpoints used by the client/channel, enabling deterministic, credential‑free CI runs.
 
 ## Technical Context
 **Language/Version**: Rust (edition 2024)
 **Primary Dependencies**: `reqwest`, `tokio`, `serde`, `serde_json`, `tracing`, `aws-config`, `aws-sdk-secretsmanager`, `jiff`
 **Storage**: N/A
 **Testing**: `cargo test` with `tokio::test`; add dev-deps: `wiremock` (mock HTTP), optionally `serde_json` for fixtures
 **Target Platform**: macOS/Linux CI and developer machines
 **Project Type**: single
 **Performance Goals**: Respect 16MB request limit in chunking; keep tests fast (<2s)
 **Constraints**: No real network credentials; tests must not hit Snowflake; deterministic responses
 **Scale/Scope**: Library tests (unit + integration) covering core flows
 
 ## Constitution Check
 **Simplicity**:
 - Projects: 2 (library, tests)
 - Using framework directly: Yes (no wrappers around `reqwest` beyond client)
 - Single data model: Yes (serde types already defined)
 - Avoiding patterns: Yes (no extra abstraction for tests)
 
 **Architecture**:
 - Library only: Yes (no app code)
 - Libraries listed: `snowpipe-streaming` (client SDK)
 - CLI: N/A
 - Library docs: Quickstart for tests provided
 
 **Testing (NON-NEGOTIABLE)**:
 - RED-GREEN-Refactor: Enforced via tasks ordering
 - Order: Contract → Integration → Unit where applicable
 - Real deps: HTTP mocked locally; no external services
 - Integration tests for client/channel flows: Yes
 
 **Observability**:
 - Uses `tracing`; validate helpful logs in tests
 
 **Versioning**:
 - Internal only; no API breakage planned
 
 ## Project Structure
 
 ### Documentation (this feature)
 ```
 specs/001-adding-tests/
 ├── plan.md              # This file
 ├── research.md          # Phase 0 output
 ├── quickstart.md        # Phase 1 output
 └── contracts/           # Phase 1 output (HTTP contract samples)
 ```
 
 ### Source Code (repository root)
 ```
 src/
 tests/
 ├── integration.rs       # Integration tests using mocked server
 └── unit/                # Unit tests per module (or inline #[cfg(test)])
 ```
 
 **Structure Decision**: Option 1 (single project) with `tests/` directory for integration tests; unit tests inline in modules where appropriate.
 
 ## Phase 0: Outline & Research
 Unknowns and decisions to resolve:
 - HTTPS in URLs: code hardcodes `https://` for ingest endpoints; local mock server will be HTTP. Decision: allow absolute base URL returned by hostname discovery (e.g., `http://127.0.0.1:PORT`) and use it directly in requests. Minimal change: treat discovered value as full base URL if it contains `://`; otherwise default to `https://{host}`.
 - Mocking library: choose `wiremock` for ergonomic async mocking with `reqwest`.
 - Test coverage focus: config reading, error conversions, chunking logic, data size limit, end‑to‑end channel open/append/status/close.
 
 Output: see research.md.
 
 ## Phase 1: Design & Contracts
 Design highlights:
 - Add dev-dependency `wiremock` and create a helper to start a mock server exposing endpoints matching current client behavior.
 - Inject control URL via config file/env for tests; discovery returns ingest base URL with scheme to avoid TLS in tests.
 - Keep production paths unchanged (default still uses HTTPS when hostname lacks scheme).
 
 Contracts (HTTP behavior to emulate):
 - `GET {control}/v2/streaming/hostname` → body: base URL string (e.g., `https://ingest.example.com` or `http://127.0.0.1:PORT`).
 - `POST {control}/oauth/token` → 200 text body: opaque token string.
 - `PUT {ingest}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel}` → JSON `OpenChannelResponse`.
 - `POST {ingest}/v2/streaming/data/.../rows?continuationToken={}&offsetToken={}` → JSON `AppendRowsResponse`.
 - `POST {ingest}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}:bulk-channel-status` → JSON map `{ channel_statuses: { <channel>: ChannelStatus } }`.
 - `DELETE {ingest}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel}` → 200.
 
 Quickstart: see quickstart.md.
 
 ## Phase 2: Task Planning Approach
 Task Generation Strategy:
 - From contracts above, generate integration test tasks covering: discovery, token fetch, open, append (single/batched), status polling, close, and error cases (4xx/5xx).
 - Unit tests for: config from env/file; error mapping; chunking boundaries (including zero/huge row sizes); MAX_REQUEST_SIZE enforcement; parsing of tokens.
 
 Ordering Strategy:
 - Write failing contract/integration tests first against mock server.
 - Add the minimal URL‑handling change to pass HTTP base in tests.
 - Add unit tests and fixes (e.g., ensure chunk size ≥ 1).
 
 Estimated Output: ~15–20 tasks in tasks.md (created by /tasks).
 
 ## Phase 3+: Future Implementation
 Executed by subsequent commands; out of scope for /plan.
 
 ## Complexity Tracking
 | Violation | Why Needed | Simpler Alternative Rejected Because |
 |-----------|------------|-------------------------------------|
 | Allowing absolute ingest base URL | Enables HTTP mock server in tests | Running local HTTPS with cert trust adds significant complexity |
 
 ## Progress Tracking
 **Phase Status**:
 - [x] Phase 0: Research complete (/plan command)
 - [x] Phase 1: Design complete (/plan command)
 - [ ] Phase 2: Task planning complete (/plan command - describe approach only)
 - [ ] Phase 3: Tasks generated (/tasks command)
 - [ ] Phase 4: Implementation complete
 - [ ] Phase 5: Validation passed
 
 **Gate Status**:
 - [x] Initial Constitution Check: PASS
 - [x] Post-Design Constitution Check: PASS
 - [x] All NEEDS CLARIFICATION resolved
 - [x] Complexity deviations documented
 
 ---
 *Based on Constitution v2.1.1 - See `/memory/constitution.md`*
