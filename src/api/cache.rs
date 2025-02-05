use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error};

use crate::api::omdb::OMDBResponse;
use crate::api::tmdb::TVShowDetails;
use crate::Result;

const MAX_CACHE_SIZE: usize = 1000;
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const SAVE_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedItem<T> {
    pub data: T,
    #[serde(with = "crate::utils::serde::timestamp")]
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

pub(crate) struct CacheManager {
    tv_details: DashMap<i32, CachedItem<TVShowDetails>>,
    omdb_ratings: DashMap<String, CachedItem<OMDBResponse>>,
    last_save: RwLock<Instant>,
    is_saving: AtomicBool,
}

static CACHE: OnceLock<CacheManager> = OnceLock::new();

impl CacheManager {
    fn new() -> Self {
        Self {
            tv_details: DashMap::new(),
            omdb_ratings: DashMap::new(),
            last_save: RwLock::new(Instant::now()),
            is_saving: AtomicBool::new(false),
        }
    }

    fn get_tv_details(&self, id: i32) -> Option<CachedItem<TVShowDetails>> {
        if let Some(item) = self.tv_details.get(&id) {
            if item.timestamp.elapsed() < CACHE_TTL {
                return Some(item.clone());
            }
            self.tv_details.remove(&id);
        }
        None
    }

    fn get_omdb_rating(&self, key: &str) -> Option<CachedItem<OMDBResponse>> {
        if let Some(item) = self.omdb_ratings.get(key) {
            if item.timestamp.elapsed() < CACHE_TTL {
                return Some(item.clone());
            }
            self.omdb_ratings.remove(key);
        }
        None
    }

    fn insert_tv_details(&self, id: i32, details: TVShowDetails) {
        self.tv_details.insert(
            id,
            CachedItem {
                data: details,
                timestamp: Instant::now(),
            },
        );
        self.cleanup(CacheType::TVDetails);
    }

    fn insert_omdb_rating(&self, key: String, rating: OMDBResponse) {
        self.omdb_ratings.insert(
            key,
            CachedItem {
                data: rating,
                timestamp: Instant::now(),
            },
        );
        self.cleanup(CacheType::OMDBRatings);
    }

    async fn should_save(&self) -> bool {
        let last_save = *self.last_save.read().await;
        last_save.elapsed() >= SAVE_INTERVAL && !self.is_saving.load(Ordering::SeqCst)
    }

    fn cleanup(&self, cache_type: CacheType) {
        match cache_type {
            CacheType::TVDetails => self.cleanup_map(&self.tv_details, "TV Details"),
            CacheType::OMDBRatings => self.cleanup_map(&self.omdb_ratings, "OMDB Ratings"),
        }

        // Spawn a background task to handle saving if needed
        tokio::spawn(async {
            if let Some(cache) = CACHE.get() {
                if cache.should_save().await {
                    if let Err(e) = cache.save_cache_to_disk().await {
                        error!("Failed to save cache to disk: {}", e);
                    }
                }
            }
        });
    }

    fn cleanup_map<K: Clone + std::hash::Hash + Eq, V: Clone>(
        &self,
        cache: &DashMap<K, CachedItem<V>>,
        cache_name: &str,
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
                cache.remove(key);
            }

            debug!(
                "{} cache cleanup complete. New size: {}",
                cache_name,
                cache.len()
            );
        }
    }

    async fn save_cache_to_disk(&self) -> Result<()> {
        self.is_saving.store(true, Ordering::SeqCst);

        let result = async {
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

            let cache_path = PathBuf::from("cache");
            tokio::fs::create_dir_all(&cache_path).await?;

            let temp_path = cache_path.join("cache.json.tmp");
            let final_path = cache_path.join("cache.json");

            let json = serde_json::to_string_pretty(&cache_file)?;
            tokio::fs::write(&temp_path, &json).await?;
            tokio::fs::rename(temp_path, final_path).await?;

            // Update last_save time after successful save
            *self.last_save.write().await = Instant::now();

            Ok(())
        }
        .await;

        self.is_saving.store(false, Ordering::SeqCst);
        result
    }

    async fn load_from_disk() -> Self {
        let manager = Self::new();

        if let Ok(cache_data) = tokio::fs::read_to_string("cache/cache.json").await {
            if let Ok(cache_file) = serde_json::from_str::<CacheFile>(&cache_data) {
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
        }

        manager
    }
}

// Public interface
pub(crate) fn get_cache() -> &'static CacheManager {
    CACHE.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(CacheManager::load_from_disk())
    })
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

pub async fn save_cache() -> Result<()> {
    get_cache().save_cache_to_disk().await
}
