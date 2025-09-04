use snowpipe_streaming::StreamingIngestClient;

fn main() {
    let max_rows = 100_000;
    let poll_attempts = 30;
    let poll_interval_ms = 1000;

    // Create Snowflake Streaming Ingest Client
    let uuid = uuid();
    let client = StreamingIngestClient::<Row>::new(
        client_name = format!("MY_CLIENT_{uuid}"),
        db_name = "MY_DATABASE",
        schema_name = "MY_SCHEMA",
        pipe_name = "MY_PIPE",
        profile_json = "profile.json", //depends on your folder structure
    ).await.expect("failed to create client");

    // Open a channel for data ingestion
    let channel_uuid = uuid();
    let channel_name = format!("my_channel_{channel_uuid}");
    let channel = client.open_channel(channel_name);
    println!("Channel opened: {channel_name}");

    // Ingest rows
    println!("Ingesting {max_rows} rows");
    for i in 0..max_rows {
        let row_id = str(i);
        let row = Row {
            c1: i,
            c2: row_id,
            ts: Datetime::now(),
        };
        channel.append_row(row, row_id);
    }

    // Wait for ingestion to complete
    for attempt in 0..poll_attempts {
        latest_offset = channel.get_latest_committed_offset_token();
        println!("Latest offset token: {latest_offset}");
        if latest_offset == max_rows - 1 {
            println!("All data committed successfully");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(poll_interval_ms));
    }

    // Close resources
    channel.close();
    client.close();

    println!("Data ingestion completed");
}
