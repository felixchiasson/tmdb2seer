use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct TMDBConfig {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct JellyseerrConfig {
    pub api_key: String,
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
pub struct Settings {
    pub tmdb: TMDBConfig,
    pub jellyseerr: JellyseerrConfig,
    pub server: ServerConfig,
    pub rate_limit: RateLimitConfig,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            .add_source(File::with_name("config/default"))
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        s.try_deserialize()
    }

    pub fn validate(self) -> Result<Self, String> {
        if self.tmdb.api_key.is_empty() {
            return Err("TMDB API key is required".into());
        }
        if self.jellyseerr.api_key.is_empty() {
            return Err("Jellyseerr API key is required".into());
        }
        Ok(self)
    }

    pub fn is_development(&self) -> bool {
        env::var("RUN_MODE").unwrap_or_else(|_| "development".into()) == "development"
    }
}
