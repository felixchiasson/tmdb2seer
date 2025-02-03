use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("TMDB error: {0}")]
    TMDB(#[from] crate::api::tmdb::TMDBError),

    #[error("OMDB error: {0}")]
    OMDB(#[from] crate::api::omdb::OMDBError),

    #[error("Jellyseerr error: {0}")]
    Jellyseerr(#[from] crate::api::jellyseerr::JellyseerrError),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("CSRF error: {0}")]
    CSRF(String),
}

pub type Result<T> = std::result::Result<T, Error>;
