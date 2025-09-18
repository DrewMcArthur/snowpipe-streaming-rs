use crate::StreamingIngestClient;
use crate::tests::test_support::{base_config, capture_logs, drain_logs};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

#[tokio::test]
async fn retries_once_after_401_then_succeeds() {
    let server = MockServer::start().await;

    let server_uri = server.uri();
    let tokens: Arc<Mutex<(Option<String>, Option<String>)>> = Arc::new(Mutex::new((None, None)));
    let tokens_clone = tokens.clone();

    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(move |req: &Request| {
            let auth = req
                .headers
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .expect("JWT header missing");

            let mut guard = tokens_clone.lock().unwrap();
            if guard.0.is_none() {
                guard.0 = Some(auth);
                std::thread::sleep(Duration::from_millis(1100));
                ResponseTemplate::new(401)
            } else {
                guard.1 = Some(auth);
                ResponseTemplate::new(200).set_body_string(server_uri.clone())
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
    let res = StreamingIngestClient::<Row>::new(
        "client",
        "db",
        "schema",
        "pipe",
        base_config(&server.uri()),
    )
    .await;
    drop(guard);

    res.expect("client creation should succeed after retry");

    let logs = drain_logs(lines);
    assert!(
        logs.iter()
            .any(|line| line.contains("WARN") && line.contains("401")),
        "expected warning log mentioning 401, got: {:?}",
        logs
    );

    let guard = tokens.lock().unwrap();
    let original = guard.0.clone().expect("first token recorded");
    let refreshed = guard.1.clone().expect("second token recorded");
    assert_ne!(
        original, refreshed,
        "expected refreshed JWT to differ from original"
    );
}
