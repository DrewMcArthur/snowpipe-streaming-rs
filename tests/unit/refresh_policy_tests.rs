use std::time::Duration;

use snowpipe_streaming::errors::Error;
use snowpipe_streaming::token::RefreshPolicy;

#[test]
fn derive_threshold_uses_20_percent_cap() {
    let ttl = Duration::from_secs(1_000);
    let threshold = RefreshPolicy::derive_threshold(ttl);
    assert_eq!(threshold, Duration::from_secs(120));
}

#[test]
fn policy_rejects_high_skew() {
    let err = RefreshPolicy::new(
        Duration::from_secs(60),
        Duration::from_secs(10),
        Duration::from_secs(60),
    )
    .expect_err("max skew must be < threshold");
    assert!(matches!(err, Error::Config(_)));
}

#[test]
fn policy_from_ttl_validates_minimum_ttl() {
    let err = RefreshPolicy::from_ttl(
        Duration::from_secs(30),
        Duration::from_secs(10),
        Duration::from_secs(1),
    )
    .expect_err("ttl too short");
    assert!(matches!(err, Error::Config(_)));
}
