# TMDB2seer
I was asked by a user of my media stack for the ability to view new releases with ratings in a neat table so they could see what they want a directly request to jellyseer.

## How to use
Currently, the only way to run this is to clone the repository and set the following env variables:
- `TMDB_API_KEY` - Your TMDB API key
- `JELLYSEERR_URL` - The URL of your jellyseer instance
- `JELLYSEERR_API_KEY` - The API key of your jellyseer instance

Then you can run the binary with `cargo run --release`.

## Why not use Jellyseer to view new releases?
I don't know - I just wanted to make this to learn more about Rust and Axum.
