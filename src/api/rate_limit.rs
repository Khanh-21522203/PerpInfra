use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crate::error::{Error, Result};
use crate::types::ids::UserId;

pub struct RateLimiter {
    limits: Arc<Mutex<HashMap<UserId, RateLimit>>>,
    max_requests: usize,
    window: Duration,
}

struct RateLimit {
    count: usize,
    window_start: Instant,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        RateLimiter {
            limits: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub fn check(&self, user_id: UserId) -> Result<()> {
        let mut limits = self.limits.lock().unwrap();
        let now = Instant::now();

        let limit = limits.entry(user_id).or_insert(RateLimit {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now.duration_since(limit.window_start) > self.window {
            limit.count = 0;
            limit.window_start = now;
        }

        // Check limit
        if limit.count >= self.max_requests {
            return Err(Error::RateLimitExceeded);
        }

        limit.count += 1;
        Ok(())
    }
}