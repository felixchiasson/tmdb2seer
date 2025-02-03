use once_cell::sync::Lazy;
use reqwest::{Client, Response};
use secrecy::{ExposeSecret, Secret};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;
use tracing::{debug, error};

use crate::error::{Error, Result};

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Request(err.to_string())
    }
}

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
});

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: &'static Client,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: &HTTP_CLIENT,
        }
    }

    // Generic GET request with query parameters
    pub async fn get<T: DeserializeOwned>(&self, url: &str) -> Result<T> {
        debug!("Making GET request to: {}", url);

        let response = self
            .client
            .get(url)
            .header("accept", "application/json")
            .send()
            .await?;

        self.handle_response(response).await
    }

    // Generic POST request with body
    pub async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        url: &str,
        body: &T,
        api_key: Option<&Secret<String>>,
    ) -> Result<R> {
        debug!("Making POST request to: {}", url);

        let mut request = self
            .client
            .post(url)
            .header("accept", "application/json")
            .json(body);

        // Add API key header if provided
        if let Some(key) = api_key {
            request = request.header("X-Api-Key", key.expose_secret());
        }

        let response = request.send().await?;

        self.handle_response(response).await
    }

    // Helper for TMDB specific requests
    pub async fn tmdb_get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        api_key: &Secret<String>,
    ) -> Result<T> {
        let separator = if endpoint.contains('?') { "&" } else { "?" };
        let url = format!(
            "https://api.themoviedb.org/3/{}{}{}&language=en-US",
            endpoint,
            separator,
            format!("api_key={}", api_key.expose_secret())
        );
        debug!(
            "Making TMDB request to: {}",
            url.replace(api_key.expose_secret(), "API_KEY")
        );
        self.get(&url).await
    }

    // Helper for OMDB specific requests
    pub async fn omdb_get<T: DeserializeOwned>(
        &self,
        title: &str,
        year: &str,
        api_key: &Secret<String>,
    ) -> Result<T> {
        let url = format!(
            "http://www.omdbapi.com/?apikey={}&t={}&y={}",
            api_key.expose_secret(),
            urlencoding::encode(title),
            year
        );
        self.get(&url).await
    }

    pub async fn jellyseerr_get<R>(
        &self,
        endpoint: &str,
        api_key: &Secret<String>,
        base_url: &str,
    ) -> Result<R>
    where
        R: DeserializeOwned,
    {
        let url = format!("{}/api/v1/{}", base_url, endpoint);

        let response = self
            .client
            .get(&url)
            .header("accept", "application/json")
            .header("X-Api-Key", api_key.expose_secret())
            .send()
            .await?;

        self.handle_response(response).await
    }

    // Helper for Jellyseerr specific requests
    pub async fn jellyseerr_post<T: Serialize, R: DeserializeOwned>(
        &self,
        endpoint: &str,
        body: &T,
        api_key: &Secret<String>,
        base_url: &str,
    ) -> Result<R> {
        let url = format!("{}/api/v1/{}", base_url, endpoint);
        self.post(&url, body, Some(api_key)).await
    }

    // Generic response handler
    async fn handle_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        match response.status() {
            status if status.is_success() => {
                // This checks for any 2xx status code
                let text = response.text().await?;

                serde_json::from_str(&text).map_err(|e| {
                    error!("Failed to parse response: {}", e);
                    Error::Parse(format!("Failed to parse response: {}", e))
                })
            }
            status => {
                let error_text = response.text().await.unwrap_or_default();
                error!("Request failed with status {}: {}", status, error_text);
                Err(Error::Api(format!(
                    "Request failed: {} - {}",
                    status, error_text
                )))
            }
        }
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}
