use crate::events::price::{PriceSnapshot, SourcePrice, AggregationMethod};
use crate::events::base::BaseEvent;
use crate::price_infra::{RawPriceUpdate, PriceSourceConfig};
use crate::error::{Error, Result};
use std::time::Duration;
use crate::types::ids::MarketId;
use crate::types::price::Price;
use crate::types::timestamp::Timestamp;
use crate::utils::helper::current_timestamp_ms;

pub struct PriceAggregator {
    sources: Vec<PriceSourceConfig>,
    staleness_threshold: Duration,
    outlier_threshold: f64,
    ema_alpha: f64,
    premium_ema: Price,
}

impl PriceAggregator {
    pub fn new(sources: Vec<PriceSourceConfig>) -> Self {
        PriceAggregator {
            sources,
            staleness_threshold: Duration::from_secs(5),
            outlier_threshold: 0.05,  // 5%
            ema_alpha: 0.05,
            premium_ema: Price::zero(),
        }
    }

    pub fn aggregate(
        &mut self,
        raw_prices: Vec<RawPriceUpdate>,
        perp_last_price: Price,
        market_id: MarketId,
    ) -> Result<PriceSnapshot> {
        let now = current_timestamp_ms();

        // Step 1: Filter stale prices
        let fresh_prices: Vec<_> = raw_prices.iter()
            .filter(|p| now - p.received_at <= self.staleness_threshold.as_millis() as u64)
            .collect();

        if fresh_prices.len() < 2 {
            return Err(Error::InsufficientFreshPrices(fresh_prices.len()));
        }

        // Step 2: Detect outliers
        let median = self.calculate_median(&fresh_prices);
        let non_outliers: Vec<_> = fresh_prices.iter()
            .filter(|p| {
                let deviation = (p.price - median).abs() / median;
                deviation <= self.outlier_threshold
            })
            .copied()
            .collect();

        if non_outliers.is_empty() {
            return Err(Error::AllPricesAreOutliers);
        }

        // Step 3: Calculate weighted median (index price) - CORRECTED
        let index_price = self.weighted_median(&non_outliers)?;

        // Step 4: Calculate mark price (EMA-adjusted)
        let premium = perp_last_price - index_price;
        self.premium_ema = Price::from_f64(
            self.ema_alpha * premium.to_f64() + (1.0 - self.ema_alpha) * self.premium_ema.to_f64()
        );
        let mark_price = index_price + self.premium_ema;

        // Step 5: Create snapshot
        Ok(PriceSnapshot {
            base: BaseEvent::new(crate::events::base::EventType::PriceSnapshot, market_id),
            mark_price,
            index_price,
            perp_last_price,
            premium_ema: self.premium_ema,
            source_prices: raw_prices.iter().map(|p| {
                let is_stale = now - p.received_at > self.staleness_threshold.as_millis() as u64;
                let is_outlier = {
                    let deviation = (p.price - median).abs() / median;
                    deviation > self.outlier_threshold
                };

                SourcePrice {
                    source_id: p.source_id.clone(),
                    price: Price::from_f64(p.price),
                    timestamp: Timestamp::from_millis(p.timestamp),
                    weight: self.get_weight(&p.source_id),
                    is_stale,
                    is_outlier,
                }
            }).collect(),
            aggregation_method: AggregationMethod::WeightedMedian,
            staleness_flags: raw_prices.iter()
                .map(|p| now - p.received_at > self.staleness_threshold.as_millis() as u64)
                .collect(),
        })
    }

    /// CORRECTED: Proper weighted median with cumulative weights
    fn weighted_median(&self, prices: &[&RawPriceUpdate]) -> Result<Price> {
        // Create weighted price pairs
        let mut weighted_prices: Vec<(f64, f64)> = prices.iter()
            .map(|p| (p.price, self.get_weight(&p.source_id)))
            .collect();

        // Sort by price
        weighted_prices.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // Calculate total weight
        let total_weight: f64 = weighted_prices.iter().map(|(_, w)| w).sum();
        let target_weight = total_weight / 2.0;

        // Find median using cumulative weights
        let mut cumulative = 0.0;
        for (price, weight) in weighted_prices {
            cumulative += weight;
            if cumulative >= target_weight {
                return Ok(Price::from_f64(price));
            }
        }

        Err(Error::WeightedMedianFailed)
    }

    fn calculate_median(&self, prices: &[&RawPriceUpdate]) -> f64 {
        let mut sorted: Vec<f64> = prices.iter().map(|p| p.price).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    fn get_weight(&self, source_id: &str) -> f64 {
        self.sources.iter()
            .find(|s| s.source_id == source_id)
            .map(|s| s.weight)
            .unwrap_or(0.0)
    }
}