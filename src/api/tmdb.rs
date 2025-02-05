use crate::api::client::ApiClient;
use crate::api::omdb;
use crate::AppConfig;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use tracing::{debug, error};

#[derive(Debug, Deserialize, Clone)]
pub struct TMDBResult {
    pub id: i32,
    pub title: Option<String>,
    pub name: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    #[serde(skip)]
    pub media_type: String,
    pub vote_average: f32,
    pub vote_count: i32,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TVShowDetails {
    pub number_of_seasons: i32,
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
    pub overview: Option<String>,
    pub imdb_rating: Option<String>,
    pub metascore: Option<String>,
    pub rotten_tomatoes: Option<String>,
}

pub async fn fetch_latest_releases(config: &AppConfig) -> Result<Vec<Release>> {
    let client = ApiClient::new(&config);
    let mut all_releases = Vec::new();

    // Fetch movies
    let movie_data: TMDBResponse = client
            .tmdb_get(
                "discover/movie?sort_by=release_date.desc&with_watch_providers=8|9|337|1899|350|15|619|283&watch_region=US&vote_count.gte=1&vote_average.gte=1&page=1",
                &config.tmdb_api_key,
            )
            .await?;

    // Process movies
    for item in movie_data.results {
        let poster_url = item
            .poster_path
            .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
            .unwrap_or_else(|| String::from("https://via.placeholder.com/500x750"));

        let tmdb_url = format!("https://www.themoviedb.org/movie/{}", item.id);

        let year = item
            .release_date
            .as_ref()
            .and_then(|date| date.split('-').next())
            .unwrap_or("");

        let mut imdb_rating = None;
        let mut metascore = None;
        let mut rotten_tomatoes = None;

        if let Some(title) = &item.title {
            if let Ok(omdb_data) = omdb::fetch_ratings(&config, title, year).await {
                imdb_rating = omdb_data.imdb_rating;
                metascore = omdb_data.metascore;

                // Extract Rotten Tomatoes rating
                if let Some(ratings) = omdb_data.ratings {
                    for rating in ratings {
                        if rating.source == "Rotten Tomatoes" {
                            rotten_tomatoes = Some(rating.value.to_string());
                            break;
                        }
                    }
                }
            }
        }

        all_releases.push(Release {
            id: item.id,
            title: item.title.unwrap_or_default(),
            release_date: item.release_date.unwrap_or_default(),
            media_type: "movie".to_string(),
            vote_count: item.vote_count,
            vote_average: item.vote_average,
            poster_url,
            tmdb_url,
            number_of_seasons: None,
            overview: item.overview,
            imdb_rating,
            metascore,
            rotten_tomatoes,
        });
    }

    let tv_data: TMDBResponse = client
            .tmdb_get(
                "discover/tv?sort_by=first_air_date.desc&with_watch_providers=8|9|337|1899|350|15|619|283&watch_region=US&with_watch_monetization_types=flatrate&vote_count.gte=1&vote_average.gte=1&page=1",
                &config.tmdb_api_key,
            )
            .await?;

    // Create futures for both TV details and providers
    let mut tv_futures = Vec::new();
    for item in &tv_data.results {
        let config_tv = config.clone();
        let id = item.id;
        let tv_future = tokio::spawn(async move {
            let details = fetch_tv_details(&config_tv, id).await;
            (id, details)
        });
        tv_futures.push((id, item.clone(), tv_future));
    }

    // Process TV shows
    for (id, item, future) in tv_futures {
        match future.await {
            Ok((_, details_result)) => {
                let number_of_seasons = match details_result {
                    Ok(details) => Some(details.number_of_seasons),
                    Err(e) => {
                        error!("Failed to fetch TV details for {}: {}", id, e);
                        None
                    }
                };

                let poster_url = item
                    .poster_path
                    .map(|path| format!("https://image.tmdb.org/t/p/w500{}", path))
                    .unwrap_or_else(|| String::from("https://via.placeholder.com/500x750"));

                let tmdb_url = format!("https://www.themoviedb.org/tv/{}", item.id);

                all_releases.push(Release {
                    id: item.id,
                    title: item.name.unwrap_or_default(),
                    release_date: item.first_air_date.unwrap_or_default(),
                    media_type: "tv".to_string(),
                    vote_count: item.vote_count,
                    vote_average: item.vote_average,
                    poster_url,
                    tmdb_url,
                    number_of_seasons,
                    overview: item.overview,
                    imdb_rating: None,
                    metascore: None,
                    rotten_tomatoes: None,
                });
            }
            Err(e) => {
                error!("Failed to join TV future: {}", e);
            }
        }
    }

    // Sort all releases by release date (newest first)
    all_releases.sort_by(|a, b| b.release_date.cmp(&a.release_date));
    debug!("Final releases with providers: {:#?}", all_releases);
    Ok(all_releases)
}

pub async fn fetch_tv_details(config: &AppConfig, tv_id: i32) -> Result<TVShowDetails> {
    if let Some(cached) = crate::api::cache::get_cached_tv_details(tv_id).await {
        debug!("Cache hit for TV details: {}", tv_id);
        return Ok(cached);
    }

    debug!("Cache miss for TV show {}, fetching from API", tv_id);

    let client = ApiClient::new(&config);
    let details: TVShowDetails = client
        .tmdb_get(&format!("tv/{}", tv_id), &config.tmdb_api_key)
        .await?;

    crate::api::cache::cache_tv_details(tv_id, details.clone()).await;

    Ok(details)
}
