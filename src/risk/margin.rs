use crate::config::risk::RiskConfig;
use crate::types::balance::Balance;
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::ratio::Ratio;

pub struct MarginCalculator {
    config: RiskConfig,
}

impl MarginCalculator {
    pub fn new(config: RiskConfig) -> Self {
        MarginCalculator { config }
    }

    /// Calculate initial margin requirement
    pub fn calculate_initial_margin(
        &self,
        position_size: Quantity,
        mark_price: Price,
    ) -> Balance {
        let notional = position_size * mark_price;
        notional / Balance::from_f64(self.config.max_leverage)
    }

    /// Calculate maintenance margin requirement
    pub fn calculate_maintenance_margin(
        &self,
        position_size: Quantity,
        mark_price: Price,
    ) -> Balance {
        let notional = position_size * mark_price;
        notional * Balance::from_f64(self.config.maintenance_margin_rate)
    }

    /// Calculate margin ratio (for liquidation check)
    pub fn calculate_margin_ratio(
        &self,
        balance: Balance,
        unrealized_pnl: Balance,
        maintenance_margin: Balance,
    ) -> Ratio {
        let equity = balance + unrealized_pnl;
        if maintenance_margin == Balance::zero() {
            return Ratio::from(f64::INFINITY);
        }
        Ratio::from(equity.to_f64() / maintenance_margin.to_f64())
    }

    /// Check if position is liquidatable
    pub fn is_liquidatable(&self, margin_ratio: Ratio) -> bool {
        margin_ratio.to_f64() < 1.0
    }

    /// Calculate available balance for new orders
    pub fn calculate_available_balance(
        &self,
        total_balance: Balance,
        unrealized_pnl: Balance,
        reserved_margin: Balance,
    ) -> Balance {
        let equity = total_balance + unrealized_pnl;
        equity - reserved_margin
    }
}