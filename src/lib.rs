pub mod api {
    pub mod cache;
    pub mod client;
    pub mod handlers;
    pub mod jellyseerr;
    pub mod middleware;
    pub mod omdb;
    pub mod rate_limiter;
    pub mod tasks;
    pub mod tmdb;
}
pub mod config {
    pub mod settings;
    pub mod setup;
}
pub mod security {
    pub mod csrf;
    pub mod deserialize;
    pub mod headers;
}
pub mod utils {
    pub mod serde;
}

mod error;
pub use config::settings::RetryConfig;
pub use error::{Error, Result};

pub use api::tmdb::Release;
use chrono::{DateTime, Utc};
pub use config::settings::Settings;
use secrecy::Secret;
use serde_json;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppConfig {
    pub tmdb_api_key: Secret<String>,
    pub jellyseerr_api_key: Secret<String>,
    pub jellyseerr_url: String,
    pub rate_limit: RateLimitConfig,
    pub omdb_api_key: Secret<String>,
    pub retry: RetryConfig,
}

#[derive(Clone)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
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

pub fn init_config() -> Result<AppConfig> {
    let settings = Settings::new()
        .map_err(|e| Error::Config(e.to_string()))?
        .validate()
        .map_err(Error::Config)?;

    Ok(AppConfig {
        tmdb_api_key: settings.tmdb.api_key,
        jellyseerr_api_key: settings.jellyseerr.api_key,
        jellyseerr_url: settings.jellyseerr.url.clone(),
        rate_limit: RateLimitConfig {
            requests_per_second: settings.rate_limit.requests_per_second,
            burst_size: settings.rate_limit.burst_size,
        },
        omdb_api_key: settings.omdb.api_key,
        retry: settings.retry,
    })
}

pub fn init_router(state: AppState) -> axum::Router {
    use crate::api::{handlers, middleware::RateLimitServiceLayer};
    use crate::security::headers::SecurityHeadersLayer;
    use axum::{
        routing::{get, post},
        Router,
    };

    let static_service = ServeDir::new("static");

    let api_router = Router::new()
        .route("/refresh", post(handlers::refresh))
        .route(
            "/request/{media_type}/{id}",
            post(handlers::add_to_jellyseerr),
        )
        .route("/hide/{media_type}/{id}", post(handlers::hide_media))
        .layer(RateLimitServiceLayer::new(
            state.config.rate_limit.requests_per_second,
            state.config.rate_limit.burst_size,
        ));

    Router::new()
        .route("/", get(handlers::index))
        .nest("/api", api_router)
        .nest_service("/static", static_service)
        .layer(SecurityHeadersLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub fn init_static_files() -> axum::Router {
    use axum::Router;
    let static_dir = if cfg!(debug_assertions) {
        "static"
    } else {
        "dist/static"
    };

    Router::new().nest_service("/static", ServeDir::new(static_dir))
}

pub fn get_template_path() -> String {
    if cfg!(debug_assertions) {
        "templates".to_string()
    } else {
        "dist/templates".to_string()
    }
}

pub fn json_encode<T: serde::Serialize>(value: &T) -> askama::Result<String> {
    serde_json::to_string(value).map_err(|_| askama::Error::Fmt(std::fmt::Error))
}
