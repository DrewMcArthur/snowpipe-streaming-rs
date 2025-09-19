use crate::StreamingIngestClient;
use crate::tests::test_support::{base_config, capture_logs, drain_logs};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

#[tokio::test]
async fn waits_two_seconds_before_retrying_429() {
    tokio::time::pause();

    let server = MockServer::start().await;
    let success_body = server.uri();
    let first_call = Arc::new(Mutex::new(true));
    let first_call_clone = first_call.clone();

    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(move |_req: &Request| {
            let mut first_call = first_call_clone.lock().unwrap();
            if *first_call {
                *first_call = false;
                ResponseTemplate::new(429)
            } else {
                ResponseTemplate::new(200).set_body_string(success_body.clone())
            }
        })
        .expect(2)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .expect(1)
        .mount(&server)
        .await;

    #[derive(serde::Serialize, Clone)]
    struct Row;

    let (lines, guard) = capture_logs();

    let server_uri = server.uri();
    let handle: JoinHandle<_> = tokio::spawn(async move {
        StreamingIngestClient::<Row>::new(
            "client",
            "db",
            "schema",
            "pipe",
            base_config(&server_uri),
        )
        .await
    });

    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(1)).await;
    assert!(
        !handle.is_finished(),
        "client construction should still be waiting before full 2s advance"
    );

    tokio::time::advance(Duration::from_secs(1)).await;
    let res = handle.await.unwrap();
    tokio::time::resume();
    drop(guard);

    res.expect("client construction should succeed after retry");

    let logs = drain_logs(lines);
    assert!(
        logs.iter()
            .any(|line| line.contains("WARN") && line.contains("429")),
        "expected warning log mentioning 429, got {:?}",
        logs
    );
}
