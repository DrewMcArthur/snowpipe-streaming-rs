use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenChannelResponse {
    pub next_continuation_token: String,
    pub channel_status: ChannelStatus
}

#[derive(Debug, Deserialize)]
pub struct ChannelStatus {
    channel_status_code: String,
    pub last_committed_offset_token: u64,
    database_name: String,
    schema_name: String,
    pipe_name: String,
    channel_name: String,
    rows_inserted: i32,
    rows_parsed: i32,
    rows_errors: i32,
    last_error_offset_upper_bound: String,
    last_error_message: String,
    last_error_timestamp: String, // timestamp_utc
    snowflake_avg_processing_latency_ms: i32
}