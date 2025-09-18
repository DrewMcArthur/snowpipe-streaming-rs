# Contract: JWT Refresh Workflow

## Overview
The Snowpipe client must guarantee a valid JWT for every outbound HTTP request while gracefully handling Snowflake responses that indicate invalid credentials or rate limiting.

## Interfaces

### `SnowpipeClient::ensure_valid_jwt()`
- **Purpose**: Returns a JWT with more than the configured safety margin remaining.  
- **Inputs**: `&self` (access to config, crypto utilities, private key).  
- **Outputs**: `Result<String, Error>` – refreshed token or authentication error.  
- **Behavior**:
  1. Read `JwtContext`. If token missing or `remaining_ttl <= refresh_margin`, lock the context for refresh.  
  2. Clamp configured expiration into `[30, 3600]`; log warning if clamping performed.  
  3. Generate new JWT via `crypto::generate_assertion` and update context metadata.  
  4. Return cached token for reuse until next margin check.
- **Errors**: `Error::Config` (invalid configuration after clamping), `Error::JwtSign`, `Error::Auth` (refresh failed).

### `SnowpipeClient::send_request(req)`
- **Precondition**: Caller provides request without `Authorization` header.  
- **Workflow**:
  1. Call `ensure_valid_jwt()`; attach `Authorization: Bearer {token}`.  
  2. Dispatch request via `reqwest`.  
  3. If response status == 429, log a warning, sleep exactly 2 seconds, optionally refresh if margin requires, then retry once.  
  4. If response status == 401, log warning, refresh token, and retry once.  
  5. Return retry response. On second 401, surface `Error::Auth`; on other errors, propagate existing behavior.

### Logging / Tracing
- `info`: JWT refreshed (include remaining TTL, clamp details).  
- `warn`: 401 received, retrying with new JWT; deprecation notice when user supplies JWT manually.  
- `warn`: 429 encountered, sleeping 2 seconds before retry.

## Test Contracts

### Unit Test: Clamp and Refresh
- **Given** config with `jwt_exp_secs = 10`, **When** context initializes, **Then** it clamps to 30 seconds, logs warning once, and refreshes before first request.

### Unit Test: Refresh Threshold
- **Given** token with 25 seconds remaining and margin 30 seconds, **When** `ensure_valid_jwt()` called, **Then** new token issued and context updated.

### Integration Test: Retry on 401
- **Given** Snowflake mock returns 401 then 200, **When** client sends request, **Then** warning log emitted, token refreshed, and 200 response returned.

### Integration Test: Double 401 Fails
- **Given** Snowflake mock returns 401 twice, **When** client sends request, **Then** it retries once, returns `Error::Auth`, and does not attempt additional retries.

### Integration Test: Back-off on 429
- **Given** Snowflake mock returns 429 then 200, **When** client sends request, **Then** client logs a warning, sleeps 2 seconds, retries, and returns 200.

### Unit Test: Deprecation Warning for User-Supplied JWT
- **Given** config includes `user_supplied_jwt`, **When** client initializes, **Then** a deprecation warning is logged and automatic refresh is bypassed.

## Non-Goals
- Managing background refresh threads or multi-client coordination.  
- Persisting credentials beyond in-memory scope.  
- Providing user-facing reauthentication messaging—the library surfaces errors to integrators only.
