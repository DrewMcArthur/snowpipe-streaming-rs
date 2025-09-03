use reqwest::Client;
use serde::Serialize;

use crate::{StreamingIngestClient, types::OpenChannelResponse};

pub struct StreamingIngestChannel<R> {
    _marker: std::marker::PhantomData<R>,
    client: StreamingIngestClient<R>,
    channel_name: String,
    continuation_token: String,
    offset: u64,
}

impl<R: Serialize + Clone> StreamingIngestChannel<R> {
    pub fn from_response(
        client: &StreamingIngestClient<R>,
        resp: OpenChannelResponse,
        channel_name: &str,
    ) -> Self {
        StreamingIngestChannel {
            _marker: std::marker::PhantomData,
            client: client.clone(),
            channel_name: channel_name.to_string(),
            continuation_token: resp.next_continuation_token,
            offset: resp.channel_status.last_committed_offset_token,
        }
    }

    /// TODO: can use the same POST to send multiple newline-delimited rows in the body
    pub async fn append_row(&mut self, row: &R) -> Result<(), reqwest::Error> {
        let url = format!(
            "https://{}/v2/streaming/data/databases/{}/schemas/{}/pipes/{}/channels/{}/rows?continuationToken={}&offsetToken={}",
            self.client.ingest_host.as_ref().expect("ingest_host not set"),
            self.client.db_name,
            self.client.schema_name,
            self.client.pipe_name,
            self.channel_name,
            self.continuation_token,
            self.offset + 1
        );

        let client = Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.client.scoped_token.as_ref().expect("scoped token not set")))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(row).expect("Failed to serialize row"))
            .send()
            .await?
            .json::<OpenChannelResponse>()
            .await?;

        self.continuation_token = resp.next_continuation_token;
        self.offset = resp.channel_status.last_committed_offset_token;
        Ok(())
    }

    pub fn get_latest_committed_offset_token(&self) -> u64 {
        self.offset
    }

    pub fn close(&self) {}
}
