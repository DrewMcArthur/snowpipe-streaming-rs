use snowpipe_streaming::telemetry::refresh::RefreshTelemetry;

#[test]
fn telemetry_preserves_context_and_id() {
    let telemetry = RefreshTelemetry::new("ctx");
    assert_eq!(telemetry.context(), "ctx");
    let first = telemetry.attempt_id();
    assert_eq!(first, telemetry.attempt_id());
}
