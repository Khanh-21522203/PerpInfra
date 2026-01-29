use crate::config::FundingConfig;
use crate::types::funding_rate::FundingRate;
use crate::types::price::Price;

pub struct FundingRateCalculator {
    config: FundingConfig,
}

impl FundingRateCalculator {
    pub fn new(config: FundingConfig) -> Self {
        FundingRateCalculator { config }
    }

    /// Calculate funding rate from premium
    /// Formula: funding_rate = clamp(premium / index_price, -max_rate, +max_rate)
    pub fn calculate_rate(
        &self,
        premium: Price,
        index_price: Price,
    ) -> FundingRate {
        let rate = premium.to_f64() / index_price.to_f64();
        let clamped = rate.max(-self.config.max_funding_rate)
            .min(self.config.max_funding_rate);

        FundingRate::from_f64(clamped)
    }

    /// Calculate premium from mark and index prices
    pub fn calculate_premium(
        &self,
        mark_price: Price,
        index_price: Price,
    ) -> Price {
        mark_price - index_price
    }
}