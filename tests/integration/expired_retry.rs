use std::fs;
use std::path::PathBuf;

use jiff::Zoned;
use serde::Serialize;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use snowpipe_streaming::{Config, StreamingIngestClient};

#[derive(Serialize, Clone)]
struct RowType {
    id: u64,
    data: String,
    dt: Zoned,
}

#[tokio::test]
async fn retries_once_on_expired_token_then_succeeds() {
    let server = MockServer::start().await;

    // Discovery
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;

    // Stub channel open to succeed
    let open_resp = include_str!("../tests/fixtures/open_channel_response.json");
    Mock::given(method("PUT"))
        .and(path(
            "/v2/streaming/databases/db/schemas/schema/pipes/pipe/channels/ch",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(open_resp))
        .mount(&server)
        .await;

    // First append rows returns 401 expired; second succeeds
    let rows_path = "/v2/streaming/data/databases/db/schemas/schema/pipes/pipe/channels/ch/rows";
    let expired = ResponseTemplate::new(401).set_body_string("token expired");
    Mock::given(method("POST")).and(path(rows_path)).respond_with(expired).mount(&server).await;
    let ok_resp = include_str!("../tests/fixtures/append_rows_response.json");
    Mock::given(method("POST"))
        .and(path(rows_path))
        .respond_with(ResponseTemplate::new(200).set_body_string(ok_resp))
        .mount(&server)
        .await;

    // Status for close
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

    // Config with pre-supplied jwt (header scheme should be Snowflake JWT in final design)
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "jwt"
    });
    let mut cfg_path = PathBuf::from("target");
    fs::create_dir_all(&cfg_path).ok();
    cfg_path.push("integration-expired-retry.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let client = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        Config::from_file(&cfg_path).expect("cfg file"),
    )
    .await
    .expect("client new failed");

    // Expected behavior: first POST 401 triggers refresh and retry → success
    let mut ch = client.open_channel("ch").await.expect("open channel");
    let res = ch
        .append_row(&RowType {
            id: 1,
            data: "x".to_string(),
            dt: Zoned::now(),
        })
        .await;

    assert!(res.is_ok(), "should refresh on expired token and succeed on retry");
}

