use base64::Engine;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use pem::parse;
use pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rand::thread_rng;
use rsa::RsaPrivateKey;
use rsa::pkcs1::EncodeRsaPrivateKey;
use snowpipe_streaming::Config;

const PASSPHRASE: &str = "test-pass";

fn to_pem(body: &str) -> String {
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
fn encrypted_pkcs8_decrypts_to_encoding_key() {
    let mut rng = thread_rng();
    let rsa = RsaPrivateKey::new(&mut rng, 2048).expect("keygen");
    let encrypted_der = rsa
        .to_pkcs8_encrypted_der(&mut rng, PASSPHRASE)
        .expect("encrypt to PKCS#8");
    let pem_body = base64::engine::general_purpose::STANDARD.encode(encrypted_der.as_bytes());
    let pem = to_pem(&pem_body);

    let cfg = Config::from_values(
        "user",
        None,
        "acct",
        "https://example",
        None,
        Some(pem.clone()),
        None,
        Some(PASSPHRASE.to_string()),
        Some(60),
    );

    let raw_key = cfg
        .private_key()
        .expect("inline encrypted key should be accessible");
    assert_eq!(raw_key, pem, "inline key should match generated PEM");

    let block = parse(&raw_key).expect("parse PEM");
    let decrypted = RsaPrivateKey::from_pkcs8_encrypted_der(block.contents(), PASSPHRASE)
        .expect("decrypt with provided passphrase");
    let pkcs1 = decrypted
        .to_pkcs1_der()
        .expect("serialize RSA key to PKCS#1 DER");
    let encoding_key = EncodingKey::from_rsa_der(pkcs1.as_bytes());

    #[derive(serde::Serialize)]
    struct Claims {
        iss: String,
        sub: String,
        iat: usize,
        exp: usize,
    }

    let header = Header::new(Algorithm::RS256);
    let claims = Claims {
        iss: "user".into(),
        sub: "user".into(),
        iat: 0,
        exp: 60,
    };

    jsonwebtoken::encode(&header, &claims, &encoding_key).expect("signing should succeed");
}
