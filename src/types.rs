use serde::Deserialize;

#[derive(Deserialize)]
pub struct AppendRowsResponse {
    pub next_continuation_token: String,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_open_channel_response() {
        let json = r#"{
          "next_continuation_token": "ctok-1",
          "channel_status": {
            "database_name": "db",
            "schema_name": "schema",
            "pipe_name": "pipe",
            "channel_name": "ch",
            "channel_status_code": "OPEN",
            "last_committed_offset_token": "0",
            "created_on_ms": 0
          }
        }"#;
        let resp: OpenChannelResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_continuation_token, "ctok-1");
        assert_eq!(resp.channel_status.channel_name, "ch");
    }

    #[test]
    fn parse_append_rows_response() {
        let json = r#"{ "next_continuation_token": "ctok-2" }"#;
        let resp: AppendRowsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.next_continuation_token, "ctok-2");
    }

    #[test]
    fn parse_channel_status_response() {
        let json = r#"{
          "channel_statuses": {
            "ch": {
              "database_name": "db",
              "schema_name": "schema",
              "pipe_name": "pipe",
              "channel_name": "ch",
              "channel_status_code": "OPEN",
              "last_committed_offset_token": "1",
              "created_on_ms": 0
            }
          }
        }"#;
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        let status = v.get("channel_statuses")
            .and_then(|cs| cs.get("ch"))
            .map(|s| serde_json::from_value::<ChannelStatus>(s.clone())).unwrap().unwrap();
        assert_eq!(status.channel_name, "ch");
        assert_eq!(status.last_committed_offset_token.as_deref(), Some("1"));
    }
}
