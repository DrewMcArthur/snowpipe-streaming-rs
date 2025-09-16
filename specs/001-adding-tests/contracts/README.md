HTTP Contract Samples for Integration Tests

Endpoints emulated by the mock server:

1) GET {control}/v2/streaming/hostname
- Response: text/plain with base URL string, e.g. `http://127.0.0.1:PORT` (tests) or `https://ingest.example.com` (docs).

2) POST {control}/oauth/token
- Response: text/plain with opaque token string, e.g. `scoped-token`.

3) PUT {ingest}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel}
- Response JSON: see open_channel_response.json

4) POST {ingest}/v2/streaming/data/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel}/rows?continuationToken=...&offsetToken=...
- Response JSON: see append_rows_response.json

5) POST {ingest}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}:bulk-channel-status
- Response JSON: see channel_status_response.json

6) DELETE {ingest}/v2/streaming/databases/{db}/schemas/{schema}/pipes/{pipe}/channels/{channel}
- Response: 200 OK, empty body

Notes
- All JSON fields align with types in `src/types.rs`.
- For tests, status progression can be simulated by advancing `last_committed_offset_token` in responses.
