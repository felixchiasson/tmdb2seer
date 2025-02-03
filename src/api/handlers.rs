use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    Json,
};
use chrono::{DateTime, Utc};
use http::{HeaderMap, Response, StatusCode};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info};

use super::{jellyseerr, tmdb};
use crate::security;
use crate::AppState;

#[derive(Deserialize)]
pub struct MediaRequest {
    seasons: Option<Vec<i32>>,
}

#[derive(Template)]
#[template(path = "../templates/index.html")]
struct IndexTemplate {
    releases: String,
    last_update: DateTime<Utc>,
    csrf_token: String,
}

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let releases = state.releases.read().await;
    let last_update = state.last_update.read().await;

    let releases_json = serde_json::to_string(&*releases).unwrap_or_else(|_| "[]".to_string());

    let template = IndexTemplate {
        releases: releases_json,
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
    Json(payload): Json<MediaRequest>,
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

    let seasons = if media_type == "tv" {
        payload.seasons
    } else {
        None
    };

    match jellyseerr::request_media(&state.config, id, &media_type, seasons).await {
        Ok(_) => {
            info!(
                "Successfully added media {}/{} to Jellyseerr",
                media_type, id
            );

            let mut releases = state.releases.write().await;
            releases.retain(|release| !(release.id == id && release.media_type == media_type));

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
    match tmdb::fetch_latest_releases(&state.config.tmdb_api_key, &state.config.omdb_api_key).await
    {
        Ok(new_releases) => {
            match jellyseerr::filter_requested_media(&state.config, new_releases).await {
                Ok(filtered_releases) => {
                    let response = Json(json!({
                        "success": true,
                        "releases": &filtered_releases,
                        "lastUpdate": Utc::now().to_rfc3339(),
                    }))
                    .into_response();

                    let mut releases = state.releases.write().await;
                    *releases = filtered_releases;
                    let mut last_update = state.last_update.write().await;
                    *last_update = Utc::now();

                    info!("Manual refresh successful");
                    response
                }
                Err(e) => {
                    error!("Failed to filter requested media: {}", e);
                    Json(json!({
                        "success": false,
                        "error": e.to_string()
                    }))
                    .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to manually refresh releases: {}", e);
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
            .into_response()
        }
    }
}

pub async fn hide_media(
    headers: HeaderMap,
    State(state): State<AppState>,
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

    let mut releases = state.releases.write().await;
    releases.retain(|release| !(release.id == id && release.media_type == media_type));

    info!("Hidden media {}/{} from view", media_type, id);

    Json(json!({
        "success": true,
        "message": "Media hidden successfully"
    }))
    .into_response()
}
