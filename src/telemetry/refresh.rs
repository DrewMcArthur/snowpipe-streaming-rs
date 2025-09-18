use std::time::SystemTime;

use tracing::{Level, event};
use uuid::Uuid;

use crate::errors::Error;

#[derive(Clone, Debug)]
pub enum RefreshOutcome {
    Success,
    Retrying,
    Failed,
}

#[derive(Clone, Debug)]
pub struct RefreshTelemetry {
    attempt_id: Uuid,
    context: String,
}

impl RefreshTelemetry {
    pub fn new(context: impl Into<String>) -> Self {
        Self {
            attempt_id: Uuid::new_v4(),
            context: context.into(),
        }
    }

    pub fn attempt_id(&self) -> Uuid {
        self.attempt_id
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    pub fn emit_start(&self, at: SystemTime) {
        event!(
            Level::INFO,
            attempt_id = %self.attempt_id,
            context = %self.context,
            timestamp = ?at,
            "refresh.start"
        );
    }

    pub fn emit_success(&self, outcome: RefreshOutcome, at: SystemTime) {
        event!(
            Level::INFO,
            attempt_id = %self.attempt_id,
            context = %self.context,
            timestamp = ?at,
            outcome = ?outcome,
            "refresh.success"
        );
    }

    pub fn emit_retry(&self, attempt: u8, delay_ms: u64) {
        event!(
            Level::WARN,
            attempt_id = %self.attempt_id,
            context = %self.context,
            attempt,
            delay_ms,
            "refresh.retry"
        );
    }

    pub fn emit_failure(&self, error: &Error, at: SystemTime) {
        event!(
            Level::ERROR,
            attempt_id = %self.attempt_id,
            context = %self.context,
            timestamp = ?at,
            error = %error,
            "refresh.failure"
        );
    }
}
