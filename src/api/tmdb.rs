use dashmap::DashMap;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
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

#[derive(Debug)]
pub enum TMDBError {
    Request(reqwest::Error),
    Parse(serde_json::Error),
    Other(String),
}

impl std::error::Error for TMDBError {}

impl fmt::Display for TMDBError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TMDBError::Request(e) => write!(f, "Request (reqwest) error: {}", e),
            TMDBError::Parse(e) => write!(f, "Parse (serde) error: {}", e),
            TMDBError::Other(e) => write!(f, "Error: {}", e),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct TVShowDetails {
    pub number_of_seasons: i32,
}

#[derive(Clone)]
struct CachedTVDetails {
    details: TVShowDetails,
    timestamp: Instant,
}

#[derive(Debug, Serialize, Clone)]
pub struct Release {
    pub id: i32,
    pub title: String,
    pub release_date: String,
    pub media_type: String,
    pub vote_average: f32,
    pub vote_count: i32,
    pub poster_url: String,
    pub tmdb_url: String,
    pub number_of_seasons: Option<i32>,
}

static TV_CACHE: OnceLock<DashMap<i32, CachedTVDetails>> = OnceLock::new();
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub async fn fetch_latest_releases(api_key: &Secret<String>) -> Result<Vec<Release>, TMDBError> {
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
        .map_err(TMDBError::Request)?;

    let status = response.status();
    debug!("TMDB response status: {}", status);

    let text = response.text().await.map_err(TMDBError::Request)?;
    let response: TMDBResponse = serde_json::from_str(&text).map_err(TMDBError::Parse)?;

    debug!(
        "Successfully fetched {} TMDB results",
        response.results.len()
    );

    let mut tv_detail_futures = Vec::new();
    let mut releases = Vec::new();

    for item in response.results {
        if item.media_type == "tv" {
            let api_key_clone = api_key.clone();
            let tv_future = tokio::spawn(async move {
                let details = fetch_tv_details(&api_key_clone, item.id).await;
                (item, details)
            });
            tv_detail_futures.push(tv_future);
        } else {
            // For movies, create the release directly
            let poster_url = item
                .poster_path
                .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
                .unwrap_or_else(|| String::from("https://via.placeholder.com/500x750"));

            let tmdb_url = format!("https://www.themoviedb.org/movie/{}", item.id);

            releases.push(Release {
                id: item.id,
                title: item.title.unwrap_or_default(),
                release_date: item.release_date.unwrap_or_default(),
                media_type: item.media_type,
                vote_count: item.vote_count,
                vote_average: item.vote_average,
                poster_url,
                tmdb_url,
                number_of_seasons: None,
            });
        }
    }

    // Wait for all TV show detail requests to complete
    for tv_future in tv_detail_futures {
        match tv_future.await {
            Ok((item, details_result)) => {
                let number_of_seasons = match details_result {
                    Ok(details) => Some(details.number_of_seasons),
                    Err(e) => {
                        error!("Failed to fetch TV details for {}: {}", item.id, e);
                        None
                    }
                };

                let poster_url = item
                    .poster_path
                    .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
                    .unwrap_or_else(|| String::from("https://via.placeholder.com/500x750"));

                let tmdb_url = format!("https://www.themoviedb.org/tv/{}", item.id);

                releases.push(Release {
                    id: item.id,
                    title: item.name.unwrap_or_default(),
                    release_date: item.first_air_date.unwrap_or_default(),
                    media_type: item.media_type,
                    vote_count: item.vote_count,
                    vote_average: item.vote_average,
                    poster_url,
                    tmdb_url,
                    number_of_seasons,
                });
            }
            Err(e) => {
                error!("Failed to join TV details future: {}", e);
            }
        }
    }

    Ok(releases)
}

pub async fn fetch_tv_details(
    api_key: &Secret<String>,
    tv_id: i32,
) -> Result<TVShowDetails, TMDBError> {
    let cache = get_cache();

    if let Some(cached) = cache.get(&tv_id) {
        if cached.timestamp.elapsed() < CACHE_TTL {
            return Ok(cached.details.clone());
        }
    } else {
        debug!("Cache miss for TV ID {}", tv_id);
        cache.remove(&tv_id);
    }

    debug!("Cache miss for TV show {}, fetching from API", tv_id);
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.themoviedb.org/3/tv/{}?api_key={}",
        tv_id,
        api_key.expose_secret()
    );

    debug!(
        "Requesting TV details URL: {}",
        url.replace(api_key.expose_secret(), "API_KEY")
    );

    let response = client
        .get(&url)
        .header("accept", "application/json")
        .send()
        .await
        .map_err(TMDBError::Request)?;

    let text = response.text().await.map_err(TMDBError::Request)?;
    let details: TVShowDetails = serde_json::from_str(&text).map_err(TMDBError::Parse)?;

    cache.insert(
        tv_id,
        CachedTVDetails {
            details: details.clone(),
            timestamp: Instant::now(),
        },
    );

    Ok(details)
}

fn get_cache() -> &'static DashMap<i32, CachedTVDetails> {
    TV_CACHE.get_or_init(|| DashMap::new())
}
