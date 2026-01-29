use crate::types::*;
use crate::types::position::Position;
use crate::risk::margin::MarginCalculator;
use crate::risk::pnl::PnLCalculator;
use crate::interfaces::balance_provider::BalanceProvider;
use crate::error::Result;
use crate::types::balance::Balance;
use crate::types::ids::UserId;
use crate::types::price::Price;
use crate::types::ratio::Ratio;

pub struct LiquidationDetector {
    margin_calculator: MarginCalculator,
}

impl LiquidationDetector {
    pub fn new(margin_calculator: MarginCalculator) -> Self {
        LiquidationDetector { margin_calculator }
    }

    pub fn detect_liquidations(
        &self,
        positions: &[Position],
        mark_price: Price,
        balance_provider: &dyn BalanceProvider,
    ) -> Result<Vec<LiquidationCandidate>> {
        let mut candidates = Vec::new();

        for position in positions {
            if position.is_flat() {
                continue;
            }

            let account = balance_provider.get_account(position.user_id)?;
            let unrealized_pnl = PnLCalculator::calculate_unrealized_pnl(position, mark_price);
            let maintenance_margin = self.margin_calculator.calculate_maintenance_margin(
                position.abs_size(),
                mark_price,
            );

            let margin_ratio = self.margin_calculator.calculate_margin_ratio(
                account.balance,
                unrealized_pnl,
                maintenance_margin,
            );

            if self.margin_calculator.is_liquidatable(margin_ratio) {
                candidates.push(LiquidationCandidate {
                    user_id: position.user_id,
                    position: position.clone(),
                    margin_ratio,
                    maintenance_margin,
                    mark_price,
                });
            }
        }

        Ok(candidates)
    }
}

#[derive(Clone, Debug)]
pub struct LiquidationCandidate {
    pub user_id: UserId,
    pub position: Position,
    pub margin_ratio: Ratio,
    pub maintenance_margin: Balance,
    pub mark_price: Price,
}