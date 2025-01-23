use crate::AppConfig;
use reqwest;
use secrecy::ExposeSecret;
use std::fmt;
use tracing::{debug, error, info};

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
