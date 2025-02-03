use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{debug, trace};

pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    rate: f64,
    capacity: f64,
}

struct TokenBucket {
    tokens: f64,
    last_update: Instant,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            rate: f64::from(requests_per_second),
            capacity: f64::from(burst_size),
        }
    }

    pub async fn is_allowed(&self, key: String) -> bool {
        let mut buckets = self.buckets.lock().await;
        let now = Instant::now();

        let bucket = buckets.entry(key.clone()).or_insert_with(|| {
            debug!("Creating new token bucket for: {}", key);
            TokenBucket {
                tokens: self.capacity,
                last_update: now,
            }
        });

        let elapsed = now.duration_since(bucket.last_update).as_secs_f64();
        let tokens_to_add = elapsed * self.rate;

        // Update bucket
        bucket.tokens = (bucket.tokens + tokens_to_add).min(self.capacity);
        bucket.last_update = now;

        trace!(
            "Bucket status for {}: tokens={:.2}, last_update={:?}",
            key,
            bucket.tokens,
            bucket.last_update
        );

        // Check if we can consume a token
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            debug!(
                "Request allowed for {}, remaining tokens: {:.2}",
                key, bucket.tokens
            );
            true
        } else {
            debug!(
                "Request denied for {}, insufficient tokens: {:.2}",
                key, bucket.tokens
            );
            false
        }
    }
}
