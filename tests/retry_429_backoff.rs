use snowpipe_streaming::{Config, StreamingIngestClient};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::subscriber::{DefaultGuard, set_default};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Registry, fmt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const PRIVATE_KEY: &str = include_str!("fixtures/id_rsa.pem");

struct VecWriter {
    lines: Arc<Mutex<Vec<String>>>,
}

impl std::io::Write for VecWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut guard = self.lines.lock().unwrap();
        guard.push(String::from_utf8_lossy(buf).into_owned());
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn capture_logs() -> (Arc<Mutex<Vec<String>>>, DefaultGuard) {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let writer_lines = lines.clone();
    let subscriber = Registry::default().with(
        fmt::Layer::default()
            .with_writer(move || VecWriter {
                lines: writer_lines.clone(),
            })
            .with_target(false)
            .with_level(true)
            .with_ansi(false),
    );
    let guard = set_default(subscriber);
    (lines, guard)
}

fn config(server: &MockServer) -> Config {
    Config::from_values(
        "user",
        None,
        "acct",
        server.uri(),
        None,
        Some(PRIVATE_KEY.to_string()),
        None,
        None,
        None,
        Some(120),
    )
}

#[tokio::test]
async fn waits_two_seconds_before_retrying_429() {
    tokio::time::pause();

    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(429))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .expect(1)
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

    let handle: JoinHandle<_> = tokio::spawn({
        let cfg = config(&server);
        async move { StreamingIngestClient::<Row>::new("client", "db", "schema", "pipe", cfg).await }
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

    let logs = lines.lock().unwrap().clone();
    assert!(
        logs.iter()
            .any(|line| line.contains("WARN") && line.contains("429")),
        "expected warning log mentioning 429, got {:?}",
        logs
    );
}
