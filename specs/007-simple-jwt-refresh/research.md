# Phase 0 Research: Simple JWT Refresh Logic

## Objectives
- Maintain uninterrupted Snowpipe streaming sessions by refreshing JWTs pre-emptively without exposing implementation burden to integrators.
- Enforce a minimum configuration window (≥30 seconds) for JWT validity, clamping out-of-range values with warnings.
- Retry unauthorized (`401`) and rate-limited (`429`) API responses in a deterministic, observable manner.
- Deprecate consumer-supplied JWTs in favor of private-key driven generation managed entirely by the library.

## Key Findings & Decisions

### Refresh Timing & Cadence
- **Decision**: Trigger refresh when remaining lifetime ≤ configured margin (default 30 seconds) and regenerate tokens indefinitely using the in-memory private key.  
  **Rationale**: Aligns with Snowflake’s expectation for short-lived JWTs while ensuring long-lived clients never expire unexpectedly.  
  **Alternatives Considered**: Fixed refresh intervals or background timers—rejected due to unnecessary complexity for single-instance clients.

- **Decision**: Auto-clamp `jwt_exp_secs` to `[30, 3600]` and emit a warning when clamping occurs.  
  **Rationale**: Honors user intent (shorter lifetimes for security) without surfacing configuration errors.  
  **Implementation Note**: Logging should include supplied and adjusted values for observability.

### Failure Handling & Messaging
- **Decision**: On `401 Unauthorized`, log a warning, refresh the JWT, and retry exactly once. If the retry fails with `401`, return `Error::Auth`.  
  **Rationale**: Keeps retry policy simple and prevents infinite loops while surfacing credential problems promptly.

- **Decision**: On `429 TOO MANY REQUESTS`, sleep a randomized 1–3 seconds before retrying the request with the current or newly refreshed JWT.  
  **Rationale**: Provides basic back pressure without overengineering concurrency controls for single-instance clients.

### Security & Credential Management
- **Decision**: Treat consumer-supplied JWT configuration as deprecated—emit a warning and rely on library-managed generation using the private key.  
  **Rationale**: Avoids integrator misconfiguration and guarantees refresh coverage.  
  **Consequence**: Documentation and quickstart must steer users toward supplying only private keys.

- **Decision**: Credentials (private keys, issued tokens) remain in-memory; no additional storage or rotation behavior is in-scope.  
  **Rationale**: Matches current deployment model and user expectations.

### Performance
- **Observation**: RSA signing latency in local benchmarks (2048-bit key) stays below 10 ms; acceptable for on-demand refresh.  
  **Action**: Record benchmark method in tests to guard against regressions.

## References
- Snowflake documentation: "Using key pair authentication (JWT)" – sections on token lifetime and retry guidance.  
- Existing modules: `src/client/crypto.rs`, `src/client/impls.rs`, error and tracing conventions in `src/client/tests.rs`.

## Outstanding Questions
None. All clarifications provided by stakeholders on 2025-09-18.
