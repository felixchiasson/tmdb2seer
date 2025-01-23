use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
};
use http::{HeaderMap, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, error, info};

use super::tmdb::Release;
use super::{jellyseerr, tmdb};
use crate::security;
use crate::AppConfig;

#[derive(Template)]
#[template(path = "../templates/index.html")]
struct IndexTemplate {
    releases: Vec<Release>,
    csrf_token: String,
}

pub async fn index(State(config): State<Arc<AppConfig>>) -> Html<String> {
    match tmdb::fetch_latest_releases(&config.tmdb_api_key).await {
        Ok(releases) => {
            debug!("Successfully fetched {} releases", releases.len());
            let template = IndexTemplate {
                releases,
                csrf_token: security::csrf::generate_csrf_token(),
            };
            match template.render() {
                Ok(html) => Html(html),
                Err(e) => {
                    error!("Template rendering error: {}", e);
                    Html(format!("Template rendering error: {}", e))
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch releases: {}", e);
            Html(format!("Failed to fetch releases: {}", e))
        }
    }
}

pub async fn add_to_jellyseerr(
    headers: HeaderMap,
    State(config): State<Arc<AppConfig>>,
    Path((media_type, id)): Path<(String, i32)>,
) -> impl IntoResponse {
    if let Some(token) = headers.get("X-CSRF-Token") {
        if token.is_empty() {
            error!("Empty CSRF token received");
            return Response::builder()
                .status(StatusCode::FORBIDDEN)
                .body(axum::body::Body::from("Empty CSRF token"))
                .unwrap();
        }
    } else {
        error!("Missing CSRF token");
        return Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(axum::body::Body::from("Missing CSRF token"))
            .unwrap();
    }

    match jellyseerr::request_media(&config, id, &media_type).await {
        Ok(_) => {
            info!(
                "Successfully added media {}/{} to Jellyseerr",
                media_type, id
            );
            StatusCode::OK.into_response()
        }
        Err(e) => {
            error!("Error requesting media: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::from("Internal Server Error"))
                .unwrap()
        }
    }
}
