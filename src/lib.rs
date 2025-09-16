mod channel;
mod client;
mod config;
mod errors;
mod types;

pub use channel::StreamingIngestChannel;
pub use client::StreamingIngestClient;
pub use config::Config;
pub use errors::Error;

#[cfg(test)]
mod tests {
    use jiff::Zoned;

    use crate::config::Config;

    use super::*;

    #[ignore]
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
            Config::from_values(
                "user",
                "acct",
                "https://example",
                Some("jwt".into()),
                None,
                None,
                None,
                Some(60),
            ),
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
