use crate::security::csrf::validate_csrf_token;
use crate::{Error, Result};
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use http::{HeaderMap, StatusCode};
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
    // Handle all potential errors and convert to IntoResponse
    match process_media_request(headers, &state, &media_type, id, payload).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error adding to Jellyseerr: {}", e);
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
            .into_response()
        }
    }
}

async fn process_media_request(
    headers: HeaderMap,
    state: &AppState,
    media_type: &str,
    id: i32,
    payload: MediaRequest,
) -> Result<Response> {
    validate_csrf_token(&headers)?;

    let seasons = match media_type {
        "tv" => payload.seasons,
        "movie" => None,
        _ => return Err(Error::Config(format!("Invalid media type: {}", media_type))),
    };

    jellyseerr::request_media(&state.config, id, media_type, seasons).await?;

    let mut releases = state.releases.write().await;
    releases.retain(|release| !(release.id == id && release.media_type == media_type));

    Ok(Json(json!({
        "success": true,
        "message": format!("{} requested successfully",
            if media_type == "tv" { "TV Show" } else { "Movie" })
    }))
    .into_response())
}

pub async fn refresh(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    match update_release_list(headers, &state).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error refreshing: {}", e);
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
            .into_response()
        }
    }
}

async fn update_release_list(headers: HeaderMap, state: &AppState) -> Result<Response> {
    validate_csrf_token(&headers)?;

    info!("Manual refresh triggered");

    let new_releases =
        tmdb::fetch_latest_releases(&state.config.tmdb_api_key, &state.config.omdb_api_key).await?;

    let filtered_releases = jellyseerr::filter_requested_media(&state.config, new_releases).await?;

    let mut releases = state.releases.write().await;
    *releases = filtered_releases.clone();
    let mut last_update = state.last_update.write().await;
    *last_update = Utc::now();

    info!("Manual refresh successful");

    Ok(Json(json!({
        "success": true,
        "releases": filtered_releases,
        "lastUpdate": Utc::now().to_rfc3339(),
    }))
    .into_response())
}

pub async fn hide_media(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path((media_type, id)): Path<(String, i32)>,
) -> impl IntoResponse {
    match remove_media_from_view(headers, &state, &media_type, id).await {
        Ok(response) => response,
        Err(e) => {
            error!("Error hiding media: {}", e);
            Json(json!({
                "success": false,
                "error": e.to_string()
            }))
            .into_response()
        }
    }
}

async fn remove_media_from_view(
    headers: HeaderMap,
    state: &AppState,
    media_type: &str,
    id: i32,
) -> Result<Response> {
    validate_csrf_token(&headers)?;

    let mut releases = state.releases.write().await;
    releases.retain(|release| !(release.id == id && release.media_type == media_type));

    info!("Hidden media {}/{} from view", media_type, id);

    Ok(Json(json!({
        "success": true,
        "message": "Media hidden successfully"
    }))
    .into_response())
}
