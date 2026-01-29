use num_traits::ToPrimitive;
use crate::config::risk::RiskConfig;
use crate::types::*;
use crate::types::position::Position;
use crate::events::order::{OrderSubmit, Side};
use crate::risk::margin::MarginCalculator;
use crate::risk::pnl::PnLCalculator;
use crate::error::{Error, Result};
use crate::interfaces::balance_provider::BalanceProvider;
use crate::types::balance::Balance;
use crate::types::price::Price;
use crate::types::quantity::Quantity;

pub struct PreTradeRiskCheck {
    margin_calculator: MarginCalculator,
    config: RiskConfig,
}

impl PreTradeRiskCheck {
    pub fn new(config: RiskConfig) -> Self {
        PreTradeRiskCheck {
            margin_calculator: MarginCalculator::new(config.clone()),
            config,
        }
    }

    pub fn check(
        &self,
        order: &OrderSubmit,
        position: &Position,
        balance_provider: &dyn BalanceProvider,
        mark_price: Price,
    ) -> Result<()> {
        // Check 1: Margin requirement
        self.check_margin(order, position, balance_provider, mark_price)?;

        // Check 2: Leverage limit
        self.check_leverage(order, position, balance_provider, mark_price)?;

        // Check 3: Position limit
        self.check_position_limit(order, position)?;

        // Check 4: Reduce-only constraint
        if order.reduce_only {
            self.check_reduce_only(order, position)?;
        }

        Ok(())
    }

    fn check_margin(
        &self,
        order: &OrderSubmit,
        position: &Position,
        balance_provider: &dyn BalanceProvider,
        mark_price: Price,
    ) -> Result<()> {
        let account = balance_provider.get_account(order.user_id)?;

        // Calculate required margin for new order
        let order_margin = self.margin_calculator.calculate_initial_margin(
            order.quantity,
            mark_price,
        );

        // Calculate available balance
        let unrealized_pnl = PnLCalculator::calculate_unrealized_pnl(position, mark_price);
        let available = self.margin_calculator.calculate_available_balance(
            account.balance,
            unrealized_pnl,
            account.reserved_margin,
        );

        if available < order_margin {
            return Err(Error::InsufficientMargin {
                required: order_margin,
                available,
            });
        }

        Ok(())
    }

    fn check_leverage(
        &self,
        order: &OrderSubmit,
        position: &Position,
        balance_provider: &dyn BalanceProvider,
        mark_price: Price,
    ) -> Result<()> {
        let account = balance_provider.get_account(order.user_id)?;

        // Calculate new position size
        let order_size_signed = match order.side {
            Side::Buy => order.quantity.to_i64(),
            Side::Sell => -order.quantity.to_i64(),
        };
        let new_position_size = Quantity::from_i64(
            (position.size + order_size_signed).abs()
        );

        // Calculate leverage
        let notional = new_position_size * mark_price;
        let unrealized_pnl = PnLCalculator::calculate_unrealized_pnl(position, mark_price);
        let equity = account.balance + unrealized_pnl;

        if equity == Balance::zero() {
            return Err(Error::InsufficientBalance);
        }

        let leverage = notional.to_f64() / equity.to_f64();

        if leverage > self.config.max_leverage {
            return Err(Error::LeverageExceeded {
                leverage,
                max: self.config.max_leverage,
            });
        }

        Ok(())
    }

    fn check_position_limit(
        &self,
        order: &OrderSubmit,
        position: &Position,
    ) -> Result<()> {
        let order_size_signed = match order.side {
            Side::Buy => order.quantity.to_i64(),
            Side::Sell => -order.quantity.to_i64(),
        };
        let new_position_size = Quantity::from_i64(
            (position.size + order_size_signed).abs()
        );

        if new_position_size > self.config.max_position_size {
            return Err(Error::PositionLimitExceeded);
        }

        Ok(())
    }

    fn check_reduce_only(
        &self,
        order: &OrderSubmit,
        position: &Position,
    ) -> Result<()> {
        let is_reducing = match order.side {
            Side::Buy => position.is_short(),
            Side::Sell => position.is_long(),
        };

        if !is_reducing {
            return Err(Error::ReduceOnlyViolation);
        }

        Ok(())
    }
}