use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
};
use chrono::{DateTime, Utc};
use http::{HeaderMap, Response, StatusCode};
use std::sync::Arc;
use tracing::{debug, error, info};

use super::tmdb::Release;
use super::{jellyseerr, tmdb};
use crate::security;
use crate::{AppConfig, AppState};

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
