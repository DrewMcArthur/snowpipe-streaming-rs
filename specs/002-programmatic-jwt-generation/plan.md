 # Implementation Plan: Programmatic JWT Generation
 
 **Branch**: `[002-programmatic-jwt-generation]` | **Date**: 2025-09-16 | **Spec**: /Users/drewmca/Coding/snowpipe-streaming-rs/specs/002-programmatic-jwt-generation/spec.md
 **Input**: Feature specification from `/specs/002-programmatic-jwt-generation/spec.md`
 
 ## Execution Flow (/plan command scope)
 ```
 1. Load feature spec from Input path
 2. Fill Technical Context and unknowns
 3. Constitution check
 4. Phase 0 → research.md (OAuth2/JWT details; key parsing)
 5. Phase 1 → contracts/ + quickstart.md
 6. Re-check constitution
 7. Plan tasks for /tasks (do not create tasks.md)
 ```
 
 ## Summary
 Enable the client to programmatically obtain a Snowflake control-plane access token using a private key, without requiring a pre-generated JWT from the constructor. Use Snowflake's OAuth2 endpoint (`/oauth2/token`) with a JWT client assertion signed by the provided private key. Maintain existing scoped-token flow after control-plane auth.
 
 ## Technical Context
 **Language/Version**: Rust 2024  
 **Primary Dependencies**: `reqwest`, `tokio`, `serde/serde_json`, `tracing`  
 **New Dev/Prod Deps (proposed)**: `jsonwebtoken` (RS256 signing), `pem` (PEM parsing) or equivalent  
 **Storage**: N/A  
 **Testing**: `cargo test` with `wiremock` for `/oauth2/token` and follow-on calls  
 **Target Platform**: Linux/macOS CI, developer machines  
 **Project Type**: single  
 **Constraints**: No real credentials in tests; support PEM key string or file path; optional passphrase [clarify]
 
 ## Constitution Check
 **Simplicity**: Single lib, minimal new deps (JWT + PEM).  
 **Architecture**: Integrate into existing `StreamingIngestClient::new` flow: 
 1) generate control-plane token via `/oauth2/token` with client assertion; 
 2) discover ingest host; 
 3) get scoped token.  
 **Testing**: Wiremock stubs for `/oauth2/token`, `/v2/streaming/hostname`, and `/oauth/token`; unit tests for key parsing and JWT construction.  
 **Observability**: Tracing logs for token generation path; clear error mapping.  
 **Versioning**: Internal; no breaking public API beyond constructor signature change (add config fields).
 
 ## Project Structure
 
 ### Documentation (this feature)
 ```
 specs/002-programmatic-jwt-generation/
 ├── plan.md
 ├── research.md
 ├── quickstart.md
 └── contracts/
     ├── oauth2_token_request.json
     └── oauth2_token_response.json
 ```
 
 ### Source Code (repository root)
 ```
 src/
 ├── config.rs        # Add private_key/private_key_path/passphrase support for JWT
 ├── client.rs        # Add oauth2 token exchange step using client assertion
 └── errors.rs        # Add error variants for key parsing/signing
 tests/
 └── integration.rs   # Wiremock test for oauth2 flow
 ```
 
 **Structure Decision**: Single project; integrate into existing client.
 
 ## Phase 0: Outline & Research
 Unknowns and decisions:
 - OAuth2 flow: Use JWT bearer client assertion to `/oauth2/token`. Request fields include `grant_type`, `client_assertion_type`, `client_assertion`, and `scope` or `audience` as required by Snowflake docs.  
 - Claims: `iss`/`sub` (Snowflake user), `aud` (control host oauth2 token endpoint), `iat`/`exp` (short-lived, e.g., 60s).  
 - Signing: RS256 with account private key. Parse PKCS#8 PEM; optionally handle encrypted PEM (if passphrase available).  
 - Config: Add fields for private key string or key path, optional passphrase, account and user, token lifetime.
 - Caching: Optional in-memory cache for control-plane token until expiration; re-generate on expiry.
 
 Output: research.md with endpoint parameters, example claims, and key parsing approach.
 
 ## Phase 1: Design & Contracts
 Design highlights:
 - Extend `Config` to support:
   - `private_key` (string) or `private_key_path` (file path)
   - `private_key_passphrase` (optional)
   - `jwt_exp_secs` (optional, default e.g. 60)
 - On `StreamingIngestClient::new`:
   - If `jwt_token` is absent, generate a client assertion JWT and POST to `{control}/oauth2/token` to obtain control-plane bearer token.
   - Continue existing steps: discover ingest host, then get scoped token using the control-plane token.
 - Errors: map key parsing/signing and HTTP errors with clear messages.
 
 Contracts (samples in contracts/):
 - `POST {control}/oauth2/token` (application/x-www-form-urlencoded)
   - Body fields: `grant_type=client_credentials` or `urn:ietf:params:oauth:grant-type:jwt-bearer` (per Snowflake docs), `client_assertion_type=urn:ietf:params:oauth:client-assertion-type:jwt-bearer`, `client_assertion=<JWT>`.
   - Response (JSON): `{ "access_token": "...", "token_type": "Bearer", "expires_in": 3600 }`.
 
 Quickstart: Show config example with private key path and generated token flow.
 
 ## Phase 2: Task Planning Approach
 Tasks to generate in /tasks:
 - Add config fields and migration path.
 - Add JWT signing utility (key parsing + claims + signing) and request to `/oauth2/token`.
 - Integration tests with wiremock for the OAuth2 token endpoint and end-to-end client init.
 - Unit tests for key parsing, claims, and error cases.
 - Docs updates: README Testing and usage.
 
 Ordering: tests first (failing), then implementation; clippy/fmt gates in CI.
 
 Estimated Output: ~12–18 tasks.
 
 ## Complexity Tracking
 | Violation | Why Needed | Simpler Alternative Rejected Because |
 |-----------|------------|-------------------------------------|
 | New crypto/JWT deps | Required to sign client assertion | Shelling to snowsql is brittle and not suitable for services |
 
 ## Progress Tracking
 **Phase Status**:
 - [x] Phase 0: Research planned
 - [x] Phase 1: Design planned
 - [ ] Phase 2: Task planning (to be generated by /tasks)
 
 **Gate Status**:
 - [x] Initial Constitution Check: PASS
 - [x] Post-Design Constitution Check: PASS
 - [ ] All NEEDS CLARIFICATION resolved
 - [ ] Complexity deviations documented
 
 ---
 *Based on Constitution v2.1.1 - See `/memory/constitution.md`*
