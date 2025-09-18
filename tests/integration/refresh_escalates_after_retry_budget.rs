use std::time::Duration;

use snowpipe_streaming::errors::Error;
use snowpipe_streaming::retry::{JitterStrategy, OperationKind, RetryCoordinator, RetryPlan};

#[tokio::test(flavor = "current_thread")]
async fn refresh_escalates_after_retry_budget() {
    let coordinator = RetryCoordinator::new(RetryPlan::new(
        3,
        Duration::from_millis(10),
        1.5,
        Duration::from_millis(50),
        JitterStrategy::Full,
    ));

    let err = coordinator
        .execute(OperationKind::GetScopedToken, |_attempt| async {
            Err(Error::Http(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                "simulated".into(),
            ))
        })
        .await
        .expect_err("should exhaust retries");

    match err {
        Error::Http(status, body) => {
            assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            assert!(body.contains("simulated"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}
