# Tasks: Encrypted PEM Support

## Setup
- [ ] T001 Add `pkcs8` to prod deps; add `rsa` and `rand` to dev-deps

## Tests First
- [ ] T002 Unit: decrypt encrypted PKCS#8 → EncodingKey
- [ ] T003 Integration: OAuth2 flow with encrypted PEM + passphrase (wiremock)

## Implementation
- [ ] T004 Implement key loading:
      - Detect `ENCRYPTED PRIVATE KEY` → decrypt with passphrase (error if missing)
      - `PRIVATE KEY` (PKCS#8) → parse to RSA → PKCS#1 DER
      - Fallback to `from_rsa_pem` for `RSA PRIVATE KEY`
- [ ] T005 Improve errors and logs

## Polish
- [ ] T006 Update README Auth section to include passphrase support and limitations
- [ ] T007 clippy/fmt/tests
