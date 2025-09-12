use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenChannelResponse {
    pub next_continuation_token: String,
    pub channel_status: ChannelStatus,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ChannelStatus {
    database_name: String,
    schema_name: String,
    pipe_name: String,
    channel_name: String,
    channel_status_code: String,
    pub last_committed_offset_token: Option<String>,
    created_on_ms: u64,
    rows_inserted: Option<i32>,
    rows_parsed: Option<i32>,
    rows_errors: Option<i32>,
    last_error_offset_upper_bound: Option<String>,
    last_error_message: Option<String>,
    last_error_timestamp: Option<u64>, // timestamp_utc
    snowflake_avg_processing_latency_ms: Option<i32>,
}
