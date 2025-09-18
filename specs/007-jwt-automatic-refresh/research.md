# Phase 0 Research – JWT Automatic Refresh

## Token Refresh Threshold
- **Decision**: Refresh whenever remaining TTL is ≤120 seconds *or* 20% of the original validity window (whichever is smaller), and require a minimum 5-minute cooldown between successive refreshes unless forced by invalidation.
- **Rationale**: Keeps proactive refresh far enough ahead of expiry to cover network jitter without over-refreshing during high-volume ingest; aligns with Snowflake guidance that JWTs remain valid for 1 hour by default.
- **Alternatives Considered**:
  - *Fixed 5-minute window*: Rejected because short-lived tokens (≤10 minutes) would refresh too early and generate unnecessary load.
  - *Refresh at 50% TTL*: Rejected for wasting usable time and adding crypto overhead without reliability gains.

## Stakeholder Notification & Observability
- **Decision**: Emit structured `tracing` events and metrics for refresh start, success, failure, and fallback; escalate persistent failures (>3 consecutive) via existing logging integration and bubble an `Error::JwtSign` or `Error::Reqwest` to callers for manual intervention.
- **Rationale**: Leverages existing tracing infrastructure without adding new notification channels; actionable telemetry appears in the operators’ log pipeline and maintains high performance.
- **Alternatives Considered**:
  - *Direct paging/Slack integration*: Rejected as out of scope for the SDK and would introduce external dependencies.
  - *Silent retries only*: Rejected because stakeholders would lack insight into chronic refresh failure.

## Refresh Audit Retention
- **Decision**: Persist refresh attempt metadata (issuer, issued_at, expiry, outcome code, jitter applied) in structured logs retained for 90 days via the downstream log aggregator; no on-disk persistence inside the SDK.
- **Rationale**: Meets compliance expectations without coupling the library to a storage backend or degrading performance.
- **Alternatives Considered**:
  - *Local file-based audit trail*: Rejected due to portability challenges and I/O overhead.
  - *Unlimited retention*: Rejected as unnecessary and potentially costly for consumers.

## Centralized Retry Policy
- **Decision**: Introduce a `RetryPlan` shared across control-plane discovery, scoped token acquisition, and ingest requests using capped exponential backoff (initial 200 ms, multiplier 1.8, cap 5 seconds) with full jitter and a maximum of 4 attempts before surfacing an error.
- **Rationale**: Aligns with AWS and Google recommendations for resilient HTTP retries while keeping latency overhead bounded for high-throughput ingest.
- **Alternatives Considered**:
  - *Per-call bespoke retries*: Rejected because it reintroduces duplication and inconsistent behavior.
  - *Unlimited retries*: Rejected for risking unbounded delays and cascading failures.

## Race Conditions & Edge Handling
- **Decision**: Guard refresh with an async mutex and share the refreshed token via an atomic snapshot so concurrent requests reuse the same renewed credential; allow callers to force refresh when they receive an authentication failure.
- **Rationale**: Prevents redundant refresh attempts, keeps latency low, and ensures compliant behavior when multiple tasks detect impending expiry.
- **Alternatives Considered**:
  - *Fire-and-forget refresh per request*: Rejected because it multiplies load and introduces race bugs.
  - *Single-threaded refresh worker*: Rejected for complicating integration in synchronous contexts and hurting responsiveness.
