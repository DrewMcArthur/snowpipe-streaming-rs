use reqwest::Client;
use serde::Serialize;
use tracing::{error, info, warn};

use crate::{
    Error, StreamingIngestClient,
    types::{AppendRowsResponse, ChannelStatus, OpenChannelResponse},
};

const MAX_REQUEST_SIZE: usize = 16 * 1024 * 1024; // 16MB

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
        let token = resp
            .channel_status
            .last_committed_offset_token
            .clone()
            .unwrap_or("0".to_string())
            .parse()
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to parse last_committed_offset_token from response: {:?}",
                    resp.channel_status.last_committed_offset_token
                )
            });
        StreamingIngestChannel {
            _marker: std::marker::PhantomData,
            client: client.clone(),
            channel_name: channel_name.to_string(),
            continuation_token: resp.next_continuation_token,
            last_committed_offset_token: token,
            last_pushed_offset_token: token,
        }
    }

    /// TODO: can use the same POST to send multiple newline-delimited rows in the body, up to 16MB
    pub async fn append_row(&mut self, row: &R) -> Result<(), Error> {
        let data = serde_json::to_string(row).expect("Failed to serialize row");
        self.append_rows_call(data).await?;
        Ok(())
    }

    pub async fn append_rows(&mut self, rows: &mut dyn Iterator<Item = R>) -> Result<usize, Error> {
        let serialized_rows = rows
            .into_iter()
            .map(|r| serde_json::to_string(&r))
            .collect::<Result<Vec<String>, serde_json::Error>>()?;
        let row_size = serialized_rows.first().map(|r| r.len()).unwrap_or(0);
        let chunk_size = if row_size == 0 {
            MAX_REQUEST_SIZE
        } else {
            let denom = 2 * row_size + 1; // +1 for newline
            let cs = MAX_REQUEST_SIZE / denom;
            cs.max(1)
        };
        let requests = serialized_rows
            .chunks(chunk_size)
            .map(|batch| batch.join("\n"))
            .collect::<Vec<String>>();
        let mut bytes_written = 0;
        for req in requests {
            bytes_written += req.len();
            self.append_rows_call(req).await?;
        }
        Ok(bytes_written)
    }

    /// Append many rows using any IntoIterator of rows. This is a convenience wrapper
    /// around `append_rows` that avoids requiring a `&mut Iterator` at call sites.
    pub async fn append_rows_iter<I>(&mut self, rows: I) -> Result<usize, Error>
    where
        I: IntoIterator<Item = R>,
    {
        let mut iter = rows.into_iter();
        self.append_rows(&mut iter).await
    }

    async fn append_rows_call(&mut self, data: String) -> Result<(), Error> {
        if data.len() > MAX_REQUEST_SIZE {
            error!(
                "Data size {} exceeds maximum request size {}",
                data.len(),
                MAX_REQUEST_SIZE
            );
            return Err(Error::DataTooLarge(data.len(), MAX_REQUEST_SIZE));
        }

        info!(
            "append rows: channel='{}' bytes={}",
            self.channel_name,
            data.len()
        );
        let offset = self.last_pushed_offset_token + 1;
        let ingest = self
            .client
            .ingest_host
            .as_ref()
            .expect("ingest_host not set");
        let base = if ingest.contains("://") {
            ingest.trim_end_matches('/').to_string()
        } else {
            format!("https://{}", ingest)
        };
        let url = format!(
            "{}/v2/streaming/data/databases/{}/schemas/{}/pipes/{}/channels/{}/rows?continuationToken={}&offsetToken={}",
            base,
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
            .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
            .body(data)
            .send()
            .await?
            .error_for_status()?
            .json::<AppendRowsResponse>()
            .await?;

        self.last_pushed_offset_token = offset;
        self.continuation_token = resp.next_continuation_token;
        info!(
            "append rows ok: channel='{}' pushed_offset={} next_ctok='{}'",
            self.channel_name, self.last_pushed_offset_token, self.continuation_token
        );
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
        let ingest = self
            .client
            .ingest_host
            .as_ref()
            .expect("ingest_host not set");
        let base = if ingest.contains("://") {
            ingest.trim_end_matches('/').to_string()
        } else {
            format!("https://{}", ingest)
        };
        let url = format!(
            "{}/v2/streaming/databases/{}/schemas/{}/pipes/{}:bulk-channel-status",
            base, self.client.db_name, self.client.schema_name, self.client.pipe_name,
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
            .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
            .body(format!(
                "{{\"channel_names\": [\"{}\"]}}",
                self.channel_name
            ))
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;

        let status = resp
            .get("channel_statuses")
            .and_then(|cs| cs.get(&self.channel_name))
            .map(|s| serde_json::from_value::<ChannelStatus>(s.clone()));

        match status {
            Some(Ok(status)) => {
                info!(
                    "channel status: committed={:?}",
                    status.last_committed_offset_token
                );
                self.last_committed_offset_token = status.last_committed_offset_token .clone()
            .unwrap_or("0".to_string())
            .parse()
            .unwrap_or_else(|_| panic!(
                "Failed to parse last_committed_offset_token while getting channel status {:?}",
                status.last_committed_offset_token
            ));
            }
            s => {
                error!("channel status parse failed: {:?}", s);
            }
        }

        Ok(())
    }

    pub async fn close(&mut self) -> Result<(), Error> {
        self.close_with_timeout(std::time::Duration::from_secs(5 * 60))
            .await
    }

    pub async fn close_with_timeout(&mut self, timeout: std::time::Duration) -> Result<(), Error> {
        let start = tokio::time::Instant::now();
        let mut last_warn_minute = 0u64;
        while self.last_committed_offset_token < self.last_pushed_offset_token {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            self.get_channel_status()
                .await
                .expect("Failed to get channel status");

            let elapsed = start.elapsed();
            let elapsed_mins = elapsed.as_secs() / 60;
            if elapsed_mins >= 1 && elapsed_mins > last_warn_minute {
                last_warn_minute = elapsed_mins;
                warn!(
                    "Channel '{}' close is still waiting after {} minute(s); committed={} pushed={}",
                    self.channel_name,
                    elapsed_mins,
                    self.last_committed_offset_token,
                    self.last_pushed_offset_token
                );
            }
            if elapsed >= timeout {
                error!(
                    "Channel '{}' close timed out after {:?}; committed={} pushed={}",
                    self.channel_name,
                    timeout,
                    self.last_committed_offset_token,
                    self.last_pushed_offset_token
                );
                return Err(Error::Timeout(timeout));
            }
        }

        let ingest = self
            .client
            .ingest_host
            .as_ref()
            .expect("ingest_host not set");
        let base = if ingest.contains("://") {
            ingest.trim_end_matches('/').to_string()
        } else {
            format!("https://{}", ingest)
        };
        let url = format!(
            "{}/v2/streaming/databases/{}/schemas/{}/pipes/{}/channels/{}",
            base,
            self.client.db_name,
            self.client.schema_name,
            self.client.pipe_name,
            self.channel_name
        );

        let client = Client::new();
        client
            .delete(&url)
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
            .header("User-Agent", "snowpipe-streaming-rust-sdk/0.1.0")
            .send()
            .await?
            .error_for_status()?;

        info!("channel closed: name='{}'", self.channel_name);

        Ok(())
    }
}

// (Unit tests live in integration to avoid constructing private client internals.)
