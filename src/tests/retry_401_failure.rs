use crate::tests::test_support::{base_config, capture_logs, drain_logs};
use crate::{Error, StreamingIngestClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn returns_auth_error_after_double_401() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(401))
        .expect(2)
        .mount(&server)
        .await;

    let (lines, guard) = capture_logs();
    #[derive(serde::Serialize, Clone)]
    struct Row;

    let res = StreamingIngestClient::<Row>::new(
        "client",
        "db",
        "schema",
        "pipe",
        base_config(&server.uri()),
    )
    .await;
    drop(guard);

    match res {
        Err(Error::Auth(msg)) => {
            assert!(msg.to_lowercase().contains("401"));
        }
        Err(other) => panic!("expected Error::Auth, got {}", other),
        Ok(_) => panic!("expected Error::Auth, got Ok"),
    }

    let logs = drain_logs(lines);
    let warn_count = logs
        .iter()
        .filter(|line| line.contains("WARN") && line.contains("401"))
        .count();
    assert_eq!(
        warn_count, 2,
        "should log a warning for each 401, got {:?}",
        logs
    );
}
