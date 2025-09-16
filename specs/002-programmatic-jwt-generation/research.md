## Research: OAuth2 Client Assertion for Snowflake

Endpoint
- Control-plane OAuth2 endpoint: `POST {control}/oauth2/token`
- Content-Type: `application/x-www-form-urlencoded`

Request parameters (per Snowflake docs; confirm exact values):
- `grant_type`: `urn:ietf:params:oauth:grant-type:jwt-bearer` (or `client_credentials` if required)
- `client_assertion_type`: `urn:ietf:params:oauth:client-assertion-type:jwt-bearer`
- `client_assertion`: JWT signed with account private key
- Optional: `scope` (e.g., ingest hostname) or token audience, depending on Snowflake configuration

Client assertion claims
- `iss`: account/user identifier
- `sub`: same as `iss` (client identification)
- `aud`: `{control}/oauth2/token`
- `iat`: now (seconds)
- `exp`: short-lived (e.g., +60 seconds)
- `jti`: unique ID (nonce)

Signing
- Algorithm: RS256
- Key: private key in PKCS#8 (PEM) or DER
- Optional: encrypted PEM (passphrase) [clarify support in v1]

Validation & Error Handling
- Parse PEM → DER → keypair for signing
- Clear errors for invalid formats, unsupported encryption, or signature failures
- Cache token until `exp - skew`

Testing Strategy
- Wiremock endpoint `POST /oauth2/token` returns JSON `{access_token, token_type, expires_in}`
- Tests verify proper form body and successful end-to-end client initialization without an input `jwt_token` in config
