use snowpipe_streaming::{Config, StreamingIngestClient};

#[tokio::test]
async fn invalid_control_host_url_fails_fast() {
    let cfg = Config::from_values(
        "user",
        None,
        "acct",
        "://not-a-valid-url",
        Some("jwt".into()),
        None,
        None,
        None,
        None,
        Some(60),
    );

    let err = match StreamingIngestClient::<()>::new("c", "db", "schema", "pipe", cfg).await {
        Ok(_) => panic!("expected invalid URL error"),
        Err(err) => err,
    };

    match err {
        snowpipe_streaming::Error::Config(msg) => {
            assert!(msg.contains("Invalid control host URL"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}
