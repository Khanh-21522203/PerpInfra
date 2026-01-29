use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp {
    pub physical: u64,  // Milliseconds since epoch
    pub logical: u64,   // Monotonic counter
}

impl Timestamp {
    pub fn now() -> Self {
        HLC.now()
    }

    pub fn from_millis(millis: u64) -> Self {
        Timestamp {
            physical: millis,
            logical: 0,
        }
    }
}

impl std::ops::Add<std::time::Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, duration: std::time::Duration) -> Timestamp {
        Timestamp {
            physical: self.physical + duration.as_millis() as u64,
            logical: self.logical,
        }
    }
}

impl std::ops::Sub for Timestamp {
    type Output = std::time::Duration;

    fn sub(self, other: Timestamp) -> std::time::Duration {
        std::time::Duration::from_millis(self.physical.saturating_sub(other.physical))
    }
}

pub struct HybridLogicalClock {
    last_physical: AtomicU64,
    last_logical: AtomicU64,
}

impl HybridLogicalClock {
    pub fn new() -> Self {
        HybridLogicalClock {
            last_physical: AtomicU64::new(0),
            last_logical: AtomicU64::new(0),
        }
    }

    pub fn now(&self) -> Timestamp {
        let wall_clock = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        loop {
            let last_physical = self.last_physical.load(Ordering::SeqCst);
            let last_logical = self.last_logical.load(Ordering::SeqCst);

            let (new_physical, new_logical) = if wall_clock > last_physical {
                // Wall clock advanced
                (wall_clock, 0)
            } else {
                // Wall clock same or went backward, increment logical
                (last_physical, last_logical + 1)
            };

            // Try to update atomically
            if self.last_physical.compare_exchange(
                last_physical,
                new_physical,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ).is_ok() {
                self.last_logical.store(new_logical, Ordering::SeqCst);
                return Timestamp {
                    physical: new_physical,
                    logical: new_logical,
                };
            }
            // Retry if CAS failed
        }
    }
}

lazy_static::lazy_static! {
    static ref HLC: HybridLogicalClock = HybridLogicalClock::new();
}