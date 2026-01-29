use serde::{Deserialize, Serialize};
use crate::events::base::BaseEvent;
use crate::types::timestamp::Timestamp;
use crate::types::price::Price;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceSnapshot {
    pub base: BaseEvent,
    pub mark_price: Price,
    pub index_price: Price,
    pub perp_last_price: Price,
    pub premium_ema: Price,
    pub source_prices: Vec<SourcePrice>,
    pub aggregation_method: AggregationMethod,
    pub staleness_flags: Vec<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourcePrice {
    pub source_id: String,
    pub price: Price,
    pub timestamp: Timestamp,
    pub weight: f64,
    pub is_stale: bool,
    pub is_outlier: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AggregationMethod {
    WeightedMedian,
    TWAP,
    VWAP,
}