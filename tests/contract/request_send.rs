use std::time::Duration;

use snowpipe_streaming::errors::Error;
use snowpipe_streaming::retry::{JitterStrategy, OperationKind, RetryCoordinator, RetryPlan};

#[tokio::test(flavor = "current_thread")]
async fn request_send_contract_surfaces_429_after_retries() {
    let coordinator = RetryCoordinator::new(RetryPlan::new(
        2,
        Duration::from_millis(5),
        1.2,
        Duration::from_millis(20),
        JitterStrategy::Full,
    ));

    let err = coordinator
        .execute(OperationKind::SendBatch, |_attempt| async {
            Err(Error::Http(
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                "throttled".into(),
            ))
        })
        .await
        .expect_err("request should surface throttling");

    match err {
        Error::Http(status, body) => {
            assert_eq!(status, reqwest::StatusCode::TOO_MANY_REQUESTS);
            assert!(body.contains("throttled"));
        }
        other => panic!("unexpected error: {:?}", other),
    }
}
