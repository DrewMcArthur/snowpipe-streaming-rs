#![allow(dead_code)]
mod channel;
mod client;
mod config;
mod errors;
mod types;

pub use channel::StreamingIngestChannel;
pub use client::StreamingIngestClient;
pub use errors::Error;
pub use config::ConfigLocation;

#[cfg(test)]
mod tests {
    use jiff::Zoned;

    use crate::config::ConfigLocation;

    use super::*;

    #[tokio::test]
    async fn it_works() {
        #[derive(serde::Serialize, Clone)]
        struct RowType {
            id: u64,
            data: String,
            dt: Zoned,
        }

        let client = StreamingIngestClient::<RowType>::new(
            "my_client",
            "my_db",
            "my_schema",
            "my_pipe",
            ConfigLocation::File("{}".into()),
        )
        .await
        .expect("Failed to create client");
        let mut channel = client
            .open_channel("my_channel")
            .await
            .expect("Failed to open channel");
        let mut i = 1;
        while i < 1000 {
            channel
                .append_row(&RowType {
                    id: i,
                    data: "some data".to_string(),
                    dt: Zoned::now(),
                })
                .await
                .expect("Failed to append row");
            i += 1;
        }
        channel.close().await.unwrap();
    }
}
