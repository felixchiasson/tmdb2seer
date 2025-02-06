use crate::config::settings::Settings;
use crate::Error;
use crate::{api, AppState, Result};
use axum::Router;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tracing::{debug, error, Level};

pub fn load_settings() -> Result<Settings> {
    Settings::new()
        .map_err(|e| {
            error!("Failed to load settings: {}", e);
            crate::Error::Config(e.to_string())
        })?
        .validate()
        .map_err(crate::Error::Config)
}

pub fn init_logging(settings: &Settings) {
    let level = if settings.is_development() {
        Level::DEBUG
    } else {
        Level::INFO
    };
    tracing_subscriber::fmt().with_max_level(level).init();
}

pub async fn setup_background_tasks(state: &AppState, settings: &Settings) -> Result<()> {
    let background_state = state.clone();
    let refresh_interval = Duration::from_secs(settings.tmdb.refresh_interval);

    tokio::spawn(async move {
        api::tasks::refresh_releases(background_state, refresh_interval).await;
    });

    if let Err(e) = api::tmdb::fetch_latest_releases(&state.config).await {
        error!("Failed initial fetch of latest releases: {}", e);
    }

    Ok(())
}

pub fn get_socket_addr(settings: &Settings) -> Result<SocketAddr> {
    let ip_addr = settings.server.host.parse::<IpAddr>().map_err(|e| {
        error!("Failed to parse host address: {}", e);
        crate::Error::Config(format!("Invalid host address: {}", e))
    })?;

    Ok(SocketAddr::new(ip_addr, settings.server.port))
}

pub async fn setup_server(app: Router, addr: SocketAddr) -> Result<()> {
    let server = axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            debug!("Received shutdown signal");
            if let Err(e) = crate::api::cache::save_cache().await {
                error!("Failed to save cache: {}", e);
            }
            Ok(())
        }
        result = server => {
            result.map_err(|e| Error::Config(format!("Server error: {}", e)))
        }
    }
}
