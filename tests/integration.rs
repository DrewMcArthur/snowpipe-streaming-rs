use std::fs;
use std::path::PathBuf;

use jiff::Zoned;
use serde::Serialize;
use std::sync::Once;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use snowpipe_streaming::{ConfigLocation, StreamingIngestClient};

#[derive(Serialize, Clone)]
struct RowType {
    id: u64,
    data: String,
    dt: Zoned,
}

#[tokio::test]
async fn discovery_and_token_flow() {
    init_logging();
    let server = MockServer::start().await;

    // Control-plane: discovery returns ingest base URL (absolute URI)
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;

    // Control-plane: scoped token
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;

    // Write per-test config file to avoid global env races
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "jwt"
    });
    let mut cfg_path = PathBuf::from("target");
    cfg_path.push(format!("test-config-{}.json", server.address().port()));
    fs::create_dir_all("target").ok();
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        ConfigLocation::File(cfg_path.to_string_lossy().to_string()),
    )
    .await
    .expect("client new failed");

    assert_eq!(client.ingest_host.as_deref(), Some(server.uri().as_str()));
    assert_eq!(client.scoped_token.as_deref(), Some("scoped-token"));
}

#[tokio::test]
async fn oauth2_token_flow_generates_control_token() {
    init_logging();
    let server = MockServer::start().await;

    // OAuth2 token endpoint returns an access token
    let token_resp = serde_json::json!({
        "access_token": "cp-token",
        "token_type": "Bearer",
        "expires_in": 3600
    })
    .to_string();
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(token_resp))
        .mount(&server)
        .await;

    // Ingest discovery and scoped token
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;

    // Config file without jwt_token, with private key
    // Use a test-only passthrough to bypass signing complexities
    let pem = "TEST://assertion:dummy-assertion";
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "private_key": pem,
        "jwt_exp_secs": 60
    });
    let mut cfg_path = PathBuf::from("target");
    cfg_path.push(format!(
        "test-config-oauth2-{}.json",
        server.address().port()
    ));
    fs::create_dir_all("target").ok();
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        ConfigLocation::File(cfg_path.to_string_lossy().to_string()),
    )
    .await
    .expect("client new failed");

    assert_eq!(client.ingest_host.as_deref(), Some(server.uri().as_str()));
    assert_eq!(client.scoped_token.as_deref(), Some("scoped-token"));
}

#[tokio::test]
async fn open_append_status_close_flow() {
    init_logging();
    let server = MockServer::start().await;

    // Control-plane endpoints
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;

    // Ingest-plane endpoints (same base as discovery for tests)
    let open_resp = include_str!("fixtures/open_channel_response.json");
    Mock::given(method("PUT"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(open_resp))
        .mount(&server)
        .await;

    let append_resp = include_str!("fixtures/append_rows_response.json");
    Mock::given(method("POST"))
        .and(path(
            "/v2/streaming/data/databases/db/schemas/schema/pipes/pipe/channels/ch/rows",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(append_resp))
        .mount(&server)
        .await;

    let status_resp = include_str!("fixtures/channel_status_response.json");
    Mock::given(method("POST"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe:bulk-channel-status",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(status_resp))
        .mount(&server)
        .await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Write per-test config file
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "jwt"
    });
    let mut cfg_path = PathBuf::from("target");
    cfg_path.push(format!("test-config-{}.json", server.address().port()));
    fs::create_dir_all("target").ok();
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        ConfigLocation::File(cfg_path.to_string_lossy().to_string()),
    )
    .await
    .expect("client new failed");

    // Open channel and append a row
    let mut ch = client
        .open_channel("ch")
        .await
        .expect("open channel failed (expected to fail before URL fix)");

    ch.append_row(&RowType {
        id: 1,
        data: "x".to_string(),
        dt: Zoned::now(),
    })
    .await
    .expect("append row failed (expected to fail before URL fix)");

    // Ensure close succeeds
    ch.close()
        .await
        .expect("close failed (expected to fail before URL fix)");
}

#[tokio::test]
async fn batched_append_rows_triggers_multiple_posts() {
    init_logging();
    let server = MockServer::start().await;

    // Control-plane endpoints
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;

    // Open channel
    let open_resp = include_str!("fixtures/open_channel_response.json");
    Mock::given(method("PUT"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(open_resp))
        .mount(&server)
        .await;

    // Setup POST rows handler (count verified after via received_requests)
    let append_resp = include_str!("fixtures/append_rows_response.json");
    let rows_path = "/v2/streaming/data/databases/db/schemas/schema/pipes/pipe/channels/ch/rows";
    Mock::given(method("POST"))
        .and(path(rows_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(append_resp))
        .mount(&server)
        .await;

    // Channel status returns a high committed token so close can finish
    let status_resp_high = serde_json::json!({
        "channel_statuses": { "ch": {
            "database_name": "db",
            "schema_name": "schema",
            "pipe_name": "pipe",
            "channel_name": "ch",
            "channel_status_code": "OPEN",
            "last_committed_offset_token": "100000",
            "created_on_ms": 0
        }}}
    )
    .to_string();
    Mock::given(method("POST"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe:bulk-channel-status",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(status_resp_high))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Config file
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "jwt"
    });
    let mut cfg_path = PathBuf::from("target");
    cfg_path.push(format!("test-config-{}.json", server.address().port()));
    fs::create_dir_all("target").ok();
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        ConfigLocation::File(cfg_path.to_string_lossy().to_string()),
    )
    .await
    .expect("client new failed");

    let mut ch = client.open_channel("ch").await.expect("open channel");

    // Create rows large enough that multiple rows combined exceed threshold, forcing multiple requests
    // Use many small rows to ensure chunking splits into multiple bodies
    // Create rows ~600KB each to force small chunk sizes
    let row = RowType {
        id: 0,
        data: "x".repeat(600_000),
        dt: Zoned::now(),
    };
    let mut iter = (0..50u64).map(|i| RowType {
        id: i,
        ..row.clone()
    });
    let _ = ch.append_rows(&mut iter).await.expect("append_rows");

    // Now close should complete because status returns large committed token
    ch.close().await.expect("close");

    // Verify at least 2 POSTs to rows endpoint
    let reqs = server.received_requests().await.unwrap_or_default();
    let rows_posts = reqs
        .iter()
        .filter(|r| r.url.path() == rows_path && format!("{:?}", r.method) == "POST")
        .count();
    assert!(
        rows_posts >= 2,
        "expected >=2 rows posts, got {}",
        rows_posts
    );
}

#[tokio::test]
async fn append_rows_error_is_mapped() {
    init_logging();
    let server = MockServer::start().await;
    // Control-plane
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;
    // Open
    let open_resp = include_str!("fixtures/open_channel_response.json");
    Mock::given(method("PUT"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(open_resp))
        .mount(&server)
        .await;
    // Rows error
    let rows_path = "/v2/streaming/data/databases/db/schemas/schema/pipes/pipe/channels/ch/rows";
    Mock::given(method("POST"))
        .and(path(rows_path))
        .respond_with(ResponseTemplate::new(413))
        .mount(&server)
        .await;

    // Config file
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "jwt"
    });
    let mut cfg_path = PathBuf::from("target");
    cfg_path.push(format!("test-config-{}.json", server.address().port()));
    fs::create_dir_all("target").ok();
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        ConfigLocation::File(cfg_path.to_string_lossy().to_string()),
    )
    .await
    .expect("client new failed");
    let mut ch = client.open_channel("ch").await.expect("open channel");

    let err = ch
        .append_row(&RowType {
            id: 1,
            data: "x".into(),
            dt: Zoned::now(),
        })
        .await
        .expect_err("expected error");
    match err {
        snowpipe_streaming::Error::Http(_) => {}
        other => panic!("unexpected error: {:?}", other),
    }
}

