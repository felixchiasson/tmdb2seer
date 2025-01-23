pub mod api {
    pub mod handlers;
    pub mod jellyseerr;
    pub mod middleware;
    pub mod rate_limiter;
    pub mod tasks;
    pub mod tmdb;
}
pub mod config {
    pub mod settings;
}
pub mod security {
    pub mod csrf;
    pub mod deserialize;
}
use crate::api::tmdb::Release;
use crate::config::settings::Settings;
use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, Secret};
use std::sync::Arc;
use tokio::sync::RwLock;

extern crate askama;
use askama::Template;

#[derive(Clone)]
pub struct AppConfig {
    pub tmdb_api_key: Secret<String>,
    pub jellyseerr_api_key: Secret<String>,
    pub jellyseerr_url: String,
    pub rate_limit: RateLimitConfig,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub releases: Arc<RwLock<Vec<Release>>>,
    pub last_update: Arc<RwLock<DateTime<Utc>>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(config),
            releases: Arc::new(RwLock::new(Vec::new())),
            last_update: Arc::new(RwLock::new(Utc::now())),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn init_config() -> AppResult<AppConfig> {
    let settings = Settings::new()
        .map_err(|e| AppError::Config(e.to_string()))?
        .validate()
        .map_err(AppError::Config)?;

    Ok(AppConfig {
        tmdb_api_key: settings.tmdb.api_key,
        jellyseerr_api_key: settings.jellyseerr.api_key,
        jellyseerr_url: settings.jellyseerr.url.clone(),
        rate_limit: RateLimitConfig {
            requests_per_second: settings.rate_limit.requests_per_second,
            burst_size: settings.rate_limit.burst_size,
        },
    })
}

pub fn init_router(config: AppState) -> axum::Router {
    use crate::api::{handlers, middleware::RateLimitServiceLayer};
    use axum::{
        routing::{get, post},
        Router,
    };

    Router::new()
        .route("/", get(handlers::index))
        .route(
            "/api/request/{media_type}/{id}",
            post(handlers::add_to_jellyseerr),
        )
        .layer(RateLimitServiceLayer::new(
            config.config.rate_limit.requests_per_second,
            config.config.rate_limit.burst_size,
        ))
        .with_state(config)
}
