use std::str::FromStr;
use std::time::Duration;

use rand::Rng;

use crate::errors::Error;

/// Strategy for adding randomness to delay calculations.
#[derive(Clone, Copy, Debug)]
pub enum JitterStrategy {
    Full,
    Decorrelated,
}

/// Shared retry/backoff configuration for Snowflake-bound HTTP operations.
#[derive(Clone, Debug)]
pub struct RetryPlan {
    pub max_attempts: u8,
    pub initial_delay: Duration,
    pub multiplier: f32,
    pub max_delay: Duration,
    pub jitter: JitterStrategy,
}

impl RetryPlan {
    pub fn new(
        max_attempts: u8,
        initial_delay: Duration,
        multiplier: f32,
        max_delay: Duration,
        jitter: JitterStrategy,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay,
            multiplier,
            max_delay,
            jitter,
        }
    }

    pub fn default_plan() -> Self {
        Self {
            max_attempts: 4,
            initial_delay: Duration::from_millis(200),
            multiplier: 1.8,
            max_delay: Duration::from_secs(5),
            jitter: JitterStrategy::Full,
        }
    }

    pub fn delay_for_attempt(&self, attempt: u8, rng: &mut impl Rng) -> Duration {
        if attempt <= 1 {
            return self.initial_delay;
        }
        let exp = (self.multiplier.powi((attempt as i32) - 1)) as f64;
        let mut delay = self.initial_delay.mul_f64(exp).min(self.max_delay);
        let jitter = match self.jitter {
            JitterStrategy::Full => rng.gen_range(0.0..1.0),
            JitterStrategy::Decorrelated => rng.gen_range(0.5..1.5),
        };
        delay = delay.mul_f64(jitter);
        delay
    }
}

impl FromStr for JitterStrategy {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "full" => Ok(JitterStrategy::Full),
            "decorrelated" => Ok(JitterStrategy::Decorrelated),
            other => Err(Error::Config(format!(
                "Unknown jitter strategy '{}'; expected 'full' or 'decorrelated'",
                other
            ))),
        }
    }
}
