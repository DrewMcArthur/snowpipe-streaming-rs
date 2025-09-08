use reqwest::Client;
use serde::Serialize;

use crate::{
    types::{ChannelStatus, OpenChannelResponse}, Error, StreamingIngestClient
};

pub struct StreamingIngestChannel<R> {
    _marker: std::marker::PhantomData<R>,
    client: StreamingIngestClient<R>,
    channel_name: String,
    continuation_token: String,
    last_committed_offset_token: u64,
    last_pushed_offset_token: u64,
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
            last_committed_offset_token: resp.channel_status.last_committed_offset_token,
            last_pushed_offset_token: resp.channel_status.last_committed_offset_token,
        }
    }

    /// TODO: can use the same POST to send multiple newline-delimited rows in the body
    pub async fn append_row(&mut self, row: &R) -> Result<(), Error> {
        let offset = self.last_pushed_offset_token + 1;
        let url = format!(
            "https://{}/v2/streaming/data/databases/{}/schemas/{}/pipes/{}/channels/{}/rows?continuationToken={}&offsetToken={}",
            self.client
                .ingest_host
                .as_ref()
                .expect("ingest_host not set"),
            self.client.db_name,
            self.client.schema_name,
            self.client.pipe_name,
            self.channel_name,
            self.continuation_token,
            offset
        );

        let client = Client::new();
        let resp = client
            .post(&url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    self.client
                        .scoped_token
                        .as_ref()
                        .expect("scoped token not set")
                ),
            )
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(row).expect("Failed to serialize row"))
            .send()
            .await?
            .json::<OpenChannelResponse>()
            .await?;

        self.last_pushed_offset_token = offset;
        self.continuation_token = resp.next_continuation_token;
        self.last_committed_offset_token = resp.channel_status.last_committed_offset_token;
        Ok(())
    }

    pub async fn get_latest_committed_offset_token(&mut self) -> u64 {
        self.get_channel_status()
            .await
            .expect("Failed to get channel status");
        self.last_committed_offset_token
    }

    async fn get_channel_status(&mut self) -> Result<(), Error> {
        let client = Client::new();
        let url = format!(
            "https://{}/v2/streaming/databases/{}/schemas/{}/pipes/{}:bulk-channel-status",
            self.client
                .ingest_host
                .as_ref()
                .expect("ingest_host not set"),
            self.client.db_name,
            self.client.schema_name,
            self.client.pipe_name,
        );

        let resp = client
            .post(&url)
            .header(
                "Authorization",
                format!(
                    "Bearer {}",
                    self.client
                        .scoped_token
                        .as_ref()
                        .expect("scoped token not set")
                ),
            )
            .header("Content-Type", "application/json")
            .body(format!(
                "{{\"channel_names\": [\"{}\"]}}",
                self.channel_name
            ))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let status = resp
            .get("channel_statuses")
            .and_then(|cs| cs.get(&self.channel_name))
            .map(|s| serde_json::from_value::<ChannelStatus>(s.clone()));

        match status {
            Some(Ok(status)) => {
                println!("Channel Status: {:?}", status);
                self.last_committed_offset_token = status.last_committed_offset_token;
            }
            s => {
                println!("Failed to parse channel status: {:?}", s);
            }
        }

        Ok(())
    }

    pub async fn close(&mut self) {
        while self.last_committed_offset_token < self.last_pushed_offset_token {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            self.get_channel_status().await.expect("Failed to get channel status");
        }
    }
}
