# TMDB2seer
I was asked by a user of my media stack for the ability to view new releases with ratings in a neat table so they could see what they want a directly request to jellyseer.

## How to use
**UPDATE:**
You can now use a config file when running in `config/default.toml`
```toml
[tmdb]
api_key = "your-default-key-here"

[jellyseerr]
api_key = "your-default-key-here"
url = "http://localhost:5055"

[server]
host = "0.0.0.0"
port = 3000

[rate_limit]
requests_per_second = 10
burst_size = 20
```

Then you can run the binary with `cargo run --release` or `cargo build --release && ./target/release/tvdb_ratings`.

You can override the default settings with environment variables prefixed with APP_:
```
APP_TMDB__API_KEY=your api key
APP_SERVER__PORT=1111
```
## Why not use Jellyseer to view new releases?
I don't know - I just wanted to make this to learn more about Rust and Axum.
