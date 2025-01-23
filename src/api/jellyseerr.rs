use crate::api::tmdb::Release;
use crate::AppConfig;
use reqwest;
use secrecy::ExposeSecret;
use serde::Deserialize;
use std::fmt;
use tracing::{debug, error, info};

#[derive(Debug, Deserialize)]
struct JellyseerrMediaResponse {
    pageInfo: PageInfo,
    results: Vec<RequestResult>,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    pages: i32,
    pageSize: i32,
    results: i32,
    page: i32,
}

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
) -> Result<(), JellyseerrError> {
    debug!("Requesting media: type={}, id={}", media_type, tmdb_id);
    let client = reqwest::Client::new();

    // Jellyseerr API endpoint for requesting media
    let url = format!("{}/api/v1/request", &config.jellyseerr_url);

    // Create the request body
    let body = serde_json::json!({
        "mediaType": if media_type == "tv" { "tv" } else { "movie" },
        "mediaId": tmdb_id,
    });

    let response = client
        .post(url)
        .header("accept", "application/json")
        .header(
            "X-Api-Key",
            &config.jellyseerr_api_key.expose_secret().to_string(),
        )
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            error!("Jellyseerr request failed: {}", e);
            JellyseerrError::Request(e)
        })?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        error!(
            "Jellyseerr request failed: Status={}, Body={}",
            status, error_text
        );
        return Err(JellyseerrError::Other(format!(
            "Request failed: {} - {}",
            status, error_text
        )));
    }

    info!(
        "Successfully requested media: type={}, id={}",
        media_type, tmdb_id
    );

    Ok(())
}

pub async fn filter_requested_media(
    config: &AppConfig,
    releases: Vec<Release>,
) -> Result<Vec<Release>, JellyseerrError> {
    let client = reqwest::Client::new();

    let url = format!("{}/api/v1/request?take=50", &config.jellyseerr_url);

    let response = client
        .get(&url)
        .header("accept", "application/json")
        .header(
            "X-Api-Key",
            config.jellyseerr_api_key.expose_secret().to_string(),
        )
        .send()
        .await
        .map_err(JellyseerrError::Request)?;

    if !response.status().is_success() {
        return Err(JellyseerrError::Other(format!(
            "Failed to check media status: {}",
            response.status()
        )));
    }

    let text = response.text().await.map_err(JellyseerrError::Request)?;

    let data: JellyseerrMediaResponse = serde_json::from_str(&text)
        .map_err(|e| JellyseerrError::Other(format!("Failed to parse response: {}", e)))?;

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
