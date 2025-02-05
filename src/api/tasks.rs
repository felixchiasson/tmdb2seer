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

        // Fetch data outside of any locks
        let fetch_result = async {
            let new_releases = tmdb::fetch_latest_releases(&state.config).await?;
            jellyseerr::filter_requested_media(&state.config, new_releases).await
        }
        .await;

        match fetch_result {
            Ok(filtered_releases) => {
                // Minimize time holding both locks
                let mut releases = state.releases.write().await;
                let mut last_update = state.last_update.write().await;
                *releases = filtered_releases;
                *last_update = Utc::now();
                drop(last_update);
                drop(releases);
                info!("Successfully refreshed releases");
            }
            Err(e) => {
                error!("Failed to refresh releases: {}", e);
            }
        }
    }
}
