use tmdb2seer::{config, init_config, init_router, AppState, Result};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let settings = config::setup::load_settings()?;
    config::setup::init_logging(&settings);

    let config = init_config()?;
    let state = AppState::new(config);

    config::setup::setup_background_tasks(&state, &settings).await?;

    let app = init_router(state.clone());
    let addr = config::setup::get_socket_addr(&settings)?;

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

    config::setup::setup_server(app, addr).await?;

    Ok(())
}
