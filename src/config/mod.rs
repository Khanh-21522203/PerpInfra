use std::time::Duration;
use serde::{Deserialize, Serialize};

pub mod market;
pub mod risk;
pub mod fees;
pub mod loader;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FundingConfig {
    pub funding_interval: Duration,
    pub max_funding_rate: f64,
    pub premium_ema_alpha: f64,
}

impl Default for FundingConfig {
    fn default() -> Self {
        FundingConfig {
            funding_interval: Duration::from_secs(28800),  // 8 hours
            max_funding_rate: 0.0005,  // 0.05%
            premium_ema_alpha: 0.05,
        }
    }
}