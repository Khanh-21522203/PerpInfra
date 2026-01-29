use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeeConfig {
    pub maker_fee_rate: f64,
    pub taker_fee_rate: f64,
    pub liquidation_fee_rate: f64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        FeeConfig {
            maker_fee_rate: 0.0002,      // 0.02%
            taker_fee_rate: 0.0005,      // 0.05%
            liquidation_fee_rate: 0.005, // 0.5%
        }
    }
}