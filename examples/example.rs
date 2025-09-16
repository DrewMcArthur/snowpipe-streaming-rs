use snowpipe_streaming::{Config, StreamingIngestClient};

#[derive(serde::Serialize, Clone)]
struct Row {
    id: u64,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Optional: enable basic logging for the example
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();

    // Load configuration from a JSON file placed next to the binary
    let cfg = Config::from_file("config.json")?;
    let client =
        StreamingIngestClient::<Row>::new("EXAMPLE_CLIENT", "MY_DB", "MY_SCHEMA", "MY_PIPE", cfg)
            .await?;

    let mut ch = client.open_channel("example_channel").await?;
    ch.append_row(&Row { id: 1 }).await?;
    ch.append_rows_iter(vec![Row { id: 2 }, Row { id: 3 }])
        .await?;
    ch.close().await?;
    Ok(())
}
