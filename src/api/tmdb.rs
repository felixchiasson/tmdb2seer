use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::{debug, error};

#[derive(Debug, Deserialize)]
pub struct TMDBResult {
    pub id: i32,
    pub title: Option<String>,
    pub name: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    #[serde(rename = "media_type")]
    pub media_type: String,
    pub vote_average: f32,
    pub vote_count: i32,
    pub poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TMDBResponse {
    pub results: Vec<TMDBResult>,
}

#[derive(Debug, Serialize)]
pub struct Release {
    pub id: i32,
    pub title: String,
    pub release_date: String,
    pub media_type: String,
    pub vote_average: f32,
    pub vote_count: i32,
    pub poster_url: String,
    pub tmdb_url: String,
}

pub async fn fetch_latest_releases(
    api_key: &Secret<String>,
) -> Result<Vec<Release>, Box<dyn Error>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.themoviedb.org/3/trending/all/week?api_key={}",
        api_key.expose_secret()
    );

    debug!(
        "Requesting URL: {}",
        url.replace(api_key.expose_secret(), "API_KEY")
    );

    let response = client
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(|e| {
            error!("TMDB request failed: {}", e);
            e
        })?;

    let status = response.status();
    debug!("TMDB response status: {}", status);

    if !status.is_success() {
        error!("TMDB request failed with status: {}", status);
        return Err(format!("TMDB request failed: {}", status).into());
    }

    let text = response.text().await.map_err(|e| {
        error!("Failed to get response text: {}", e);
        e
    })?;
    let response: TMDBResponse = serde_json::from_str(&text).map_err(|e| {
        error!("Failed to parse TMDB response: {}", e);
        e
    })?;

    debug!(
        "Successfully fetched {} TMDB results",
        response.results.len()
    );

    Ok(response
        .results
        .into_iter()
        .take(10)
        .map(|item| {
            let poster_url = item
                .poster_path
                .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
                .unwrap_or_else(|| String::from("https://via.placeholder.com/500x750"));
            let tmdb_url = format!(
                "https://www.themoviedb.org/{}/{}",
                if item.media_type == "movie" {
                    "movie"
                } else {
                    "tv"
                },
                item.id
            );
            Release {
                id: item.id,
                title: item.title.or(item.name).unwrap_or_default(),
                release_date: item
                    .release_date
                    .or(item.first_air_date)
                    .unwrap_or_default(),
                media_type: item.media_type,
                vote_count: item.vote_count,
                vote_average: item.vote_average,
                poster_url,
                tmdb_url,
            }
        })
        .collect())
}
