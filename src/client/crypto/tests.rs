// tests/generate_assertion_tests.rs

use base64::Engine;
use base64::engine::general_purpose::{self, STANDARD, URL_SAFE_NO_PAD};
use pkcs8::{DecodePublicKey, EncodePrivateKey};
use rand::thread_rng;
use rsa::{RsaPrivateKey, RsaPublicKey};
use serde_json::Value;

use crate::client::crypto::{JwtContext, build_assertion, compute_fingerprint};
use crate::tests::test_support::with_captured_logs;
use crate::{Config, Error};

fn generate_assertion(cfg: &Config) -> Result<String, Error> {
    Ok(build_assertion(cfg, true)?.token)
}

fn decode_jwt_payload(jwt: &str) -> Value {
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3, "JWT must have 3 segments");
    let payload_b64 = parts[1];
    let bytes = URL_SAFE_NO_PAD
        .decode(payload_b64.as_bytes())
        .expect("payload must be valid base64url (no padding)");
    serde_json::from_slice::<Value>(&bytes).expect("payload must be valid JSON")
}

/// A minimal PKCS#8 private key PEM for tests. You can replace with your own fixture.
/// This is intended only to exercise signing; the test does not verify the signature.
const TEST_PKCS8_PRIVKEY_PEM: &str = include_str!("../../../tests/fixtures/id_rsa.pem");

/// Happy path: builds a JWT whose claims match Snowflake’s expectations.
/// We assert:
/// - `iss` contains UPPERCASE <ACCOUNT>.<USER>.SHA256:<fingerprint>
/// - `sub` equals UPPERCASE <ACCOUNT>.<USER>
/// - `exp - iat == jwt_exp_secs * 1000` (claims now use milliseconds)
/// - `iat` ≈ now (allow small skew)
#[test]
fn generates_snowflake_style_jwt_claims() {
    // Arrange
    // Example inputs you expect your library to normalize:
    let account = "xy12345.us-east-1"; // many users pass this form
    let user = "alice";
    // In unit tests we don't recompute the fingerprint; we pass the value we expect to see in `iss`.
    // Ensure it starts with "SHA256:".
    let exp_secs = 900u64;

    let cfg = Config {
        user: user.to_string(),
        login: None,
        account: account.to_string(),
        url: "https://xy12345.us-east-1.snowflakecomputing.com".to_string(),
        jwt_token: None,
        private_key: Some(TEST_PKCS8_PRIVKEY_PEM.to_string()),
        private_key_path: None,
        private_key_passphrase: None,
        public_key_fp: None,
        jwt_exp_secs: Some(exp_secs),
        jwt_refresh_margin_secs: None,
        retry_on_unauthorized: None,
    };

    let t0 = super::now_millis().unwrap();

    // Act
    let jwt = generate_assertion(&cfg).expect("should generate a JWT");
    let payload = decode_jwt_payload(&jwt);

    // Assert structure
    let iss = payload
        .get("iss")
        .and_then(|v| v.as_str())
        .expect("iss must exist");
    let sub = payload
        .get("sub")
        .and_then(|v| v.as_str())
        .expect("sub must exist");
    let iat = payload
        .get("iat")
        .and_then(|v| v.as_u64())
        .expect("iat must be u64");
    let exp = payload
        .get("exp")
        .and_then(|v| v.as_u64())
        .expect("exp must be u64");

    // Account & user should be uppercased in both iss/sub.
    let want_account_uc = account.to_uppercase().replace('.', "-"); // common normalization
    let want_user_uc = user.to_uppercase();

    // `sub` is "<ACCOUNT>.<USER>"
    assert!(
        sub == format!("{}.{}", want_account_uc, want_user_uc)
            || sub == format!("{}.{}", account.to_uppercase(), want_user_uc),
        "sub should be '<ACCOUNT>.<USER>' uppercased (with possible '.'→'-' normalization); got '{}'",
        sub
    );

    // `iss` is "<ACCOUNT>.<USER>.SHA256:<fp>"
    assert!(
        iss.contains("SHA256"),
        "iss must contain the SHA256 fingerprint; got '{}'",
        iss
    );
    assert!(
        iss.starts_with(&format!("{}.", sub)),
        "iss should start with '<ACCOUNT>.<USER>.' and then fingerprint; got '{}'",
        iss
    );

    // Time math: exp - iat == jwt_exp_secs and <= 3600
    assert_eq!(
        exp.saturating_sub(iat),
        exp_secs * 1_000,
        "exp - iat must equal jwt_exp_secs in milliseconds"
    );
    assert!(
        exp_secs <= 3600,
        "jwt_exp_secs should be <= 3600 for Snowflake key-pair auth"
    );

    // iat should be close to now (allow generous skew for CI)
    let t1 = super::now_millis().unwrap();
    assert!(
        iat >= t0.saturating_sub(30_000) && iat <= t1.saturating_add(30_000),
        "iat should be near 'now' (±30s); got {}, window [{}, {}]",
        iat,
        t0.saturating_sub(30_000),
        t1.saturating_add(30_000)
    );
}

