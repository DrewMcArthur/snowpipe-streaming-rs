use rand::{SeedableRng, rngs::StdRng};
use std::fmt;
use tokio::sync::Mutex;
use tokio::time::Instant;
use tracing::warn;

use crate::errors::Error;

use super::{RetryOutcome, plan::RetryPlan};

#[derive(Debug, Clone, Copy)]
pub enum OperationKind {
    DiscoverHost,
    GetScopedToken,
    OpenChannel,
    SendBatch,
}

impl fmt::Display for OperationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OperationKind::DiscoverHost => write!(f, "discover_ingest_host"),
            OperationKind::GetScopedToken => write!(f, "get_scoped_token"),
            OperationKind::OpenChannel => write!(f, "open_channel"),
            OperationKind::SendBatch => write!(f, "append_rows"),
        }
    }
}

pub struct RetryCoordinator {
    plan: RetryPlan,
    rng: Mutex<StdRng>,
}

impl RetryCoordinator {
    pub fn new(plan: RetryPlan) -> Self {
        Self {
            plan,
            rng: Mutex::new(StdRng::from_entropy()),
        }
    }

    pub fn plan(&self) -> RetryPlan {
        self.plan.clone()
    }

    pub async fn execute<F, Fut, T>(
        &self,
        operation: OperationKind,
        mut op: F,
    ) -> Result<(T, RetryOutcome), Error>
    where
        F: FnMut(u8) -> Fut + Send,
        Fut: std::future::Future<Output = Result<T, Error>> + Send,
    {
        let mut attempt: u8 = 1;
        let start = Instant::now();
        loop {
            match op(attempt).await {
                Ok(value) => {
                    let outcome = RetryOutcome {
                        operation,
                        attempts: attempt,
                        success: true,
                        total_delay: start.elapsed(),
                    };
                    outcome.log();
                    return Ok((value, outcome));
                }
                Err(err) => {
                    if attempt >= self.plan.max_attempts || !Self::is_retriable(&err) {
                        let outcome = RetryOutcome {
                            operation,
                            attempts: attempt,
                            success: false,
                            total_delay: start.elapsed(),
                        };
                        outcome.log();
                        return Err(err);
                    }
                    let delay = {
                        let mut rng = self.rng.lock().await;
                        self.plan.delay_for_attempt(attempt + 1, &mut *rng)
                    };
                    warn!(
                        operation = %operation,
                        attempt,
                        max_attempts = self.plan.max_attempts,
                        delay_ms = delay.as_millis() as u64,
                        error = %err,
                        "retry.scheduling"
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }

    fn is_retriable(err: &Error) -> bool {
        matches!(
            err,
            Error::Http(_, _)
                | Error::Reqwest(_)
                | Error::IngestHostDiscovery(_, _)
                | Error::Timeout(_)
        )
    }
}

impl Default for RetryCoordinator {
    fn default() -> Self {
        Self::new(RetryPlan::default_plan())
    }
}
