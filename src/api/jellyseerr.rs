use crate::api::client::ApiClient;
use crate::api::tmdb::Release;
use crate::{AppConfig, Result};
use reqwest;
use serde::Deserialize;
use std::fmt;
use tracing::{debug, info};

#[derive(Debug, Deserialize)]
struct JellyseerrMediaResponse {
    results: Vec<RequestResult>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RequestResult {
    #[serde(rename = "type")]
    request_type: String,
    media: MediaInfo,
}

#[derive(Debug, Deserialize)]
struct MediaInfo {
    #[serde(rename = "mediaType")] // Match exact field name from JSON
    media_type: String,
    #[serde(rename = "tmdbId")] // Match exact field name from JSON
    tmdb_id: i32,
}

#[derive(Debug)]
pub enum JellyseerrError {
    Request(reqwest::Error),
    Other(String),
}

impl std::error::Error for JellyseerrError {}

impl fmt::Display for JellyseerrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JellyseerrError::Request(e) => write!(f, "Request (reqwest) error: {}", e),
            JellyseerrError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

pub async fn request_media(
    config: &AppConfig,
    tmdb_id: i32,
    media_type: &str,
    season: Option<Vec<i32>>,
) -> Result<()> {
    debug!("Requesting media: type={}, id={}", media_type, tmdb_id);
    let client = ApiClient::new(&config);

    // Create the request body
    let body = match media_type {
        "movie" => serde_json::json!({
            "mediaType": "movie",
            "mediaId": tmdb_id,
        }),
        "tv" => serde_json::json!({
            "mediaType": "tv",
            "mediaId": tmdb_id,
            "seasons": season,
        }),
        _ => {
            return Err(
                JellyseerrError::Other(format!("Invalid media type: {}", media_type)).into(),
            );
        }
    };
    debug!("Sending request to Jellyseerr: {:?}", body);

    let _: () = client
        .jellyseerr_post(
            "request",
            &body,
            &config.jellyseerr_api_key,
            &config.jellyseerr_url,
        )
        .await?;

    info!(
        "Successfully requested media: type={}, id={}",
        media_type, tmdb_id
    );

    Ok(())
}

pub async fn filter_requested_media(
    config: &AppConfig,
    releases: Vec<Release>,
) -> Result<Vec<Release>> {
    let client = ApiClient::new(&config);

    let data: JellyseerrMediaResponse = client
        .jellyseerr_get(
            "request?take=50",
            &config.jellyseerr_api_key,
            &config.jellyseerr_url,
        )
        .await?;

    let requested_media: std::collections::HashSet<(String, i32)> = data
        .results
        .iter()
        .map(|request| (request.media.media_type.clone(), request.media.tmdb_id))
        .collect();

    let filtered_releases = releases
        .into_iter()
        .filter(|release| !requested_media.contains(&(release.media_type.clone(), release.id)))
        .collect();

    Ok(filtered_releases)
}
