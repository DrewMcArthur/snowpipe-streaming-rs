# Data Model: Simple JWT Refresh Logic

## Entities

### JwtContext
- **Purpose**: Holds the currently active JWT and timing metadata for the Snowpipe client.  
- **Attributes**:  
  - `token: String` – signed JWT used for authentication.  
  - `issued_at: u64` – UNIX timestamp when token was generated.  
  - `expires_at: u64` – UNIX timestamp when token becomes invalid.  
  - `refresh_margin: Duration` – safety buffer (default 30s) before expiry that triggers refresh.  
  - `was_clamped: bool` – indicates whether the configured lifetime was adjusted.  
  - `last_refresh_warning: Option<Instant>` – throttles repeated warning logs.
- **Validation Rules**:  
  - Token lifetime (`expires_at - issued_at`) after clamping stays within `[30, 3600]` seconds.  
  - `refresh_margin` ≤ lifetime and ≥30 seconds.

### RetryPolicy
- **Purpose**: Encapsulates automatic retry behavior for Snowflake responses.  
- **Attributes**:  
  - `max_unauthorized_retries: u8` – fixed at 1.  
  - `backoff_range: (Duration, Duration)` – randomized delay window for 429 responses (1s–3s).  
  - `last_backoff: Option<Instant>` – optional telemetry hook.  
- **Validation Rules**:  
  - `backoff_range.0 >= 1s`, `backoff_range.1 <= 3s`, and `start <= end`.

### ClientConfig (extended)
- **Fields**:  
  - `jwt_exp_secs: u64` – clamped into `[30, 3600]` with warning.  
  - `jwt_refresh_margin_secs: u64` – optional override (defaults to 30; must be ≥30 and < effective expiration).  
  - `retry_on_unauthorized: bool` – retains compatibility but defaults to true and may become hard-coded.  
  - `user_supplied_jwt: Option<String>` – flagged as deprecated; usage logs warning and bypasses cache refresh. 
- **Validation Rules**:  
  - When `user_supplied_jwt` present, emit deprecation warning and encourage private key usage.  
  - Ensure clamping logic runs before context initialization.

## Relationships & State Transitions
- `SnowpipeClient` owns a `JwtContext` guarded by an async mutex to serialize refresh operations.  
- Before each request, client checks `JwtContext.needs_refresh()`; if true, it signs a new token using the private key.  
- On 401: client logs a warning, refreshes token, retries once, and surfaces `Error::Auth` on repeat failure.  
- On 429: client sleeps for randomized 1–3 seconds before retrying; refresh logic still applies if margin exceeded.  
- When clamping occurs, context records `was_clamped` to avoid duplicate warnings.

## Derived/Computed Values
- `remaining_ttl()` – returns `expires_at - now`, clamped at zero.  
- `needs_refresh()` – true when token missing or `remaining_ttl() <= refresh_margin`.  
- `clamp_warning_needed()` – indicates whether to log about adjusted expiration.  
- `next_backoff_delay()` – random duration within configured range for 429 responses.

## Notes
- No persistent or shared state; all credentials and tokens remain in process memory.  
- Randomized back-off uses thread-safe RNG (e.g., `rand::Rng`).  
- Deprecation messaging for `user_supplied_jwt` should provide migration guidance but continue functioning until removal.
