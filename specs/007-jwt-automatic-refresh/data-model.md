# Data Model – JWT Automatic Refresh

## TokenEnvelope
- **Represents**: Active authentication token plus metadata used to decide refresh eligibility.
- **Fields**:
  - `value: String` – Raw JWT used for Authorization headers.
  - `issued_at: Instant` – Creation timestamp for calculating TTL.
  - `expires_at: Instant` – Absolute expiration time returned by Snowflake.
  - `scoped: bool` – Indicates whether token targets ingest host or control plane.
  - `refresh_in_progress: bool` – Flip set while a refresh attempt is underway to coordinate concurrent calls.
- **Validation Rules**:
  - `expires_at` must be > `issued_at` by at least 60 seconds.
  - Guard prohibits reuse if `expires_at - now < RefreshPolicy::threshold`.
- **State Transitions**:
  1. *Active → Refreshing*: When remaining TTL ≤ threshold.
  2. *Refreshing → Active*: On successful renewal, `issued_at`/`expires_at` replaced, `refresh_in_progress` cleared.
  3. *Refreshing → Invalid*: On terminal failure; consumers receive error and must retry per `RetryPlan`.

## RefreshPolicy
- **Represents**: Configurable thresholds governing proactive refresh behavior.
- **Fields**:
  - `threshold: Duration` – Minimum acceptable time-to-expiry (defaults to min(120s, ttl*0.2)).
  - `min_cooldown: Duration` – Required delay between successful refreshes (default 5 minutes).
  - `max_skew: Duration` – Allowed clock skew before forcing refresh (default 30 seconds).
- **Relationships**: Consumed by Token Guard to decide when to refresh; derived from user config or defaults.
- **Validation Rules**: `threshold <= ttl`, `min_cooldown >= 0`, `max_skew < threshold`.

## RetryPlan
- **Represents**: Shared retry/backoff strategy for Snowflake-bound HTTP operations.
- **Fields**:
  - `max_attempts: u8` – Defaults to 4.
  - `initial_delay: Duration` – Defaults to 200 ms.
  - `multiplier: f32` – Defaults to 1.8.
  - `max_delay: Duration` – Caps sleep at 5 seconds.
  - `jitter: JitterStrategy` – Enum selecting full jitter vs. decorrelated jitter.
- **Relationships**: Injected into control host discovery, scoped token fetch, channel operations, and data ingest.
- **State Transitions**: Maintains per-operation attempt count, short-circuits when non-retriable error occurs.

## RetryOutcome
- **Represents**: Result of executing a retry plan for a specific operation.
- **Fields**:
  - `operation: OperationKind` – Enum for `DiscoverHost`, `GetScopedToken`, `OpenChannel`, `SendBatch`.
  - `attempts: u8` – Total attempts executed.
  - `final_status: Result<(), Error>` – Whether operation ultimately succeeded.
  - `latency_budget: Duration` – Total time spent retrying vs. allowed budget.
- **Usage**: Logged for observability and fed into metrics; used to trigger escalation when repeated failures occur.

## RequestDispatchContext
- **Represents**: Immutable snapshot passed to ingest requests ensuring they use current token + retry plan.
- **Fields**:
  - `http_client: Arc<Client>` – Reused reqwest client instance.
  - `token: Arc<RwLock<TokenEnvelope>>` – Shared token cache protected by async lock.
  - `retry_plan: RetryPlan` – Shared policy instance.
  - `refresh_policy: RefreshPolicy` – Shared thresholds for token refresh.
- **Relationships**: Created by `StreamingIngestClient` during initialization; cloned per channel/request.
- **Validation Rules**: Must be built with valid token envelope (refresh called during initialization).

## RefreshTelemetry
- **Represents**: Structured event capturing refresh attempts for auditing.
- **Fields**:
  - `attempt_id: Uuid` – Unique identifier correlating distributed traces.
  - `issued_at`, `expires_at`: Mirror from `TokenEnvelope`.
  - `outcome: RefreshOutcome` – Enum for `Success`, `Retrying`, `Failed`.
  - `jitter_applied: Duration` – Backoff delay used before this attempt.
  - `error_kind: Option<ErrorKind>` – Semantic error classification when failures occur.
- **Retention**: Emitted via `tracing` and retained 90 days by log pipeline.
