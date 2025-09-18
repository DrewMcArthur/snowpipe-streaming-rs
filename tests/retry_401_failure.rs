use snowpipe_streaming::{Config, Error, StreamingIngestClient};
use std::sync::{Arc, Mutex};
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

    let res =
        StreamingIngestClient::<Row>::new("client", "db", "schema", "pipe", config(&server)).await;
    drop(guard);

    match res {
        Err(Error::Auth(msg)) => {
            assert!(msg.to_lowercase().contains("401"));
        }
        Err(other) => panic!("expected Error::Auth, got {}", other),
        Ok(_) => panic!("expected Error::Auth, got Ok"),
    }

    let logs = lines.lock().unwrap().clone();
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
