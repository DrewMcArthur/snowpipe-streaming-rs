use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use snowpipe_streaming::telemetry::refresh::RefreshTelemetry;
use snowpipe_streaming::token::{RefreshPolicy, TokenEnvelope, TokenGuard, TokenGuardConfig, TokenSnapshot};

fn envelope_with_ttl(ttl_secs: u64) -> TokenEnvelope {
    let issued_at = SystemTime::now();
    let expires_at = issued_at + Duration::from_secs(ttl_secs);
    TokenEnvelope::try_new(
        "initial".to_string(),
        issued_at,
        expires_at,
        false,
    )
    .expect("valid envelope")
}

fn policy() -> RefreshPolicy {
    RefreshPolicy::new(
        Duration::from_secs(120),
        Duration::from_secs(300),
        Duration::from_secs(30),
    )
    .expect("policy")
}

fn snapshot(value: &str, ttl_secs: u64) -> TokenSnapshot {
    let issued = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    TokenSnapshot {
        value: value.to_string(),
        issued_at: issued,
        expires_at: issued + ttl_secs,
        scoped: false,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn ensure_token_returns_200_when_fresh() {
    let envelope = envelope_with_ttl(600);
    let guard = TokenGuard::new(
        envelope,
        TokenGuardConfig {
            policy: policy(),
            clock_skew: Duration::from_secs(0),
        },
    );

    let refresh_called = Arc::new(AtomicUsize::new(0));
    let telemetry = RefreshTelemetry::new("contract.token.ensure.fresh");
    let token = guard
        .ensure_fresh(false, {
            let called = refresh_called.clone();
            move || {
                let called = called.clone();
                async move {
                    called.fetch_add(1, Ordering::SeqCst);
                    Ok(snapshot("refreshed", 600))
                }
            }
        }, &telemetry)
        .await
        .expect("should succeed");

    assert_eq!(token.value, "initial");
    assert_eq!(refresh_called.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn ensure_token_returns_202_when_near_expiry() {
    let issued_at = SystemTime::now();
    let expires_at = issued_at + Duration::from_secs(30);
    let envelope = TokenEnvelope::try_new("stale".into(), issued_at, expires_at, false).unwrap();
    let guard = TokenGuard::new(
        envelope,
        TokenGuardConfig {
            policy: policy(),
            clock_skew: Duration::from_secs(0),
        },
    );
    let telemetry = RefreshTelemetry::new("contract.token.ensure.refresh");
    let token = guard
        .ensure_fresh(false, || async { Ok(snapshot("refreshed", 600)) }, &telemetry)
        .await
        .expect("refresh succeeds");
    assert_eq!(token.value, "refreshed");
}

#[tokio::test(flavor = "current_thread")]
async fn ensure_token_returns_503_when_refresh_fails() {
    let issued_at = SystemTime::now();
    let expires_at = issued_at + Duration::from_secs(10);
    let envelope = TokenEnvelope::try_new("stale".into(), issued_at, expires_at, false).unwrap();
    let guard = TokenGuard::new(
        envelope,
        TokenGuardConfig {
            policy: policy(),
            clock_skew: Duration::from_secs(0),
        },
    );
    let telemetry = RefreshTelemetry::new("contract.token.ensure.failure");
    let err = guard
        .ensure_fresh(false, || async {
            Err(snowpipe_streaming::Error::Config(
                "simulated refresh failure".into(),
            ))
        }, &telemetry)
        .await
        .expect_err("should error");

    match err {
        snowpipe_streaming::Error::Config(msg) => {
            assert!(msg.contains("simulated"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}
