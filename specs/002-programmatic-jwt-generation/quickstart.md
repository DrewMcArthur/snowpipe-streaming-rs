Quickstart: Programmatic JWT Generation

Config
```
{
  "user": "<SF_USER>",
  "account": "<SF_ACCOUNT>",
  "url": "https://<account-host>",
  "private_key_path": "/path/to/private_key.pem",
  "jwt_exp_secs": 60
}
```

Usage
```
use snowpipe_streaming::{ConfigLocation, StreamingIngestClient};

let client = StreamingIngestClient::<YourRow>::new(
  "svc-client", "DB", "SCHEMA", "PIPE",
  ConfigLocation::File("config.json".into())
).await?;
// Client will generate control-plane token via /oauth2/token using private key
```
