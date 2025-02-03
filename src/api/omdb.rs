use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OMDBResponse {
    #[serde(rename = "imdbRating")]
    pub imdb_rating: Option<String>,
    #[serde(rename = "Metascore")]
    pub metascore: Option<String>,
    #[serde(rename = "Ratings")]
    pub ratings: Option<Vec<Rating>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Rating {
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Value")]
    pub value: String,
}

#[derive(Debug)]
pub enum OMDBError {
    Request(reqwest::Error),
    Parse(serde_json::Error),
    Other(String),
}

impl OMDBResponse {
    fn clean_rating(rating: &Option<String>) -> Option<String> {
        rating.as_ref().and_then(|r| {
            if r == "N/A" {
                None
            } else {
                Some(r.to_string())
            }
        })
    }

    pub fn get_imdb_rating(&self) -> Option<String> {
        Self::clean_rating(&self.imdb_rating)
    }

    pub fn get_metascore(&self) -> Option<String> {
        Self::clean_rating(&self.metascore)
    }
}

pub async fn fetch_ratings(
    api_key: &Secret<String>,
    title: &str,
    year: &str,
) -> Result<OMDBResponse, OMDBError> {
    // Check cache first
    if let Some(cached) = crate::api::cache::get_cached_omdb_rating(title, year) {
        debug!("Cache hit for OMDB: {} ({})", title, year);
        return Ok(cached);
    }

    debug!(
        "Cache miss for OMDB, fetching from API: {} ({})",
        title, year
    );

    let client = reqwest::Client::new();
    let url = format!(
        "http://www.omdbapi.com/?apikey={}&t={}&y={}",
        api_key.expose_secret(),
        urlencoding::encode(title),
        year
    );

    debug!("Fetching OMDB data for: {} ({})", title, year);

    let response = client.get(url).send().await.map_err(OMDBError::Request)?;

    let response_text = response.text().await.map_err(OMDBError::Request)?;

    let data: OMDBResponse = serde_json::from_str(&response_text).map_err(OMDBError::Parse)?;

    let cleaned_data = OMDBResponse {
        imdb_rating: data.get_imdb_rating(),
        metascore: data.get_metascore(),
        ratings: data.ratings,
    };

    crate::api::cache::cache_omdb_rating(title, year, cleaned_data.clone());

    Ok(cleaned_data)
}
