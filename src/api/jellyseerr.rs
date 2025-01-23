use crate::AppConfig;
use reqwest;
use secrecy::ExposeSecret;
use std::error::Error;

pub async fn request_media(
    config: &AppConfig,
    tmdb_id: i32,
    media_type: &str,
) -> Result<(), Box<dyn Error>> {
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
        .await?;

    if !response.status().is_success() {
        return Err(format!(
            "Jellyseerr request failed: {} - {}",
            response.status(),
            response.text().await?
        )
        .into());
    }

    Ok(())
}
