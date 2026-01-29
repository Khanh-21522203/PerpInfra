pub mod connectors;
pub mod aggregator;
pub mod circuit_breaker;

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PriceSourceConfig {
    pub source_id: String,
    pub symbol: String,
    pub connection_type: ConnectionType,
    pub weight: f64,
    pub staleness_threshold: Duration,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ConnectionType {
    WebSocket { url: String },
    RestPolling { url: String, interval: Duration },
}

#[derive(Clone, Debug)]
pub struct RawPriceUpdate {
    pub source_id: String,
    pub symbol: String,
    pub price: f64,
    pub volume: Option<f64>,
    pub timestamp: u64,
    pub received_at: u64,
}