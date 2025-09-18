use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use snowpipe_streaming::telemetry::refresh::RefreshTelemetry;
use snowpipe_streaming::token::{RefreshPolicy, TokenEnvelope, TokenGuard, TokenGuardConfig, TokenSnapshot};

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

fn guard_with_ttl(ttl_secs: u64) -> TokenGuard {
    let issued = SystemTime::now();
    let expires = issued + Duration::from_secs(ttl_secs);
    let envelope = TokenEnvelope::try_new("initial".into(), issued, expires, false).unwrap();
    let policy = RefreshPolicy::new(
        Duration::from_secs(120),
        Duration::from_secs(60),
        Duration::from_secs(5),
    )
    .unwrap();
    TokenGuard::new(
        envelope,
        TokenGuardConfig {
            policy,
            clock_skew: Duration::from_secs(0),
        },
    )
}

#[tokio::test(flavor = "current_thread")]
async fn refresh_triggers_before_expiry() {
    let guard = guard_with_ttl(30);
    let refreshes = Arc::new(AtomicUsize::new(0));
    let telemetry = RefreshTelemetry::new("integration.refresh.before_expiry");
    let result = guard
        .ensure_fresh(false, {
            let refreshes = refreshes.clone();
            move || {
                let refreshes = refreshes.clone();
                async move {
                    refreshes.fetch_add(1, Ordering::SeqCst);
                    Ok(snapshot("renewed", 600))
                }
            }
        }, &telemetry)
        .await
        .unwrap();

    assert_eq!(result.value, "renewed");
    assert_eq!(refreshes.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn refresh_race_coalesced() {
    let guard = guard_with_ttl(10);
    let refreshes = Arc::new(AtomicUsize::new(0));
    let telemetry = RefreshTelemetry::new("integration.refresh.race");

    async fn run_once(
        guard: Arc<TokenGuard>,
        telemetry: RefreshTelemetry,
        counter: Arc<AtomicUsize>,
    ) -> Result<String, snowpipe_streaming::Error> {
        let token = guard
            .ensure_fresh(false, {
                move || {
                    let counter = counter.clone();
                    async move {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        counter.fetch_add(1, Ordering::SeqCst);
                        Ok(snapshot("race-renewed", 300))
                    }
                }
            }, &telemetry)
            .await?;
        Ok(token.value)
    }

    let guard = Arc::new(guard);
    let telemetry = telemetry;
    let counter = refreshes.clone();

    let (a, b, c) = tokio::join!(
        run_once(guard.clone(), telemetry.clone(), counter.clone()),
        run_once(guard.clone(), telemetry.clone(), counter.clone()),
        run_once(guard.clone(), telemetry.clone(), counter.clone()),
    );

    assert_eq!(a.unwrap(), "race-renewed");
    assert_eq!(b.unwrap(), "race-renewed");
    assert_eq!(c.unwrap(), "race-renewed");
    assert_eq!(refreshes.load(Ordering::SeqCst), 1, "refresh executed once");
}

#[tokio::test(flavor = "current_thread")]
async fn refresh_overhead_under_five_ms() {
    let issued = SystemTime::now();
    let expires = issued + Duration::from_secs(300);
    let envelope = TokenEnvelope::try_new("steady".into(), issued, expires, false).unwrap();
    let policy = RefreshPolicy::new(
        Duration::from_millis(10),
        Duration::from_secs(10),
        Duration::from_millis(1),
    )
    .unwrap();
    let guard = TokenGuard::new(
        envelope,
        TokenGuardConfig {
            policy,
            clock_skew: Duration::from_millis(1),
        },
    );

    let telemetry = RefreshTelemetry::new("integration.refresh.overhead");
    let start = tokio::time::Instant::now();
    let _ = guard
        .ensure_fresh(false, || async { Ok(snapshot("steady", 300)) }, &telemetry)
        .await
        .unwrap();
    let elapsed = start.elapsed();
    assert!(elapsed < Duration::from_millis(5), "overhead was {:?}", elapsed);
}
