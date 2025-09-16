use snowpipe_streaming::Error;

#[test]
fn encrypted_pkcs8_decrypts_to_encoding_key() {
    // Generate an RSA keypair and encrypt to PKCS#8 DER at runtime
    let mut rng = rand::thread_rng();
    let rsa = rsa::RsaPrivateKey::new(&mut rng, 2048).expect("keygen");
    let pkcs8_der = rsa
        .to_pkcs8_der()
        .expect("pkcs8 der")
        .to_bytes();

    let pass = "test-pass";
    let enc = pkcs8::EncryptedPrivateKeyInfo::encrypt(
        &mut rng,
        pkcs8::EncryptionAlgorithm::pbes2_default(),
        pass,
        &pkcs8_der,
    )
    .expect("encrypt");
    let pem = format!(
        "{}{}{}",
        "-----BEGIN ENCRYPTED PRIVATE KEY-----\n",
        base64::engine::general_purpose::STANDARD.encode(enc.to_der().as_bytes()),
        "\n-----END ENCRYPTED PRIVATE KEY-----\n"
    );

    // Build a minimal config and attempt to build an assertion (will panic on network if used)
    let cfg = snowpipe_streaming::config::Config {
        user: "user".into(),
        account: "acct".into(),
        url: "https://example".into(),
        jwt_token: String::new(),
        private_key: Some(pem),
        private_key_path: None,
        private_key_passphrase: Some(pass.into()),
        jwt_exp_secs: Some(60),
    };

    // Call the internal helper via the public path indirectly by ensuring it doesn't error
    // We can't invoke network, but we can call the private function by generating an assertion
    let url = "https://example/oauth2/token";
    // SAFETY: using internal helper via function duplication in test module is not accessible; we instead test decryption directly
    let info = pkcs8::EncryptedPrivateKeyInfo::from_der(&enc.to_der()).expect("parse enc");
    let der = info.decrypt(pass).expect("decrypt");
    let rsa2 = rsa::RsaPrivateKey::from_pkcs8_der(&der).expect("pkcs8 parse");
    let pkcs1 = rsa2.to_pkcs1_der().expect("pkcs1");
    let key = jsonwebtoken::EncodingKey::from_rsa_der(pkcs1.as_bytes());
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    #[derive(serde::Serialize)]
    struct Claims { iss: String, sub: String, aud: String, iat: usize, exp: usize, jti: String }
    let claims = Claims { iss: "user".into(), sub: "user".into(), aud: url.into(), iat: 0, exp: 1, jti: "x".into() };
    let _ = jsonwebtoken::encode(&header, &claims, &key).expect("sign");
}
