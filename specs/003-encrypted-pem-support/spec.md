# Feature Specification: Encrypted PEM Support

**Feature Branch**: `[003-encrypted-pem-support]`  
**Created**: 2025-09-16  
**Status**: Draft  
**Input**: User description: "support encrypted PEM private keys"

## User Scenarios & Testing
- As a developer, I can provide an encrypted PKCS#8 private key (PEM) with a passphrase and the client will generate control-plane tokens successfully.
- If no passphrase is provided for an encrypted key, the client returns an actionable error without panicking.

## Requirements
- FR-001: Detect encrypted PKCS#8 PEM ("ENCRYPTED PRIVATE KEY") and decrypt using provided passphrase.
- FR-002: Support unencrypted PKCS#8 ("PRIVATE KEY") and PKCS#1 ("RSA PRIVATE KEY") as before.
- FR-003: Return clear errors for missing/incorrect passphrase or unsupported formats.
- FR-004: Add tests that exercise decryption path (unit) and end-to-end (integration via mocked server).

## Acceptance
- Programmatic JWT flow works with encrypted PEM + passphrase.
- Lints and formatting clean; docs mention passphrase support.
