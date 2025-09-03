mod client;
mod channel;
mod types;

pub use client::StreamingIngestClient;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        #[derive(serde::Serialize, Clone)]
        struct RowType {
            id: u64,
            data: String,
            dt: DateTime
        }

        let client = StreamingIngestClient::<RowType>::new(
            "my_client",
            "my_db",
            "my_schema",
            "my_pipe",
            "{}",
        ).await;
        let channel = client.open_channel("my_channel").await.expect("Failed to open channel");
        let mut i = 1;
        while i < 1000 {
            channel.append_row(&RowType {
                id: i,
                data: "some data".to_string(),
                dt: DateTime::now()
            }).await.expect("Failed to append row");
            i += 1;
        }
        channel.close();
    }
}
