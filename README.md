# Snowpipe Streaming Rust SDK

This library is meant to replicate the python api interface, to allow users to stream data to snowflake via the snowpipe streaming API.  This is a naive implementation, since the Python and Java SDKs are not open source, so I'm guessing based off the REST guide and the external visibility of the Python SDK.

## References
- [Python SDK Guide](https://docs.snowflake.com/en/user-guide/snowpipe-streaming-high-performance-getting-started)
- [Python Example Usage](https://gist.github.com/sfc-gh-chathomas/a7b06bb46907bead737954d53b3a8495#file-example-py/)
- [REST Guide](https://docs.snowflake.com/en/user-guide/snowpipe-streaming-high-performance-rest-tutorial)


## TODO
- check flows and make sure it all works, get example working
- batch `append_row` requests, to speed up / minimize HTTP throughput
- documentation! doctests!
- mock http server? 
- change how client is created, use a builder or the model that has different structs for different stages so that errors are less possible, like what's described [here](https://blog.systems.ethz.ch/blog/2018/a-hammer-you-can-only-hold-by-the-handle.html)