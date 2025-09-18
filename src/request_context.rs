use std::sync::Arc;

use reqwest::Client;

use crate::retry::{RetryCoordinator, RetryPlan};
use crate::token::{RefreshPolicy, TokenEnvelope, TokenGuard, TokenGuardConfig};

/// Shared context for outbound requests ensuring consistent retry/token handling.
#[derive(Clone)]
pub struct RequestDispatchContext {
    http_client: Client,
    retry: Arc<RetryCoordinator>,
    guard: Arc<TokenGuard>,
}

impl RequestDispatchContext {
    pub fn build(
        http_client: Client,
        retry_plan: RetryPlan,
        envelope: TokenEnvelope,
        policy: RefreshPolicy,
        clock_skew: std::time::Duration,
    ) -> Self {
        let guard_config = TokenGuardConfig { policy, clock_skew };
        let guard = TokenGuard::new(envelope, guard_config);
        Self {
            http_client,
            retry: Arc::new(RetryCoordinator::new(retry_plan)),
            guard: Arc::new(guard),
        }
    }

    pub fn http_client(&self) -> &Client {
        &self.http_client
    }

    pub fn retry(&self) -> Arc<RetryCoordinator> {
        Arc::clone(&self.retry)
    }

    pub fn guard(&self) -> Arc<TokenGuard> {
        Arc::clone(&self.guard)
    }
}
