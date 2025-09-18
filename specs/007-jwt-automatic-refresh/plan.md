# Implementation Plan: JWT Automatic Refresh

**Branch**: `007-jwt-automatic-refresh` | **Date**: 2025-09-17 | **Spec**: /Users/drewmca/Coding/snowpipe-streaming-rs/specs/007-jwt-automatic-refresh/spec.md
**Input**: Feature specification from `/specs/007-jwt-automatic-refresh/spec.md`

## Execution Flow (/plan command scope)
```
1. Load feature spec from Input path
   → If not found: ERROR "No feature spec at {path}"
2. Fill Technical Context (scan for NEEDS CLARIFICATION)
   → Detect Project Type from context (web=frontend+backend, mobile=app+api)
   → Set Structure Decision based on project type
3. Fill the Constitution Check section based on the content of the constitution document.
4. Evaluate Constitution Check section below
   → If violations exist: Document in Complexity Tracking
   → If no justification possible: ERROR "Simplify approach first"
   → Update Progress Tracking: Initial Constitution Check
5. Execute Phase 0 → research.md
   → If NEEDS CLARIFICATION remain: ERROR "Resolve unknowns"
6. Execute Phase 1 → contracts, data-model.md, quickstart.md, agent-specific template file (e.g., `CLAUDE.md` for Claude Code, `.github/copilot-instructions.md` for GitHub Copilot, `GEMINI.md` for Gemini CLI, `QWEN.md` for Qwen Code or `AGENTS.md` for opencode).
7. Re-evaluate Constitution Check section
   → If new violations: Refactor design, return to Phase 1
   → Update Progress Tracking: Post-Design Constitution Check
8. Plan Phase 2 → Describe task generation approach (DO NOT create tasks.md)
9. STOP - Ready for /tasks command
```

**IMPORTANT**: The /plan command STOPS at step 7. Phases 2-4 are executed by other commands:
- Phase 2: /tasks command creates tasks.md
- Phase 3-4: Implementation execution (manual or via tools)

## Summary
Proactively refresh Snowflake keypair JWTs using a dedicated Token Guard that evaluates expiry against a 120-second or 20% remaining-window threshold (whichever is smaller), centralizes retry/backoff rules for all outbound Snowflake calls, and keeps ingest traffic flowing without duplicated token handling paths.

## Technical Context
**Language/Version**: Rust 1.78+ (edition 2024)  
**Primary Dependencies**: reqwest 0.12 (async HTTP), tokio 1.47 (runtime), jsonwebtoken 9.x, tracing 0.1  
**Storage**: N/A (in-memory token cache only)  
**Testing**: cargo test with wiremock for HTTP simulations  
**Target Platform**: Linux/macOS services consuming the Rust SDK  
**Project Type**: single (Rust async library)  
**Performance Goals**: <5ms per request overhead from refresh checks; sustain 500 req/min/channel without extra latency  
**Constraints**: Centralize retry policy with capped exponential backoff + jitter; reuse HTTP client handles; avoid blocking crypto paths; maintain structured tracing on refresh outcomes  
**Scale/Scope**: Up to 100 concurrent ingest channels each issuing sub-minute batches

## Constitution Check
- **Observation**: `.specify/memory/constitution.md` is a placeholder with no binding principles; default governance requires simplicity and explicit documentation.
- **Initial Assessment**: Centralizing retry logic reduces complexity and duplication, aligning with the implied simplicity requirement; no conflicts identified.
- **Post-Design Review**: Phase 1 outputs maintain a single-project layout and avoid unnecessary abstractions—no violations introduced.

## Project Structure

### Documentation (this feature)
```
specs/007-jwt-automatic-refresh/
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
├── models/
├── services/
├── cli/
└── lib/

tests/
├── contract/
├── integration/
└── unit/
```

**Structure Decision**: Option 1 – Single Rust library (no separate frontend/backend).

## Phase 0: Outline & Research
1. **Unknowns to Resolve**
   - Refresh window definition, notification expectations, and audit retention from the spec.
   - Best practice for centralizing retry logic without hurting throughput.
   - Edge-case handling when refresh fails or multiple requests race refresh operations.
2. **Research Tasks & Outputs** (documented in `research.md`)
   - Establish proactive refresh threshold and guard timings for Snowflake JWTs.
   - Define stakeholder notification and observability channels leveraging existing tracing.
   - Decide on retention expectations for refresh telemetry.
   - Summarize retry policy structure (shared exponential backoff + telemetry hooks).
3. **Result**: `research.md` records decisions, rationales, and rejected alternatives so downstream design is unambiguous.

## Phase 1: Design & Contracts
1. **Data Model (`data-model.md`)**
   - Document `TokenEnvelope`, `RefreshPolicy`, `RetryPlan`, and `RequestDispatchContext` entities with state transitions and validation.
2. **API Contracts (`contracts/*`)**
   - Provide an OpenAPI description for the SDK-facing control surface (`POST /sdk/request` guarded by `ensureToken`) and an internal refresh endpoint (`POST /sdk/token/refresh`) to drive contract tests.
3. **Contract Tests**
   - Outline failing tests (in `/tests/contract/`) asserting the Token Guard refreshes before TTL threshold and that retries follow the centralized policy.
4. **Quickstart (`quickstart.md`)**
   - Step-by-step guide showing configuration of refresh policy, running cargo tests, and observing token rollover + retry behavior with tracing.
5. **Agent Context**
   - Update `AGENTS.md` via `.specify/scripts/bash/update-agent-context.sh opencode`; on macOS, fall back to a manual edit mirroring the script when BSD `sed` blocks execution.
6. **Outcome**: Phase 1 artifacts capture how retry orchestration, token refreshing, and failure handling integrate without duplicative code paths.

## Phase 2: Task Planning Approach
**Task Generation Strategy**:
- Derive tasks from contracts (`ensureToken` flow, retry orchestration) and data model entities.
- Map quickstart flow to integration and performance validation tasks.
- Prioritize building contract tests before implementation to satisfy TDD expectations.

**Ordering Strategy**:
- Generate contract and integration tests first; follow with token manager + retry coordinator implementation.
- Gate performance-focused tasks after correctness paths but before documentation polish.
- Use `[P]` markers for independent test additions (e.g., parallel contract specs).

**Estimated Output**: 25–30 ordered tasks in `tasks.md` once `/tasks` command runs.

## Phase 3+: Future Implementation
- **Phase 3**: Execute generated tasks to implement token guard, retry coordinator, and observability hooks.
- **Phase 4**: Integrate changes into SDK, resolve tests, and refactor duplicated code paths.
- **Phase 5**: Validate performance targets, run quickstart scenario, and gather release sign-off.

## Complexity Tracking
No constitution deviations identified; centralized retry logic reduces overall complexity. Ingest-path retry refactor is intentionally deferred to a follow-up feature to keep this delivery scoped to control-plane refresh.

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
*Based on Constitution v2.1.1 - See `/memory/constitution.md`*
