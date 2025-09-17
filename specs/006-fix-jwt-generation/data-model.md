# Data Model: JWT Auth and Client

Entities
- AuthConfig
  - account_identifier: String
  - login_name: String (Snowflake username)
  - audience: String (Snowflake audience for JWT)
  - expiry_secs: u64 (default 3600)
  - key_source: KeySource
  - passphrase: Option<String>
- KeySource (enum)
  - Path(path: String)
  - Inline(pem: String)
- JwtClaims
  - iss (issuer): String
  - sub (subject): String
  - aud (audience): String
  - iat (issued-at): i64 (epoch seconds)
  - exp (expiration): i64 (epoch seconds)
  - jti (unique id): String (UUID v4)
- TokenCache
  - token: String (JWT)
  - iat: i64
  - exp: i64
- Client
  - auth: AuthConfig
  - cache: Option<TokenCache>
  - http: HttpClient (abstraction over reqwest for testing)

Relationships
- Client holds AuthConfig and optional TokenCache
- TokenCache derived from JwtClaims used to compute refresh timings

Validation Rules
- expiry_secs MUST be <= 3600 (1 hour) per requirement
- passphrase MUST be provided for encrypted PEM keys; reject otherwise
- clock skew: allow ±60s when deciding proactive refresh

State Transitions
- Client initialization → generate JWT → set TokenCache
- Before request → if now >= exp - 300 → regenerate JWT
- On 401/403 with token-expired semantics → regenerate JWT and retry once
