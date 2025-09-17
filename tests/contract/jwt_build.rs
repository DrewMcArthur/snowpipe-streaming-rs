use snowpipe_streaming::{Config, StreamingIngestClient};

#[tokio::test]
async fn encrypted_pem_without_passphrase_is_error() {
    // Build an encrypted PKCS#8 PEM at runtime
    let mut rng = rand::thread_rng();
    let rsa = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("keygen");
    let pkcs8_der = rsa
        .to_pkcs8_der()
        .expect("pkcs8 der")
        .to_bytes();
    let enc = pkcs8::EncryptedPrivateKeyInfo::encrypt(
        &mut rng,
        pkcs8::EncryptionAlgorithm::pbes2_default(),
        "test-pass",
        &pkcs8_der,
    )
    .expect("encrypt");
    let pem = format!(
        "{}{}{}",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(enc.to_der().as_bytes()),
        "\n-----END ENCRYPTED PRIVATE KEY-----\n"
    );

    // Missing passphrase should cause key error before any network IO
    let cfg = Config::from_values(
        "user", "acct", "https://example", None, Some(pem), None, None, Some(60),
    );
    let res = StreamingIngestClient::<serde_json::Value>::new(
        "c", "db", "schema", "pipe", cfg,
    )
    .await;
    assert!(res.is_err(), "expected error for encrypted PEM without passphrase");
}

#[tokio::test]
async fn malformed_key_is_error() {
    let pem = "-----BEGIN PRIVATE KEY-----\nMALFORMED\n-----END PRIVATE KEY-----\n";
    let cfg = Config::from_values(
        "user", "acct", "https://example", None, Some(pem.into()), None, None, Some(60),
    );
    let res = StreamingIngestClient::<serde_json::Value>::new(
        "c", "db", "schema", "pipe", cfg,
    )
    .await;
    assert!(res.is_err(), "expected error for malformed key");
}

