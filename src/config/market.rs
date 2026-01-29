use serde::{Deserialize, Serialize};
use crate::types::ids::MarketId;
use crate::types::price::Price;
use crate::types::quantity::Quantity;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketConfig {
    pub market_id: MarketId,
    pub symbol: String,
    pub base_asset: String,
    pub quote_asset: String,

    // Trading parameters
    pub tick_size: Price,
    pub lot_size: Quantity,
    pub min_order_size: Quantity,
    pub max_order_size: Quantity,

    // Status
    pub enabled: bool,
}

impl Default for MarketConfig {
    fn default() -> Self {
        MarketConfig {
            market_id: MarketId::new(),
            symbol: "BTC-PERP".to_string(),
            base_asset: "BTC".to_string(),
            quote_asset: "USD".to_string(),
            tick_size: Price::from_i64(1),
            lot_size: Quantity::from_i64(1),
            min_order_size: Quantity::from_i64(1),
            max_order_size: Quantity::from_i64(1_000_000),
            enabled: true,
        }
    }
}