#[tokio::test]
async fn data_too_large_is_returned() {
    init_logging();
    let server = MockServer::start().await;
    // Control-plane
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;
    // Open
    let open_resp = include_str!("fixtures/open_channel_response.json");
    Mock::given(method("PUT"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(open_resp))
        .mount(&server)
        .await;

    // Config file
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "jwt"
    });
    let mut cfg_path = PathBuf::from("target");
    cfg_path.push(format!("test-config-{}.json", server.address().port()));
    fs::create_dir_all("target").ok();
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        ConfigLocation::File(cfg_path.to_string_lossy().to_string()),
    )
    .await
    .expect("client new failed");
    let mut ch = client.open_channel("ch").await.expect("open channel");

    // Create an oversized row that exceeds MAX_REQUEST_SIZE after serialization
    let big = RowType {
        id: 1,
        data: "a".repeat(16 * 1024 * 1024),
        dt: Zoned::now(),
    };
    let err = ch.append_row(&big).await.expect_err("expected error");
    match err {
        snowpipe_streaming::Error::DataTooLarge(actual, max) => {
            assert!(actual > max);
        }
        other => panic!("unexpected error: {:?}", other),
    }
}

#[tokio::test]
async fn chunk_size_guard_two_large_rows_yield_two_posts() {
    init_logging();
    let server = MockServer::start().await;
    // Control-plane
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;
    // Open
    let open_resp = include_str!("fixtures/open_channel_response.json");
    Mock::given(method("PUT"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(open_resp))
        .mount(&server)
        .await;
    // Rows: respond OK; we will count POSTs later
    let rows_path = "/v2/streaming/data/databases/db/schemas/schema/pipes/pipe/channels/ch/rows";
    let append_resp = include_str!("fixtures/append_rows_response.json");
    Mock::given(method("POST"))
        .and(path(rows_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(append_resp))
        .mount(&server)
        .await;
    // Status high commit
    let status_resp_high = serde_json::json!({
        "channel_statuses": { "ch": {
            "database_name": "db",
            "schema_name": "schema",
            "pipe_name": "pipe",
            "channel_name": "ch",
            "channel_status_code": "OPEN",
            "last_committed_offset_token": "100000",
            "created_on_ms": 0
        }}}
    )
    .to_string();
    Mock::given(method("POST"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe:bulk-channel-status",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(status_resp_high))
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Config
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "jwt"
    });
    let mut cfg_path = PathBuf::from("target");
    cfg_path.push(format!("test-config-{}.json", server.address().port()));
    fs::create_dir_all("target").ok();
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        ConfigLocation::File(cfg_path.to_string_lossy().to_string()),
    )
    .await
    .expect("client new failed");
    let mut ch = client.open_channel("ch").await.expect("open channel");

    // Two very large rows (~9MB) → chunk size clamps to 1 → exactly 2 posts
    let big = RowType {
        id: 0,
        data: "x".repeat(9_000_000),
        dt: Zoned::now(),
    };
    let mut iter = vec![
        RowType {
            id: 1,
            ..big.clone()
        },
        RowType { id: 2, ..big },
    ]
    .into_iter();
    let _ = ch.append_rows(&mut iter).await.expect("append_rows");
    ch.close().await.expect("close");

    let reqs = server.received_requests().await.unwrap_or_default();
    let rows_posts = reqs
        .iter()
        .filter(|r| r.url.path() == rows_path && format!("{:?}", r.method) == "POST")
        .count();
    assert_eq!(
        rows_posts, 2,
        "expected exactly 2 rows posts, got {}",
        rows_posts
    );
}
static INIT: Once = Once::new();
fn init_logging() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    });
}
