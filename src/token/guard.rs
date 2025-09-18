use std::sync::Arc;
use std::time::{Duration, SystemTime};

use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::sync::{Mutex, RwLock};
use tracing::debug;

use crate::errors::Error;
use crate::telemetry::refresh::{RefreshOutcome, RefreshTelemetry};

use super::{RefreshPolicy, TokenEnvelope, TokenSnapshot};

/// Convenience result alias for guard operations.
pub type TokenGuardResult<T> = Result<T, Error>;

/// Configuration inputs required to build a TokenGuard.
#[derive(Clone)]
pub struct TokenGuardConfig {
    pub policy: RefreshPolicy,
    pub clock_skew: Duration,
}

/// Coordinates token refresh decisions and provides refreshed token snapshots.
pub struct TokenGuard {
    envelope: Arc<RwLock<TokenEnvelope>>,
    policy: RefreshPolicy,
    refresh_lock: Mutex<()>,
    clock_skew: Duration,
    last_refresh: Arc<RwLock<Option<SystemTime>>>,
    rng: Mutex<StdRng>,
}

impl TokenGuard {
    pub fn new(envelope: TokenEnvelope, config: TokenGuardConfig) -> Self {
        Self {
            envelope: Arc::new(RwLock::new(envelope)),
            policy: config.policy,
            refresh_lock: Mutex::new(()),
            clock_skew: config.clock_skew,
            last_refresh: Arc::new(RwLock::new(None)),
            rng: Mutex::new(StdRng::from_entropy()),
        }
    }

    pub fn envelope(&self) -> Arc<RwLock<TokenEnvelope>> {
        Arc::clone(&self.envelope)
    }

    pub async fn ensure_fresh<F, Fut>(
        &self,
        force_refresh: bool,
        refresh_cb: F,
        telemetry: &RefreshTelemetry,
    ) -> TokenGuardResult<TokenSnapshot>
    where
        F: FnMut() -> Fut + Send,
        Fut: std::future::Future<Output = TokenGuardResult<TokenSnapshot>> + Send,
    {
        let mut refresh_cb = refresh_cb;
        let now = SystemTime::now();
        let last_refresh = { *self.last_refresh.read().await };
        {
            let envelope = self.envelope.read().await;
            if !force_refresh && !self.should_refresh(&envelope, last_refresh, now) {
                return Ok(envelope.to_snapshot());
            }
        }

        // Only one refresh attempt should run at a time.
        let _lock = self.refresh_lock.lock().await;
        let last_refresh = { *self.last_refresh.read().await };
        {
            let mut envelope = self.envelope.write().await;
            if !force_refresh && !self.should_refresh(&envelope, last_refresh, SystemTime::now()) {
                return Ok(envelope.to_snapshot());
            }
            envelope.set_refresh_in_progress(true);
        }

        let jitter = {
            let mut rng = self.rng.lock().await;
            self.policy.threshold.mul_f64(rng.gen_range(0.0..=0.1))
        };
        if !jitter.is_zero() {
            tokio::time::sleep(jitter).await;
        }

        let refresh_start = SystemTime::now();
        telemetry.emit_start(refresh_start);
        match refresh_cb().await {
            Ok(snapshot) => {
                {
                    let mut writer = self.envelope.write().await;
                    writer.update_from_snapshot(snapshot.clone())?;
                }
                {
                    let mut last_refresh = self.last_refresh.write().await;
                    *last_refresh = Some(refresh_start);
                }
                telemetry.emit_success(RefreshOutcome::Success, SystemTime::now());
                Ok(self.envelope.read().await.to_snapshot())
            }
            Err(err) => {
                telemetry.emit_failure(&err, SystemTime::now());
                {
                    let mut writer = self.envelope.write().await;
                    writer.set_refresh_in_progress(false);
                }
                Err(err)
            }
        }
    }

    pub async fn force_refresh<F, Fut>(
        &self,
        refresh_cb: F,
        telemetry: &RefreshTelemetry,
    ) -> TokenGuardResult<TokenSnapshot>
    where
        F: FnMut() -> Fut + Send,
        Fut: std::future::Future<Output = TokenGuardResult<TokenSnapshot>> + Send,
    {
        self.ensure_fresh(true, refresh_cb, telemetry).await
    }

    fn should_refresh(
        &self,
        envelope: &TokenEnvelope,
        last_refresh: Option<SystemTime>,
        now: SystemTime,
    ) -> bool {
        if envelope.refresh_in_progress() {
            return false;
        }

        if let Some(remaining) = envelope.remaining(now + self.clock_skew) {
            if remaining > (self.policy.threshold + self.policy.max_skew) {
                return false;
            }
        } else {
            debug!("token already expired; forcing refresh");
            return true;
        }

        if let Some(last) = last_refresh
            && let Ok(elapsed) = now.duration_since(last)
            && elapsed < self.policy.min_cooldown
        {
            return false;
        }

        true
    }
}
