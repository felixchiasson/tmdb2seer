use std::time::Duration;
use tmdb2seer::{api, init_config, init_router, AppState, Result};
use tracing::{debug, Level};
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let settings = tmdb2seer::config::settings::Settings::new()
        .map_err(|e| {
            error!("Failed to load settings: {}", e);
            tmdb2seer::Error::Config(e.to_string())
        })?
        .validate()
        .map_err(tmdb2seer::Error::Config)?;

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

    if let Err(e) =
        api::tmdb::fetch_latest_releases(&state.config.tmdb_api_key, &state.config.omdb_api_key)
            .await
    {
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
                tmdb2seer::Error::Config(format!("Invalid host address: {}", e))
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

    let save_cache = async {
        tokio::signal::ctrl_c().await.unwrap();
        debug!("Saving cache before exit...");
        crate::api::cache::save_cache();
        std::process::exit(0);
    };

    tokio::select! {
        _ = save_cache => {},
        _ = axum::serve(
            tokio::net::TcpListener::bind(addr).await?,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        ) => {},
    }

    Ok(())
}
