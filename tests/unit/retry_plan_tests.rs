use std::time::Duration;

use rand::SeedableRng;

use snowpipe_streaming::retry::{JitterStrategy, RetryPlan};

#[test]
fn delay_respects_cap_and_multiplier() {
    let plan = RetryPlan::new(
        5,
        Duration::from_millis(10),
        2.0,
        Duration::from_millis(40),
        JitterStrategy::Decorrelated,
    );
    let mut rng = rand::rngs::StdRng::seed_from_u64(7);
    let delay = plan.delay_for_attempt(4, &mut rng);
    assert!(delay <= Duration::from_millis(40));
    assert!(delay >= Duration::from_millis(5));
}
