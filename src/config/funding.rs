use std::time::Duration;

#[derive(Clone, Debug)]
pub struct FundingConfig {
    pub max_funding_rate: f64,
    pub funding_interval: Duration,
    pub premium_ema_alpha: f64,
}

impl Default for FundingConfig {
    fn default() -> Self {
        FundingConfig {
            max_funding_rate: 0.001,  // 0.1% per interval
            funding_interval: Duration::from_secs(28800),  // 8 hours
            premium_ema_alpha: 0.05,
        }
    }
}