use serde::{Deserialize, Serialize};
use crate::types::quantity::Quantity;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RiskConfig {
    pub max_leverage: f64,
    pub maintenance_margin_rate: f64,
    pub initial_margin_rate: f64,
    pub max_position_size: Quantity,
    pub liquidation_fee_rate: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        RiskConfig {
            max_leverage: 20.0,
            maintenance_margin_rate: 0.05,  // 5%
            initial_margin_rate: 0.10,      // 10%
            max_position_size: Quantity::from_i64(10_000_000),
            liquidation_fee_rate: 0.005,    // 0.5%
        }
    }
}