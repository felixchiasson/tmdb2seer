use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    Json,
};
use chrono::{DateTime, Utc};
use http::{HeaderMap, Response, StatusCode};
use serde_json::json;
use tracing::{error, info};

use super::tmdb::Release;
use super::{jellyseerr, tmdb};
use crate::security;
use crate::AppState;

#[derive(Template)]
#[template(path = "../templates/index.html")]
struct IndexTemplate {
    releases: Vec<Release>,
    last_update: DateTime<Utc>,
    csrf_token: String,
}

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let releases = state.releases.read().await;
    let last_update = state.last_update.read().await;

    let template = IndexTemplate {
        releases: releases.clone(),
        last_update: *last_update,
        csrf_token: security::csrf::generate_csrf_token(),
    };

    match template.render() {
        Ok(html) => Html(html),
        Err(e) => {
            error!("Failed to render template: {}", e);
            Html("Template Renderer Error".to_string())
        }
    }
}

pub async fn add_to_jellyseerr(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((media_type, id)): Path<(String, i32)>,
) -> impl IntoResponse {
    info!("Received request for {}/{}", media_type, id);

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

    match jellyseerr::request_media(&state.config, id, &media_type).await {
        Ok(_) => {
            info!(
                "Successfully added media {}/{} to Jellyseerr",
                media_type, id
            );
            Json(json!({
                "success": true,
                "message": "Media requested successfully"
            }))
            .into_response()
        }
        Err(e) => {
            error!("Error requesting media: {}", e);
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
            .into_response()
        }
    }
}

pub async fn refresh(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
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

    info!("Manual refresh triggered");
    match tmdb::fetch_latest_releases(&state.config.tmdb_api_key).await {
        Ok(new_releases) => {
            let mut releases = state.releases.write().await;
            *releases = new_releases.clone();

            let mut last_update = state.last_update.write().await;
            *last_update = Utc::now();

            info!("Manual refresh successful");
            Json(json!({
                "success": true,
                "releases": new_releases,
                "lastUpdate": Utc::now().to_rfc3339(),
            }))
            .into_response()
        }
        Err(e) => {
            error!("Failed to manually refresh releases: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
