use crate::config::risk::RiskConfig;
use crate::error::{Error, InvariantViolation, Result};
use crate::interfaces::balance_provider::BalanceProvider;
use crate::matching::order_book::OrderBook;
use crate::risk::margin::MarginCalculator;
use crate::risk::pnl::PnLCalculator;
use crate::settlement::balance_manager::BalanceManager;
use crate::types::balance::Balance;
use crate::types::price::Price;
use crate::types::quantity::Quantity;

pub struct InvariantChecks;

impl InvariantChecks {
    /// Check order book consistency - IMPLEMENTED
    pub fn check_order_book_consistency(order_book: &OrderBook) -> Result<()> {
        // Verify price levels match order quantities
        for (price, level) in &order_book.bids {
            let calculated_qty: i64 = level.orders.iter()
                .map(|o| (o.quantity - o.filled).to_i64())
                .sum();

            if calculated_qty != level.total_quantity.to_i64() {
                return Err(Error::InvariantViolation(InvariantViolation {
                    invariant: "order_book_consistency",
                    details: format!(
                        "Bid level at price {} has inconsistent quantity: expected {}, got {}",
                        price.0,
                        level.total_quantity.to_i64(),
                        calculated_qty
                    ),
                }));
            }
        }

        for (price, level) in &order_book.asks {
            let calculated_qty: i64 = level.orders.iter()
                .map(|o| (o.quantity - o.filled).to_i64())
                .sum();

            if calculated_qty != level.total_quantity.to_i64() {
                return Err(Error::InvariantViolation(InvariantViolation {
                    invariant: "order_book_consistency",
                    details: format!(
                        "Ask level at price {} has inconsistent quantity: expected {}, got {}",
                        price,
                        level.total_quantity.to_i64(),
                        calculated_qty
                    ),
                }));
            }
        }

        Ok(())
    }

    /// Check no negative balances
    pub fn check_no_negative_balances(
        balance_manager: &BalanceManager,
    ) -> Result<()> {
        for account in balance_manager.accounts.values() {
            if account.balance < Balance::zero() {
                return Err(Error::InvariantViolation(InvariantViolation {
                    invariant: "no_negative_balances",
                    details: format!(
                        "Account {:?} has negative balance: {}",
                        account.user_id,
                        account.balance.to_i64()
                    ),
                }));
            }
        }
        Ok(())
    }

    /// Check margin requirements - IMPLEMENTED
    pub fn check_margin_requirements(
        balance_manager: &BalanceManager,
        positions: &[crate::types::position::Position],
        mark_price: Price,
    ) -> Result<()> {

        let margin_calc = MarginCalculator::new(RiskConfig::default());

        for position in positions {
            if position.is_flat() {
                continue;
            }

            let account = balance_manager.get_account(position.user_id)?;
            let unrealized_pnl = PnLCalculator::calculate_unrealized_pnl(position, mark_price);
            let maintenance_margin = margin_calc.calculate_maintenance_margin(
                position.abs_size(),
                mark_price,
            );

            let margin_ratio = margin_calc.calculate_margin_ratio(
                account.balance,
                unrealized_pnl,
                maintenance_margin,
            );

            // Check if position should have been liquidated
            if margin_calc.is_liquidatable(margin_ratio) {
                return Err(Error::InvariantViolation(InvariantViolation {
                    invariant: "margin_requirements",
                    details: format!(
                        "Position for user {:?} is under-margined: margin_ratio={}",
                        position.user_id,
                        margin_ratio.to_f64()
                    ),
                }));
            }
        }

        Ok(())
    }

    /// Check funding zero-sum
    pub fn check_funding_zero_sum(
        payments: &[crate::events::funding::FundingPayment],
    ) -> Result<()> {
        let sum: i64 = payments.iter()
            .map(|p| p.payment.to_i64())
            .sum();

        if sum.abs() > 1 {
            return Err(Error::InvariantViolation(InvariantViolation {
                invariant: "funding_zero_sum",
                details: format!("Funding payments sum to {}, expected 0", sum),
            }));
        }

        Ok(())
    }

    /// Check position size limits
    pub fn check_position_limits(
        positions: &[crate::types::position::Position],
        max_position_size: Quantity,
    ) -> Result<()> {
        for position in positions {
            if position.abs_size() > max_position_size {
                return Err(Error::InvariantViolation(InvariantViolation {
                    invariant: "position_limits",
                    details: format!(
                        "Position for user {:?} exceeds limit: size={}, max={}",
                        position.user_id,
                        position.abs_size().to_i64(),
                        max_position_size.to_i64()
                    ),
                }));
            }
        }

        Ok(())
    }

    /// Check reserved margin consistency
    pub fn check_reserved_margin(
        balance_manager: &BalanceManager,
    ) -> Result<()> {
        for account in balance_manager.accounts.values() {
            if account.reserved_margin < Balance::zero() {
                return Err(Error::InvariantViolation(InvariantViolation {
                    invariant: "reserved_margin",
                    details: format!(
                        "Account {:?} has negative reserved margin: {}",
                        account.user_id,
                        account.reserved_margin.to_i64()
                    ),
                }));
            }

            if account.reserved_margin > account.balance {
                return Err(Error::InvariantViolation(InvariantViolation {
                    invariant: "reserved_margin",
                    details: format!(
                        "Account {:?} has reserved margin exceeding balance: reserved={}, balance={}",
                        account.user_id,
                        account.reserved_margin.to_i64(),
                        account.balance.to_i64()
                    ),
                }));
            }
        }

        Ok(())
    }
}