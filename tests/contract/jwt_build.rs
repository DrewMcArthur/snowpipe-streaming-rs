use snowpipe_streaming::{Config, StreamingIngestClient};

const ENCRYPTED_PRIVATE_KEY: &str = include_str!("../fixtures/id_rsa_encrypted.pem");

#[tokio::test]
async fn encrypted_pem_without_passphrase_is_error() {
    // Missing passphrase should cause key error before any network IO
    let cfg = Config::from_values(
        "user",
        None,
        "acct",
        "https://example",
        None,
        Some(ENCRYPTED_PRIVATE_KEY.to_string()),
        None,
        None,
        None,
        Some(60),
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
        "user",
        None,
        "acct",
        "https://example",
        None,
        Some(pem.into()),
        None,
        None,
        None,
        Some(60),
    );
    let res = StreamingIngestClient::<serde_json::Value>::new(
        "c", "db", "schema", "pipe", cfg,
    )
    .await;
    assert!(res.is_err(), "expected error for malformed key");
}

