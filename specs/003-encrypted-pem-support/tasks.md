# Tasks: Encrypted PEM Support

## Setup
- [x] T001 Add `pkcs8` to prod deps; add `rsa` and `rand` to dev-deps

## Tests First
- [x] T002 Unit: decrypt encrypted PKCS#8 → EncodingKey
- [x] T003 Integration: OAuth2 flow with encrypted PEM + passphrase (wiremock)

## Implementation
- [x] T004 Implement key loading:
      - Detect `ENCRYPTED PRIVATE KEY` → decrypt with passphrase (error if missing)
      - `PRIVATE KEY` (PKCS#8) → parse to RSA → PKCS#1 DER
      - Fallback to `from_rsa_pem` for `RSA PRIVATE KEY`
- [x] T005 Improve errors and logs

## Polish
- [x] T006 Update README Auth section to include passphrase support and limitations
- [x] T007 clippy/fmt/tests (CI)
- [x] T008 Optimize tests: use pre-generated assertion in integration tests; keep decryption/signing in unit test to reduce runtime
