use std::sync::Arc;
use std::time::Duration;
use tmdb2seer::{api, init_config, init_router, AppResult, AppState};
use tracing::Level;
use tracing::{error, info};

#[tokio::main]
async fn main() -> AppResult<()> {
    let settings = tmdb2seer::config::settings::Settings::new()
        .map_err(|e| {
            error!("Failed to load settings: {}", e);
            tmdb2seer::AppError::Config(e.to_string())
        })?
        .validate()
        .map_err(tmdb2seer::AppError::Config)?;

    let level = if settings.is_development() {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(level).init();

    let config = init_config()?;
    let state = AppState::new(config);

    let background_state = state.clone();
    let refresh_interval = Duration::from_secs(settings.tmdb.refresh_interval);

    // Background task refresh
    tokio::spawn(async move {
        api::tasks::refresh_releases(background_state, refresh_interval).await;
    });

    if let Err(e) = api::tmdb::fetch_latest_releases(&state.config.tmdb_api_key).await {
        error!("Failed initial fetch of latest releases: {}", e);
    }

    let app = init_router(state);

    let addr = std::net::SocketAddr::from((
        settings
            .server
            .host
            .parse::<std::net::IpAddr>()
            .map_err(|e| {
                error!("Failed to parse host address: {}", e);
                tmdb2seer::AppError::Config(format!("Invalid host address: {}", e))
            })?,
        settings.server.port,
    ));

    info!(
        "Server running on http://{}:{} in {} mode",
        settings.server.host,
        settings.server.port,
        if settings.is_development() {
            "development"
        } else {
            "production"
        }
    );

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            error!("Failed to bind to address: {}", e);
            tmdb2seer::AppError::Internal(e.to_string())
        })?,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}
