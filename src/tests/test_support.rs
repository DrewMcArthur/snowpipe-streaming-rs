use crate::Config;
use std::sync::{Arc, Mutex};
use tracing::subscriber::{set_default, DefaultGuard};
use tracing_subscriber::{fmt, layer::SubscriberExt, Registry};

pub const FIXTURE_PRIVATE_KEY: &str = include_str!("../../tests/fixtures/id_rsa.pem");

pub fn base_config(server_uri: &str) -> Config {
    Config::from_values(
        "user",
        None,
        "acct",
        server_uri,
        None,
        Some(FIXTURE_PRIVATE_KEY.to_string()),
        None,
        None,
        None,
        Some(120),
    )
}

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

fn make_subscriber(lines: Arc<Mutex<Vec<String>>>) -> impl tracing::Subscriber + Send + Sync {
    let writer_lines = lines.clone();
    Registry::default().with(
        fmt::Layer::default()
            .with_writer(move || VecWriter {
                lines: writer_lines.clone(),
            })
            .with_target(false)
            .with_level(true)
            .with_ansi(false),
    )
}

pub fn capture_logs() -> (Arc<Mutex<Vec<String>>>, DefaultGuard) {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let guard = set_default(make_subscriber(lines.clone()));
    (lines, guard)
}

pub fn drain_logs(lines: Arc<Mutex<Vec<String>>>) -> Vec<String> {
    Arc::try_unwrap(lines).unwrap().into_inner().unwrap()
}

pub fn with_captured_logs<F, T>(f: F) -> (Vec<String>, T)
where
    F: FnOnce() -> T,
{
    let lines = Arc::new(Mutex::new(Vec::new()));
    let subscriber = make_subscriber(lines.clone());
    let result = tracing::subscriber::with_default(subscriber, f);
    let logs = Arc::try_unwrap(lines).unwrap().into_inner().unwrap();
    (logs, result)
}