fn to_encrypted_pem(body: &str) -> String {
    let mut out = String::with_capacity(body.len() + body.len() / 64 + 64);
    out.push_str("-----BEGIN ENCRYPTED PRIVATE KEY-----\n");
    for chunk in body.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).expect("valid base64 chunk"));
        out.push('\n');
    }
    out.push_str("-----END ENCRYPTED PRIVATE KEY-----\n");
    out
}

#[test]
fn encrypted_pkcs8_with_passphrase_parses() {
    const PASSPHRASE: &str = "test-pass";
    let mut rng = thread_rng();
    let rsa = RsaPrivateKey::new(&mut rng, 2048).expect("keygen");
    let encrypted = rsa
        .to_pkcs8_encrypted_der(&mut rng, PASSPHRASE)
        .expect("encrypt");
    let pem_body = STANDARD.encode(encrypted.as_bytes());
    let pem = to_encrypted_pem(&pem_body);

    let cfg = Config::from_values(
        "user",
        None,
        "acct",
        "https://example",
        None,
        Some(pem),
        None,
        Some(PASSPHRASE.to_string()),
        None,
        Some(60),
    );

    generate_assertion(&cfg).expect("should generate assertion with encrypted key");
}

#[test]
fn correctly_generates_fingerprint() {
    let b64 = "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2RmwUycPmCSycr6WgS/NXcffCs6U025B+rT2zQDl1UWeKcSIh1TSdh7aHTyMuDaWcu3u+3+93L443D2nXJntZvcg8JV08a/QN+bI3RGdVabGL74ewqn3fuGleWYsIz3oLhse6zwbrhLGdVsD3ADOIl/nAmjOnalyuJ0fUjPgxLwRACEV5WIchVqrkG3wxRJCsj+ze8HrFMMsZ2rEtZb5XwoUiw5gbuvFhrU1y6b821Efe/ajI7h+h8qIIXcqTWSFZj93dmqWl8jUU9GkRouSVD8PrHUu0LMRNNsJ/ZC5e0u6mjVc47PyTKTUn+2q0ySoyWLRkyF0SWzqD4WI12gzIQIDAQAB";
    let der = general_purpose::STANDARD.decode(b64).unwrap();
    let pubkey = RsaPublicKey::from_public_key_der(&der).unwrap();
    let fp = "SHA256:xZx8qqibbh7x0CTGVPZNf3z463BMMn7vIoIxSUJQ/Bc=";
    assert_eq!(compute_fingerprint(&pubkey).unwrap(), fp);
}

fn config_with_exp_secs(exp: u64) -> Config {
    Config {
        user: "user".into(),
        login: None,
        account: "acct".into(),
        url: "https://example".into(),
        jwt_token: None,
        private_key: Some(TEST_PKCS8_PRIVKEY_PEM.to_string()),
        private_key_path: None,
        private_key_passphrase: None,
        public_key_fp: None,
        jwt_exp_secs: Some(exp),
        jwt_refresh_margin_secs: None,
        retry_on_unauthorized: None,
    }
}

#[test]
fn clamps_short_expiry_and_warns_once() {
    let cfg = config_with_exp_secs(10);

    let (logs, jwt) = with_captured_logs(|| generate_assertion(&cfg).expect("jwt"));

    let payload = decode_jwt_payload(&jwt);
    let iat = payload.get("iat").and_then(|v| v.as_u64()).unwrap();
    let exp = payload.get("exp").and_then(|v| v.as_u64()).unwrap();

    assert_eq!(
        exp.saturating_sub(iat),
        30_000,
        "short lifetimes should clamp to 30 seconds (milliseconds)"
    );

    let warn_logs: Vec<&String> = logs
        .iter()
        .filter(|line| line.contains("WARN") && line.to_lowercase().contains("clamp"))
        .collect();
    assert_eq!(
        warn_logs.len(),
        1,
        "expected a single clamp warning, got: {:?}",
        warn_logs
    );
}

#[tokio::test]
async fn refreshes_token_when_near_expiry() {
    let cfg = config_with_exp_secs(60);
    let mut ctx = JwtContext::new(&cfg, 30).expect("context");

    // First call should produce a token we can reuse until margin threshold hit.
    let first = ctx.ensure_valid(&cfg).expect("first token");

    // Simulate time passage so remaining TTL drops below margin.
    ctx.force_issued_at(super::now_millis().unwrap().saturating_sub(40_000));

    let (logs, second) = with_captured_logs(|| ctx.ensure_valid(&cfg).expect("refresh token"));

    assert_ne!(
        first, second,
        "token should refresh once below safety margin"
    );
    assert!(
        logs.iter()
            .any(|line| line.contains("INFO") && line.to_lowercase().contains("refresh")),
        "refresh should emit informative log; got {:?}",
        logs
    );
}
