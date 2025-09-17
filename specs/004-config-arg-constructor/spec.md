# Feature Specification: Config Argument in Client Constructor

**Feature Branch**: `[004-config-arg-constructor]`  
**Created**: 2025-09-16  
**Status**: Draft  
**Input**: User description: "config arg for client construction rather than config location"

## User Scenarios & Testing

- As a developer, I can construct `StreamingIngestClient` by passing a ready `Config` object (or builder output) instead of a `ConfigLocation`, avoiding internal I/O/env lookups and enabling dependency injection and testability.
- Backward-compat: existing `ConfigLocation`-based constructor is available for a deprecation period or delegates to the new API.

## Requirements

- FR-001: Add a constructor that accepts a `Config` (or reference) directly and performs no file/env reads.
- FR-002: Preserve (or soft-deprecate) the `ConfigLocation` constructor; implement via shared path.
- FR-003: Ensure programmatic JWT generation and encrypted PEM support work unchanged when config is passed directly.
- FR-004: Update tests to use the new constructor where appropriate; keep location-based tests for coverage temporarily.
- FR-005: Document the new constructor and migration guidance.

## Acceptance Criteria

- New constructor exists and is used in tests without reading files/env.
- All prior features (client-side keypair JWT generation, encrypted PEM, timeouts) operate via the new path.
- CI passes: clippy/fmt/tests.

## Notes / Open Questions

- Should the new constructor take `&Config`, owned `Config`, or a builder pattern? [NEEDS CLARIFICATION]
- Deprecation plan for `ConfigLocation` constructor timing and warnings. [NEEDS CLARIFICATION]
