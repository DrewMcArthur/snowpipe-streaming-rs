use base64::Engine;
use pkcs8::EncodePrivateKey;
use rand::thread_rng;
use rsa::RsaPrivateKey;
use snowpipe_streaming::{Config, StreamingIngestClient};

const PASSPHRASE: &str = "test-pass";

fn generate_encrypted_pem() -> String {
    let mut rng = thread_rng();
    let rsa = RsaPrivateKey::new(&mut rng, 2048).expect("keygen");
    let encrypted = rsa
        .to_pkcs8_encrypted_der(&mut rng, PASSPHRASE)
        .expect("encrypt to PKCS#8");
    let body = base64::engine::general_purpose::STANDARD.encode(encrypted.as_bytes());
    let mut out = String::with_capacity(body.len() + body.len() / 64 + 64);
    out.push_str("-----BEGIN ENCRYPTED PRIVATE KEY-----\n");
    for chunk in body.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).expect("base64 chunk"));
        out.push('\n');
    }
    out.push_str("-----END ENCRYPTED PRIVATE KEY-----\n");
    out
}

#[tokio::test]
async fn encrypted_pkcs8_with_passphrase_parses() {
    let pem = generate_encrypted_pem();

    let cfg = Config::from_values(
        "user",
        None,
        "acct",
        "https://example", // discovery will fail if executed; test only asserts no key error
        None,
        Some(pem),
        None,
        Some(PASSPHRASE.to_string()),
        Some(60),
    );

    let res = StreamingIngestClient::<serde_json::Value>::new(
        "c", "db", "schema", "pipe", cfg,
    )
    .await;

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
