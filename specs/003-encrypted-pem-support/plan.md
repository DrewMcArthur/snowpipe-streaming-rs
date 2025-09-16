# Implementation Plan: Encrypted PEM Support

**Branch**: `[003-encrypted-pem-support]` | **Date**: 2025-09-16 | **Spec**: specs/003-encrypted-pem-support/spec.md

## Summary
Add support for encrypted PKCS#8 PEM private keys when generating control-plane JWTs. Decrypt keys using the provided passphrase and derive a signing key for RS256.

## Technical Context
- Use `pkcs8` crate to parse/decrypt `EncryptedPrivateKeyInfo`.
- For PKCS#8 unencrypted, parse as usual. For PKCS#1, keep existing `jsonwebtoken::EncodingKey::from_rsa_pem`.
- For encrypted PKCS#8, decrypt to DER, convert to PKCS#1 using `rsa` crate if needed, and feed to `EncodingKey::from_rsa_der`.

## Tasks
- Add dependency: `pkcs8` (prod), `rsa` and `rand` (dev for tests).
- Implement decryption in key-loading path.
- Unit test: generate RSA key, encode PKCS#8, encrypt with passphrase, verify decryption builds a usable `EncodingKey`.
- Integration test: programmatic JWT flow using encrypted PEM + passphrase against mocked OAuth2 endpoint.
- Update README to document passphrase support and limitations.
