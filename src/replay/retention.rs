use std::time::Duration;
use crate::types::timestamp::Timestamp;

pub struct RetentionPolicy {
    pub hot_storage_duration: Duration,
    pub warm_storage_duration: Duration,
    pub cold_storage_duration: Duration,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        RetentionPolicy {
            hot_storage_duration: Duration::from_secs(86400 * 7),    // 7 days
            warm_storage_duration: Duration::from_secs(86400 * 90),  // 90 days
            cold_storage_duration: Duration::from_secs(86400 * 2555), // 7 years
        }
    }
}

pub struct DataRetentionManager {
    policy: RetentionPolicy,
}

impl DataRetentionManager {
    pub fn new(policy: RetentionPolicy) -> Self {
        DataRetentionManager { policy }
    }

    pub fn should_archive_to_warm(&self, timestamp: Timestamp) -> bool {
        let age = Timestamp::now() - timestamp;
        age > self.policy.hot_storage_duration
    }

    pub fn should_archive_to_cold(&self, timestamp: Timestamp) -> bool {
        let age = Timestamp::now() - timestamp;
        age > self.policy.warm_storage_duration
    }

    pub fn should_delete(&self, timestamp: Timestamp) -> bool {
        let age = Timestamp::now() - timestamp;
        age > self.policy.cold_storage_duration
    }
}