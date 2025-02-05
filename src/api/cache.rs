use crate::api::omdb::OMDBResponse;
use crate::api::tmdb::TVShowDetails;
use crate::utils::serde::timestamp;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tracing::{debug, error};

const MAX_CACHE_SIZE: usize = 1000;
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedItem<T> {
    pub data: T,
    #[serde(with = "timestamp")]
    pub timestamp: Instant,
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    tv_details: Vec<(i32, CachedItem<TVShowDetails>)>,
    omdb_ratings: Vec<(String, CachedItem<OMDBResponse>)>,
}

pub enum CacheType {
    TVDetails,
    OMDBRatings,
}

struct CacheManager {
    tv_details: DashMap<i32, CachedItem<TVShowDetails>>,
    omdb_ratings: DashMap<String, CachedItem<OMDBResponse>>,
}

static CACHE: OnceLock<CacheManager> = OnceLock::new();

fn get_cache() -> &'static CacheManager {
    CACHE.get_or_init(|| CacheManager::load_cache_from_disk())
}

impl CacheManager {
    pub fn get_tv_details(&self, id: i32) -> Option<CachedItem<TVShowDetails>> {
        if let Some(item) = self.tv_details.get(&id) {
            if item.timestamp.elapsed() < CACHE_TTL {
                return Some(item.clone());
            }
            self.tv_details.remove(&id);
        }
        None
    }

    pub fn get_omdb_rating(&self, key: &str) -> Option<CachedItem<OMDBResponse>> {
        if let Some(item) = self.omdb_ratings.get(key) {
            if item.timestamp.elapsed() < CACHE_TTL {
                return Some(item.clone());
            }
            self.omdb_ratings.remove(key);
        }
        None
    }

    pub fn insert_tv_details(&self, id: i32, details: TVShowDetails) {
        self.tv_details.insert(
            id,
            CachedItem {
                data: details,
                timestamp: Instant::now(),
            },
        );
        self.cleanup(CacheType::TVDetails);
    }

    pub fn insert_omdb_rating(&self, key: String, rating: OMDBResponse) {
        self.omdb_ratings.insert(
            key,
            CachedItem {
                data: rating,
                timestamp: Instant::now(),
            },
        );
        self.cleanup(CacheType::OMDBRatings);
    }

    fn cleanup_cache<K: Clone + std::hash::Hash + Eq>(
        cache: &DashMap<K, CachedItem<impl Clone>>,
        cache_name: &str,
        remove_key: impl Fn(&K),
    ) {
        let current_size = cache.len();

        if current_size > MAX_CACHE_SIZE {
            debug!(
                "{} cache size ({}) exceeded limit ({}), cleaning up...",
                cache_name, current_size, MAX_CACHE_SIZE
            );

            let mut entries: Vec<_> = cache
                .iter()
                .map(|entry| (entry.key().clone(), entry.value().timestamp))
                .collect();

            entries.sort_by_key(|(_key, timestamp)| *timestamp);

            let entries_to_remove = current_size - (MAX_CACHE_SIZE * 3 / 4);
            for (key, _) in entries.iter().take(entries_to_remove) {
                remove_key(key);
            }

            debug!(
                "{} cache cleanup complete. New size: {}",
                cache_name,
                cache.len()
            );
        }
    }

    fn cleanup(&self, cache_type: CacheType) {
        match cache_type {
            CacheType::TVDetails => {
                Self::cleanup_cache(&self.tv_details, "TV Details", |key| {
                    self.tv_details.remove(key);
                });
            }
            CacheType::OMDBRatings => {
                Self::cleanup_cache(&self.omdb_ratings, "OMDB Ratings", |key| {
                    self.omdb_ratings.remove(key);
                });
            }
        }
        self.save_cache_to_disk();
    }

    fn load_cache_from_disk() -> Self {
        let cache_path = "cache/cache.json";
        let manager = CacheManager {
            tv_details: DashMap::new(),
            omdb_ratings: DashMap::new(),
        };

        if let Ok(cache_data) = fs::read_to_string(cache_path) {
            match serde_json::from_str::<CacheFile>(&cache_data) {
                Ok(cache_file) => {
                    for (id, item) in cache_file.tv_details {
                        if item.timestamp.elapsed() < CACHE_TTL {
                            manager.tv_details.insert(id, item);
                        }
                    }
                    for (key, item) in cache_file.omdb_ratings {
                        if item.timestamp.elapsed() < CACHE_TTL {
                            manager.omdb_ratings.insert(key, item);
                        }
                    }
                }
                Err(e) => {
                    error!("Error deserializing cache file: {}", e);
                }
            }
        }
        manager
    }

    fn save_cache_to_disk(&self) {
        let cache_dir = Path::new("cache");
        if !cache_dir.exists() {
            if let Err(e) = fs::create_dir(cache_dir) {
                error!("Failed to create cache directory: {}", e);
                return;
            }
        }

        let cache_file = CacheFile {
            tv_details: self
                .tv_details
                .iter()
                .map(|r| (*r.key(), r.value().clone()))
                .collect(),
            omdb_ratings: self
                .omdb_ratings
                .iter()
                .map(|r| (r.key().clone(), r.value().clone()))
                .collect(),
        };

        match serde_json::to_string_pretty(&cache_file) {
            Ok(json) => {
                if let Err(e) = fs::write("cache/cache.json", json) {
                    error!("Failed to write cache file: {}", e);
                } else {
                    debug!("Cache saved to disk successfully");
                }
            }
            Err(e) => {
                error!("Failed to serialize cache: {}", e);
            }
        }
    }
}

pub fn get_cached_tv_details(id: i32) -> Option<TVShowDetails> {
    get_cache().get_tv_details(id).map(|item| item.data)
}

pub fn get_cached_omdb_rating(title: &str, year: &str) -> Option<OMDBResponse> {
    let key = format!("{}_{}", title, year);
    get_cache().get_omdb_rating(&key).map(|item| item.data)
}

pub fn cache_tv_details(id: i32, details: TVShowDetails) {
    get_cache().insert_tv_details(id, details);
}

pub fn cache_omdb_rating(title: &str, year: &str, rating: OMDBResponse) {
    let key = format!("{}_{}", title, year);
    get_cache().insert_omdb_rating(key, rating);
}

pub fn save_cache() {
    if let Some(cache) = CACHE.get() {
        cache.save_cache_to_disk();
    }
}
