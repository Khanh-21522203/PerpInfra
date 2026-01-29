use std::time::{Duration, Instant};
use std::collections::VecDeque;

pub struct RateLimiter {
    max_per_interval: usize,
    interval: Duration,
    timestamps: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn new(max_per_interval: usize, interval: Duration) -> Self {
        RateLimiter {
            max_per_interval,
            interval,
            timestamps: VecDeque::new(),
        }
    }

    pub fn check_and_record(&mut self) -> bool {
        let now = Instant::now();

        // Remove old timestamps
        while let Some(&front) = self.timestamps.front() {
            if now.duration_since(front) > self.interval {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check limit
        if self.timestamps.len() >= self.max_per_interval {
            return false;
        }

        // Record new timestamp
        self.timestamps.push_back(now);
        true
    }
}