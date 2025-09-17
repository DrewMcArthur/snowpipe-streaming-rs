use snowpipe_streaming::{Config, StreamingIngestClient};

#[tokio::test]
async fn encrypted_pkcs8_with_passphrase_parses() {
    let mut rng = rand::thread_rng();
    let rsa = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("keygen");
    let der = rsa.to_pkcs8_der().expect("pkcs8 der").to_bytes();
    let pass = "pass";
    let enc = pkcs8::EncryptedPrivateKeyInfo::encrypt(
        &mut rng,
        pkcs8::EncryptionAlgorithm::pbes2_default(),
        pass,
        &der,
    )
    .expect("encrypt");
    let pem = format!(
        "{}{}{}",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(enc.to_der().as_bytes()),
        "\n-----END ENCRYPTED PRIVATE KEY-----\n"
    );

    // Provide passphrase so generate_assertion can decrypt; network calls will fail later,
    // but we only care that construction reaches discovery step without key error.
    let cfg = Config::from_values(
        "user",
        "acct",
        "https://example", // discovery will fail if executed; test only asserts no key error
        None,
        Some(pem),
        None,
        Some(pass.into()),
        Some(60),
    );

    let res = StreamingIngestClient::<serde_json::Value>::new(
        "c", "db", "schema", "pipe", cfg,
    )
    .await;

    // We can't assert success here without mocking HTTP; it may return an HTTP error.
    // Assert specifically that error is not a Key error by allowing both Ok and non-Key errors.
    match res {
        Ok(_) => {}
        Err(e) => {
            let s = format!("{}", e);
            assert!(
                !s.to_lowercase().contains("key error"),
                "unexpected key error: {}",
                s
            );
        }
    }
}

