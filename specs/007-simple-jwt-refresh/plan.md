# Implementation Plan: Simple JWT Refresh Logic

**Branch**: `[007-simple-jwt-refresh]` | **Date**: 2025-09-18 | **Spec**: `/Users/drewmca/Coding/snowpipe-streaming-rs/specs/007-simple-jwt-refresh/spec.md`
**Input**: Feature specification from `/specs/007-simple-jwt-refresh/spec.md`

## Summary
Ensure the Snowpipe streaming client keeps a valid JWT throughout long-running sessions by refreshing tokens whenever the remaining lifetime falls below the safety margin, transparently handling 401 and 429 responses, and discouraging consumer-supplied JWTs in favor of library-managed signing.

## Technical Context
**Language/Version**: Rust 1.79+ (edition 2024)  
**Primary Dependencies**: `jsonwebtoken`, `reqwest`, `tokio`, `tracing`  
**Storage**: In-memory only; credentials supplied via configuration are not persisted.  
**Testing**: `cargo test` with `wiremock` for HTTP behavior simulation  
**Target Platform**: Single-instance Rust ingestion clients interacting with Snowflake Snowpipe Streaming API  
**Project Type**: single (library + client)  
**Performance Goals**: Handle sequential request workloads with occasional retries while keeping refresh latency negligible (<10 ms signing cost).  
**Constraints**: Auto-clamp JWT expirations into `[30, 3600]` seconds; refresh when remaining lifetime ≤ configured margin (default 30s); retry 401 once with warning; log a warning and sleep 2s on 429 before retry.  
**Scale/Scope**: One concurrent instance per credential; no shared credential coordination required.

## Constitution Check
- Constitution document is presently a placeholder with no enforceable mandates.  
- Proceed under documented repository workflows while flagging the missing constitution content for maintainers.

## Project Structure

### Documentation (this feature)
```
specs/007-simple-jwt-refresh/
├── plan.md              # This file (/plan command output)
├── research.md          # Phase 0 output (/plan command)
├── data-model.md        # Phase 1 output (/plan command)
├── quickstart.md        # Phase 1 output (/plan command)
├── contracts/           # Phase 1 output (/plan command)
└── tasks.md             # Phase 2 output (/tasks command - NOT created by /plan)
```

### Source Code (repository root)
```
src/
├── client/
│   ├── crypto.rs
│   ├── impls.rs
│   ├── mod.rs
│   └── tests.rs
├── lib.rs
└── ...

tests/
├── contract/
├── integration/
└── unit/
```

**Structure Decision**: Option 1 (single project) remains suitable; enhancements stay within existing client modules and supporting tests.

## Phase 0: Outline & Research
1. **Focus Areas**
   - Validate Snowflake documentation for 401 retry expectations and 429 back-off guidance.  
   - Measure JWT signing duration to confirm refresh latency stays below target (<10 ms).  
   - Inventory current logging patterns to ensure new warnings/errors follow house style.  
   - Review configuration surfaces to plan deprecation messaging for user-supplied JWTs.  
   - Define fixed 2s back-off handling for 429 responses and capture logging expectations.

2. **Research Activities**
   - Document authoritative references (Snowflake docs, internal style guides).  
   - Prototype micro-benchmark for RSA signing under expected key sizes.  
   - Capture recommended logging fields (`trace_id`, request id) for warning events.  
   - Draft migration guidance describing how integrators should move away from custom JWT injection.

3. **Output Expectations**
   - `research.md` summarizing decisions with rationale, benchmarks, and migration notes.  
   - Clear documentation that no unresolved ambiguities remain.

## Phase 1: Design & Contracts
1. **Data Model Updates**
   - Extend `JwtContext` (or equivalent) with refresh margin, clamp tracking, and warning flags.  
   - Represent deprecation state for user-supplied JWT configuration.  
   - Define back-off metadata for handling the fixed 2s pause on 429 responses.

2. **Contracts & Interfaces**
   - Document `ensure_valid_jwt()` behavior for refresh-on-demand and clamp warnings.  
   - Specify retry workflow for `send_request` covering 401 and 429 cases.  
   - Describe logging contract (warning on 401 before retry, info on refresh, debug on reuse).

3. **Testing Strategy**
   - Unit tests for clamping logic and warning emission.  
   - Unit tests ensuring refresh occurs when margin threshold reached.  
   - Integration tests with `wiremock`: 401 success-after-retry, 401 failure-after-retry, 429 with randomized back-off.  
   - Tests covering deprecation warning when integrator supplies JWT directly.

4. **Quickstart Guidance**
   - Steps for configuring exp seconds and observing clamp warnings.  
   - Commands to run targeted tests validating retry logic.  
   - Example logs demonstrating refresh and 401 handling.

5. **Agent Context Update**
   - After plan completion, rerun `.specify/scripts/bash/update-agent-context.sh claude` once sed patterns are fixed to capture updated Rust/JWT context.

## Phase 2: Task Planning Approach
**Task Generation Strategy**:
- Derive tasks from research/design outputs: configuration validation, JWT cache updates, retry/back-off handling, deprecation messaging, and tests preceding implementation.  
- Highlight tasks that can proceed independently (e.g., logging tests vs. retry tests) for potential parallelism.

**Ordering Strategy**:
- Start with configuration validation + clamping logic (tests, then implementation).  
- Follow with JWT context refresh behavior, then request pipeline retry/back-off changes.  
- Close with documentation updates (quickstart, migration notes).

**Estimated Output**: ~20 ordered tasks; mark test-writing tasks as [P] where they do not share code paths.

## Phase 3+: Future Implementation
- Phase 3: `/tasks` command produces actionable checklist.  
- Phase 4: Execute tasks following TDD.  
- Phase 5: Validate with automated tests and sample run against mocked Snowflake API.

## Complexity Tracking
No constitution deviations identified; section reserved for future governance updates.

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
- [ ] Complexity deviations documented

---
*Based on Constitution (pending content) - See `/memory/constitution.md`*
