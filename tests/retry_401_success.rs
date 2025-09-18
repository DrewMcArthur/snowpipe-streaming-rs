use snowpipe_streaming::{Config, StreamingIngestClient};
use std::sync::{Arc, Mutex};
use tracing::subscriber::{DefaultGuard, set_default};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{Registry, fmt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

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
async fn retries_once_after_401_then_succeeds() {
    let server = MockServer::start().await;

    let server_uri = server.uri();
    let first_token = Arc::new(Mutex::new(None::<String>));
    let first_token_clone = first_token.clone();

    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(move |req: &Request| {
            let auth = req
                .headers
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
                .expect("JWT header missing");

            let mut guard = first_token_clone.lock().unwrap();
            if guard.is_none() {
                *guard = Some(auth);
                ResponseTemplate::new(401)
            } else {
                let previous = guard.as_ref().unwrap();
                assert_ne!(
                    previous, &auth,
                    "expected refreshed JWT to differ from original"
                );
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
    let res =
        StreamingIngestClient::<Row>::new("client", "db", "schema", "pipe", config(&server)).await;
    drop(guard);

    res.expect("client creation should succeed after retry");

    let logs = lines.lock().unwrap().clone();
    assert!(
        logs.iter()
            .any(|line| line.contains("WARN") && line.contains("401")),
        "expected warning log mentioning 401, got: {:?}",
        logs
    );

    let original = first_token
        .lock()
        .unwrap()
        .clone()
        .expect("first token recorded");
    assert!(
        original.starts_with("Bearer"),
        "expected bearer token header payload"
    );
}
