use serde::{Deserialize, Serialize};
use crate::types::quantity::Quantity;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RiskConfig {
    pub max_leverage: f64,
    pub maintenance_margin_rate: f64,
    pub initial_margin_rate: f64,
    pub max_position_size: Quantity,
}

impl Default for RiskConfig {
    fn default() -> Self {
        RiskConfig {
            max_leverage: 20.0,
            maintenance_margin_rate: 0.05,  // 5%
            initial_margin_rate: 0.10,      // 10% (1/max_leverage for 10x effective)
            max_position_size: Quantity::from_i64(1000_00000000), // 1000 BTC
        }
    }
}