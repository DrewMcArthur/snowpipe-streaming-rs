use std::time::Duration;

use tracing::Level;
use tracing::event;

use super::OperationKind;

#[derive(Debug, Clone)]
pub struct RetryOutcome {
    pub operation: OperationKind,
    pub attempts: u8,
    pub success: bool,
    pub total_delay: Duration,
}

impl RetryOutcome {
    pub fn log(&self) {
        event!(
            Level::INFO,
            operation = %self.operation,
            attempts = self.attempts,
            success = self.success,
            total_delay_ms = self.total_delay.as_millis() as u64,
            "retry.outcome"
        );
    }
}
