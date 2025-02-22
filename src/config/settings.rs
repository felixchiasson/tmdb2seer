use config::{Config, ConfigError, Environment, File};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use std::env;
use tracing::warn;

use crate::security::deserialize::deserialize_secret_string;

#[derive(Debug, Deserialize)]
pub struct TMDBConfig {
    #[serde(deserialize_with = "deserialize_secret_string")]
    pub api_key: Secret<String>,
    pub refresh_interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct JellyseerrConfig {
    #[serde(deserialize_with = "deserialize_secret_string")]
    pub api_key: Secret<String>,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct OMDBConfig {
    #[serde(deserialize_with = "deserialize_secret_string")]
    pub api_key: Secret<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 500,
            max_delay_ms: 5000,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub tmdb: TMDBConfig,
    pub jellyseerr: JellyseerrConfig,
    pub server: ServerConfig,
    pub rate_limit: RateLimitConfig,
    pub omdb: OMDBConfig,
    pub retry: RetryConfig,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| {
            warn!("No RUN_MODE set, defaulting to development");
            "development".into()
        });

        let final_config = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        final_config.try_deserialize()
    }

    pub fn validate(self) -> Result<Self, String> {
        if self.tmdb.api_key.expose_secret().is_empty() {
            return Err("TMDB API key is required".into());
        }
        if self.jellyseerr.api_key.expose_secret().is_empty() {
            return Err("Jellyseerr API key is required".into());
        }
        Ok(self)
    }

    pub fn is_development(&self) -> bool {
        env::var("RUN_MODE").unwrap_or_else(|_| "development".into()) == "development"
    }
}
