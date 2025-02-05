use crate::api::omdb::OMDBResponse;
use crate::api::tmdb::TVShowDetails;
use crate::utils::serde::timestamp;
use crate::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::OnceCell;
use tokio::time::timeout;
use tracing::{debug, error};

const MAX_CACHE_SIZE: usize = 1000;
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const CLEANUP_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour

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

#[derive(Clone)]
pub struct CacheManager {
    tv_details: Arc<DashMap<i32, CachedItem<TVShowDetails>>>,
    omdb_ratings: Arc<DashMap<String, CachedItem<OMDBResponse>>>,
    last_cleanup: Arc<AtomicU64>,
}

static CACHE: OnceCell<Arc<CacheManager>> = OnceCell::const_new();

impl CacheManager {
    fn new() -> Self {
        Self {
            tv_details: Arc::new(DashMap::with_capacity(MAX_CACHE_SIZE)),
            omdb_ratings: Arc::new(DashMap::with_capacity(MAX_CACHE_SIZE)),
            last_cleanup: Arc::new(AtomicU64::new(0)),
        }
    }

    fn get_tv_details(&self, id: i32) -> Option<CachedItem<TVShowDetails>> {
        if let Some(item) = self.tv_details.get(&id) {
            if item.timestamp.elapsed() < CACHE_TTL {
                return Some(item.clone());
            }

            // Remove expired item without holding the reference
            drop(item);
            let id = id.clone();
            let tv_details = self.tv_details.clone();
            tokio::spawn(async move {
                if let Err(_) = timeout(Duration::from_secs(1), async {
                    tv_details.remove(&id);
                })
                .await
                {
                    debug!("Timeout while trying to remove expired TV details cache entry");
                }
            });
        }
        None
    }

    fn get_omdb_rating(&self, key: &str) -> Option<CachedItem<OMDBResponse>> {
        if let Some(item) = self.omdb_ratings.get(key) {
            if item.timestamp.elapsed() < CACHE_TTL {
                return Some(item.clone());
            }

            // Remove expired item without holding the reference
            drop(item);
            let key = key.to_string();
            let omdb_ratings = self.omdb_ratings.clone();
            tokio::spawn(async move {
                if let Err(_) = timeout(Duration::from_secs(1), async {
                    omdb_ratings.remove(&key);
                })
                .await
                {
                    debug!("Timeout while trying to remove expired OMDB cache entry");
                }
            });
        }
        None
    }

    fn maybe_cleanup(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let last = self.last_cleanup.load(Ordering::Relaxed);
        if now - last > CLEANUP_INTERVAL.as_secs() {
            if self
                .last_cleanup
                .compare_exchange(last, now, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                Self::cleanup_map(&self.tv_details, "TV Details");
                Self::cleanup_map(&self.omdb_ratings, "OMDB Ratings");
            }
        }
    }

    fn cleanup_map<K: Clone + std::hash::Hash + Eq + Send + 'static, V: Clone + Send + 'static>(
        cache: &DashMap<K, CachedItem<V>>,
        cache_name: &'static str,
    ) {
        let current_size = cache.len();
        if current_size > MAX_CACHE_SIZE {
            debug!(
                "{} cache size ({}) exceeded limit ({}), cleaning up...",
                cache_name, current_size, MAX_CACHE_SIZE
            );

            // Collect keys to remove
            let keys_to_remove: Vec<_> = cache
                .iter()
                .filter_map(|entry| {
                    if entry.value().timestamp.elapsed() > CACHE_TTL {
                        Some(entry.key().clone())
                    } else {
                        None
                    }
                })
                .collect();

            // Remove expired entries
            for key in keys_to_remove {
                cache.remove(&key);
                debug!("Removed expired entry from {}", cache_name);
            }

            debug!(
                "{} cache cleanup complete. New size: {}",
                cache_name,
                cache.len()
            );
        }
    }

    fn insert_tv_details(&self, id: i32, details: TVShowDetails) {
        self.tv_details.insert(
            id,
            CachedItem {
                data: details,
                timestamp: Instant::now(),
            },
        );
        self.maybe_cleanup();
    }

    fn insert_omdb_rating(&self, key: String, rating: OMDBResponse) {
        self.omdb_ratings.insert(
            key,
            CachedItem {
                data: rating,
                timestamp: Instant::now(),
            },
        );
        self.maybe_cleanup();
    }

    async fn save_cache_to_disk(&self) -> Result<()> {
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

        debug!("Cache saved to disk successfully");
        Ok(())
    }

    async fn load_from_disk() -> Self {
        let manager = Self::new();

        if let Ok(cache_data) = tokio::fs::read_to_string("cache/cache.json").await {
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
                    debug!("Cache loaded successfully");
                }
                Err(e) => {
                    error!("Failed to parse cache file: {}", e);
                }
            }
        }

        manager
    }
}

pub async fn get_cache() -> &'static Arc<CacheManager> {
    CACHE
        .get_or_init(|| async { Arc::new(CacheManager::load_from_disk().await) })
        .await
}

pub async fn get_cached_tv_details(id: i32) -> Option<TVShowDetails> {
    get_cache().await.get_tv_details(id).map(|item| item.data)
}

pub async fn get_cached_omdb_rating(title: &str, year: &str) -> Option<OMDBResponse> {
    let key = format!("{}_{}", title, year);
    get_cache()
        .await
        .get_omdb_rating(&key)
        .map(|item| item.data)
}

pub async fn cache_tv_details(id: i32, details: TVShowDetails) {
    get_cache().await.insert_tv_details(id, details);
}

pub async fn cache_omdb_rating(title: &str, year: &str, rating: OMDBResponse) {
    let key = format!("{}_{}", title, year);
    get_cache().await.insert_omdb_rating(key, rating);
}

pub async fn save_cache() -> Result<()> {
    get_cache().await.save_cache_to_disk().await
}
