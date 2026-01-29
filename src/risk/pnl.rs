use crate::events::order::Side;
use crate::types::balance::Balance;
use crate::types::position::Position;
use crate::types::price::Price;
use crate::types::quantity::Quantity;

pub struct PnLCalculator;

impl PnLCalculator {
    /// Calculate unrealized PnL for a position
    pub fn calculate_unrealized_pnl(
        position: &Position,
        mark_price: Price,
    ) -> Balance {
        if position.is_flat() {
            return Balance::zero();
        }

        // size is already signed
        Balance::from_i64(
            position.size * (mark_price.to_i64() - position.entry_price.to_i64())
        )
    }

    /// Calculate realized PnL from a trade
    pub fn calculate_realized_pnl(
        position: &Position,
        trade_side: Side,
        trade_quantity: Quantity,
        trade_price: Price,
    ) -> Balance {
        // Only realize PnL if reducing position
        let is_reducing = match trade_side {
            Side::Buy => position.is_short(),
            Side::Sell => position.is_long(),
        };

        if !is_reducing {
            return Balance::zero();
        }

        let close_qty = trade_quantity.to_i64().min(position.size.abs());
        let pnl_per_unit = if position.is_long() {
            trade_price.to_i64() - position.entry_price.to_i64()
        } else {
            position.entry_price.to_i64() - trade_price.to_i64()
        };

        Balance::from_i64(close_qty * pnl_per_unit)
    }

    /// Update position after trade
    pub fn update_position(
        position: &mut Position,
        trade_side: Side,
        trade_quantity: Quantity,
        trade_price: Price,
    ) {
        let trade_size_signed = match trade_side {
            Side::Buy => trade_quantity.to_i64(),
            Side::Sell => -trade_quantity.to_i64(),
        };

        let new_size = position.size + trade_size_signed;

        // Calculate realized PnL if reducing
        let realized = Self::calculate_realized_pnl(position, trade_side, trade_quantity, trade_price);
        position.realized_pnl = position.realized_pnl + realized;

        // Update entry price if increasing or flipping
        if (position.size >= 0 && new_size > position.size) ||
            (position.size <= 0 && new_size < position.size) {
            // Increasing position
            let old_notional = position.size.abs() * position.entry_price.to_i64();
            let new_notional = trade_quantity.to_i64() * trade_price.to_i64();
            let total_size = position.size.abs() + trade_quantity.to_i64();

            if total_size > 0 {
                position.entry_price = Price::from_i64((old_notional + new_notional) / total_size);
            }
        } else if new_size == 0 {
            // Position closed
            position.entry_price = Price::zero();
        }

        position.size = new_size;
    }
}