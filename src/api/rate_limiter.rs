use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

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

        let bucket = buckets.entry(key).or_insert_with(|| TokenBucket {
            tokens: self.capacity,
            last_update: now,
        });

        let elapsed = now.duration_since(bucket.last_update).as_secs_f64();
        let tokens_to_add = elapsed * self.rate;

        // Update bucket
        bucket.tokens = (bucket.tokens + tokens_to_add).min(self.capacity);
        bucket.last_update = now;

        // Check if we can consume a token
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

impl TokenBucket {
    // We will use this later when I get to making the table pretty
    fn time_until_next_token(&self, rate: f64) -> Duration {
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds_needed = tokens_needed / rate;
            Duration::from_secs_f64(seconds_needed)
        }
    }
}
