use std::time::Duration;

use crate::errors::Error;

/// Business rules governing proactive refresh behaviour.
#[derive(Clone, Debug)]
pub struct RefreshPolicy {
    /// Minimum acceptable time-to-expiry before forcing a refresh.
    pub threshold: Duration,
    /// Minimum time between successful refreshes to avoid thrashing.
    pub min_cooldown: Duration,
    /// Acceptable clock skew margin when evaluating expiry.
    pub max_skew: Duration,
}

impl RefreshPolicy {
    pub fn new(
        threshold: Duration,
        min_cooldown: Duration,
        max_skew: Duration,
    ) -> Result<Self, Error> {
        if threshold.is_zero() {
            return Err(Error::Config("Refresh threshold must be > 0".into()));
        }
        if max_skew >= threshold {
            return Err(Error::Config(
                "Clock skew must be lower than the refresh threshold".into(),
            ));
        }
        Ok(Self {
            threshold,
            min_cooldown,
            max_skew,
        })
    }

    /// Builds a policy using the "lesser of" derivation between absolute seconds and TTL percentage.
    pub fn from_ttl(
        ttl: Duration,
        min_cooldown: Duration,
        max_skew: Duration,
    ) -> Result<Self, Error> {
        if ttl < Duration::from_secs(60) {
            return Err(Error::Config("TTL must be ≥ 60 seconds".into()));
        }
        let threshold = Self::derive_threshold(ttl);
        if threshold > ttl {
            return Err(Error::Config(
                "Refresh threshold cannot exceed token TTL".into(),
            ));
        }
        Self::new(threshold, min_cooldown, max_skew)
    }

    /// Applies the "lesser of" logic between absolute threshold and TTL percentage.
    pub fn derive_threshold(ttl: Duration) -> Duration {
        let percent_window = ttl.mul_f64(0.2);
        let hard_cap = Duration::from_secs(120);
        std::cmp::min(percent_window, hard_cap)
    }
}
