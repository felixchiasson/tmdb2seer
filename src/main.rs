use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::{error::Error, sync::Arc, time::Duration};

mod api {
    pub mod middleware;
    mod rate_limiter;
}
use crate::api::middleware::RateLimitServiceLayer;

// Configuration struct
#[derive(Clone)]
struct AppConfig {
    tmdb_api_key: String,
    jellyseerr_api_key: String,
    jellyseerr_url: String,
}

// TMDB Response structures
#[derive(Debug, Deserialize)]
struct TMDBResult {
    id: i32,
    title: Option<String>,
    name: Option<String>,
    release_date: Option<String>,
    first_air_date: Option<String>,
    #[serde(rename = "media_type")]
    media_type: String,
    vote_average: f32,
    vote_count: i32,
    poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TMDBResponse {
    results: Vec<TMDBResult>,
}

#[derive(Debug, Serialize)]
struct Release {
    id: i32,
    title: String,
    release_date: String,
    media_type: String,
    vote_average: f32,
    vote_count: i32,
    poster_url: String,
    tmdb_url: String,
}

#[derive(Template)]
#[template(path = "../templates/index.html")] // Note the changed path
struct IndexTemplate {
    releases: Vec<Release>,
}

async fn request_media(
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
        .header("X-Api-Key", &config.jellyseerr_api_key)
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

async fn test_rate_limit() -> impl IntoResponse {
    tokio::time::sleep(Duration::from_millis(50)).await; // Add a small delay to make rate limiting more noticeable
    "OK".into_response()
}

async fn add_to_jellyseerr(
    State(config): State<Arc<AppConfig>>,
    Path((media_type, id)): Path<(String, i32)>,
) -> impl IntoResponse {
    match request_media(&config, id, &media_type).await {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            eprintln!("Error requesting media: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

async fn fetch_latest_releases(api_key: &str) -> Result<Vec<Release>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.themoviedb.org/3/trending/all/week?api_key={}",
        api_key
    );

    println!("Requesting URL: {}", url.replace(api_key, "API_KEY")); // Log URL (safely)

    let response = client
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await?;

    let status = response.status();
    println!("Response status: {}", status);

    let text = response.text().await?;
    println!("Raw API Response: {}", text);

    // Try to parse as a generic JSON value first
    let json: serde_json::Value = serde_json::from_str(&text)?;
    println!("Parsed JSON structure: {:#?}", json);

    // Now try to parse into our structure
    let response: TMDBResponse = serde_json::from_str(&text)?;

    Ok(response
        .results
        .into_iter()
        .take(10)
        .map(|item| {
            let poster_url = item
                .poster_path
                .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
                .unwrap_or_else(|| String::from("https://via.placeholder.com/500x750"));
            let tmdb_url = format!(
                "https://www.themoviedb.org/{}/{}",
                if item.media_type == "movie" {
                    "movie"
                } else {
                    "tv"
                },
                item.id
            );
            Release {
                id: item.id,
                title: item.title.or(item.name).unwrap_or_default(),
                release_date: item
                    .release_date
                    .or(item.first_air_date)
                    .unwrap_or_default(),
                media_type: item.media_type,
                vote_count: item.vote_count,
                vote_average: item.vote_average,
                poster_url,
                tmdb_url,
            }
        })
        .collect())
}

async fn refresh_releases(State(config): State<Arc<AppConfig>>) -> impl IntoResponse {
    match fetch_latest_releases(&config.tmdb_api_key).await {
        Ok(releases) => {
            let template = IndexTemplate { releases };
            match template.render() {
                Ok(html) => (StatusCode::OK, Html(html)).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Template rendering error: {}", e),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch releases: {}", e),
        )
            .into_response(),
    }
}

async fn index(State(config): State<Arc<AppConfig>>) -> Html<String> {
    match fetch_latest_releases(&config.tmdb_api_key).await {
        Ok(releases) => {
            let template = IndexTemplate { releases };
            match template.render() {
                Ok(html) => Html(html),
                Err(e) => Html(format!("Template rendering error: {}", e)),
            }
        }
        Err(e) => Html(format!("Failed to fetch releases: {}", e)),
    }
}

#[tokio::main]
async fn main() {
    let config = Arc::new(AppConfig {
        tmdb_api_key: std::env::var("TMDB_API_KEY").expect("TMDB_API_KEY not set"),
        jellyseerr_api_key: std::env::var("JELLYSEERR_API_KEY")
            .expect("JELLYSEERR_API_KEY not set"),
        jellyseerr_url: std::env::var("JELLYSEERR_URL").expect("JELLYSEERR_URL not set"),
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/api/request/{media_type}/{id}", post(add_to_jellyseerr))
        .route("/test-rate-limit", get(test_rate_limit))
        .route("/refresh", get(refresh_releases))
        .layer(RateLimitServiceLayer::new(10, 20))
        .with_state(config);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Server running on http://localhost:3000");

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service(),
    )
    .await
    .unwrap();
}
