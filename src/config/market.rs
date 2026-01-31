use serde::{Deserialize, Serialize};
use crate::types::ids::MarketId;
use crate::types::price::Price;
use crate::types::quantity::Quantity;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketConfig {
    pub market_id: MarketId,
    pub symbol: String,
    pub tick_size: Price,
    pub lot_size: Quantity,
    pub min_order_size: Quantity,
    pub max_order_size: Quantity,
    pub max_leverage: f64,
}

impl Default for MarketConfig {
    fn default() -> Self {
        MarketConfig {
            market_id: MarketId::from_string("BTC-PERP").expect("REASON"),
            symbol: "BTC-PERP".to_string(),
            tick_size: Price::from_f64(0.01),        // $0.01
            lot_size: Quantity::from_f64(0.001),     // 0.001 BTC
            min_order_size: Quantity::from_f64(0.001), // 0.001 BTC
            max_order_size: Quantity::from_f64(100.0), // 100 BTC
            max_leverage: 20.0,
        }
    }
}