use std::sync::Arc;
use tracing::Level;
use tracing::{error, info};
use tvdb_ratings::{init_config, init_router, AppResult};

#[tokio::main]
async fn main() -> AppResult<()> {
    let settings = tvdb_ratings::config::settings::Settings::new()
        .map_err(|e| {
            error!("Failed to load settings: {}", e);
            tvdb_ratings::AppError::Config(e.to_string())
        })?
        .validate()
        .map_err(tvdb_ratings::AppError::Config)?;

    let level = if settings.is_development() {
        Level::DEBUG
    } else {
        Level::INFO
    };

    tracing_subscriber::fmt().with_max_level(level).init();

    let config = Arc::new(init_config()?);

    let app = init_router(config.clone());

    let addr = std::net::SocketAddr::from((
        settings
            .server
            .host
            .parse::<std::net::IpAddr>()
            .map_err(|e| {
                error!("Failed to parse host address: {}", e);
                tvdb_ratings::AppError::Config(format!("Invalid host address: {}", e))
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
            tvdb_ratings::AppError::Internal(e.to_string())
        })?,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}
