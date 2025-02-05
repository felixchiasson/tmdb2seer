use chrono::Utc;
use tokio::time::{interval, Duration};
use tracing::{error, info};

use super::tmdb;
use crate::api::jellyseerr;
use crate::AppState;

pub async fn refresh_releases(state: AppState, refresh_interval: Duration) {
    let mut interval = interval(refresh_interval);

    loop {
        interval.tick().await;
        info!("Refreshing release");

        match tmdb::fetch_latest_releases(&state.config).await {
            Ok(new_releases) => {
                match jellyseerr::filter_requested_media(&state.config, new_releases).await {
                    Ok(filtered_releases) => {
                        let mut releases = state.releases.write().await;
                        *releases = filtered_releases;
                        let mut last_update = state.last_update.write().await;
                        *last_update = Utc::now();

                        info!("Successfully refreshed releases");
                    }
                    Err(e) => {
                        error!("Failed to filter requested media: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to refresh releases: {}", e);
            }
        }
    }
}
