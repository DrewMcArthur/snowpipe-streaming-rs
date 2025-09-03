use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenChannelResponse {
    pub next_continuation_token: String,
    pub channel_status: ChannelStatus
}

#[derive(Deserialize)]
pub struct ChannelStatus {
    pub last_committed_offset_token: u64
}