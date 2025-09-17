# Phase 0 Research: Fix JWT Generation

Decision 1: Generate JWT client-side (no `/oauth2/token`)
- Rationale: Snowflake key pair authentication requires clients to sign a JWT locally using the account/user identifiers and private key; there is no token issuance endpoint.
- Alternatives considered: Request token from `/oauth2/token` (rejected—endpoint not applicable for key pair auth), use OAuth flow (out of scope for Snowpipe key pair auth).

Decision 2: Support private key via path or inline string (optional passphrase)
- Rationale: Enables CI/containers to inject secrets securely while also supporting filesystem-based keys for local dev.
- Alternatives: Path-only (too restrictive), env-only (less flexible, complicates local usage).

Decision 3: Claims and signing
- Rationale: Use RS256 with required claims: `iss`, `sub`, `aud`, `iat`, `exp`, and a unique `jti`.
- Alternatives: Other algorithms (ES256) postponed; keep scope aligned with current dependencies.

Decision 4: Token caching and refresh
- Rationale: JWT valid for 1 hour; cache token and refresh proactively 5 minutes before expiry to avoid mid-request failures; also refresh reactively if server indicates expiry.
- Alternatives:
  - Reactive-only refresh (simpler but risks request failures under load).
  - Always generate per request (unnecessary CPU and key handling overhead).

Open Question: Exact server error semantics for expired JWT
- Current approach: Implement proactive refresh and treat 401/403 with explicit “expired/invalid token” semantics as a trigger to refresh and retry once.
- Follow-up: Verify Snowflake response codes/messages for JWT expiry in integration tests and adjust matching accordingly.

Security Considerations
- Keep private key only in memory; avoid writing to disk.
- Support encrypted PEM with passphrase when provided; otherwise reject encrypted keys without passphrase.
- Minimize token lifetime to 1 hour per requirement and avoid overlong expirations.

Testing Strategy
- Unit tests for claim construction, expiration boundaries, and refresh scheduler logic.
- Integration tests with mocked HTTP to verify Authorization header and refresh-on-error behavior.
- Property tests around clock skew boundaries (±N seconds) if practical.

Summary
- Implement client-side JWT generation with robust caching and refresh.
- Remove any calls or references to `/oauth2/token` in client setup and tests.
