use std::fs;
use std::path::PathBuf;

use jiff::Zoned;
use serde::Serialize;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use snowpipe_streaming::{Config, StreamingIngestClient};

#[derive(Serialize, Clone)]
struct RowType {
    id: u64,
    data: String,
    dt: Zoned,
}

#[tokio::test]
async fn client_must_not_call_oauth2_token_when_private_key_provided() {
    // When only a private key is provided, the client must generate JWT locally
    // and must NOT call /oauth2/token (which is 404 in real Snowflake for keypair auth).
    let server = MockServer::start().await;

    // If the client incorrectly calls /oauth2/token, this mock would match.
    // Expect 0 invocations.
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(404))
        .expect(0)
        .mount(&server)
        .await;

    // Discovery remains available.
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;

    // Scoped token remains available for now (subject to redesign later).
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("scoped-token"))
        .mount(&server)
        .await;

    // Config file without jwt_token, with private key (test assertion shortcut)
    let pem = "TEST://assertion:dummy-assertion";
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "private_key": pem,
        "jwt_exp_secs": 60
    });
    let mut cfg_path = PathBuf::from("target");
    fs::create_dir_all(&cfg_path).ok();
    cfg_path.push("contract-client-auth.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let res = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        Config::from_file(&cfg_path).expect("cfg file"),
    )
    .await;

    // Under correct behavior, client creation should succeed without calling /oauth2/token.
    // Current implementation calls /oauth2/token and should therefore fail this assertion.
    assert!(res.is_ok(), "client should initialize without /oauth2/token calls");
}

#[tokio::test]
async fn discovery_uses_bearer_with_keypair_header() {
    let server = MockServer::start().await;

    // Expect Bearer scheme with KEYPAIR_JWT token type header.
    Mock::given(method("GET"))
        .and(path("/v2/streaming/hostname"))
        .and(header("Authorization", "Bearer dummy"))
        .and(header(
            "X-Snowflake-Authorization-Token-Type",
            "KEYPAIR_JWT",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(server.uri()))
        .mount(&server)
        .await;

    // Config with a pre-supplied JWT token; header expectation enforces scheme.
    let cfg = serde_json::json!({
        "user": "user",
        "account": "acct",
        "url": server.uri(),
        "jwt_token": "dummy"
    });
    let mut cfg_path = PathBuf::from("target");
    fs::create_dir_all(&cfg_path).ok();
    cfg_path.push("contract-client-auth-header.json");
    fs::write(&cfg_path, serde_json::to_string(&cfg).unwrap()).unwrap();

    let res = StreamingIngestClient::<RowType>::new(
        "test-client",
        "db",
        "schema",
        "pipe",
        Config::from_file(&cfg_path).expect("cfg file"),
    )
    .await;

    assert!(res.is_ok(), "discovery must use Bearer + KEYPAIR_JWT header scheme");
}
