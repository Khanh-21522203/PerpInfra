use crate::events::order::{OrderRejected, OrderSubmit};
use crate::events::trade::TradeEvent;
use crate::types::account::Account;
use crate::types::balance::Balance;
use crate::types::ids::UserId;
use crate::types::position::Position;
use crate::types::price::Price;

pub struct ExplainabilityEngine;

impl ExplainabilityEngine {
    /// Explain why an order was rejected
    pub fn explain_order_rejection(
        order: &OrderSubmit,
        rejection: &OrderRejected,
        account: &Account,
        position: &Position,
        mark_price: Price,
    ) -> String {
        format!(
            "Order {} rejected: {}\n\
             Order details: side={:?}, quantity={}, price={:?}\n\
             Account balance: {}\n\
             Position size: {}\n\
             Mark price: {}",
            order.order_id,
            rejection.reason,
            order.side,
            order.quantity.to_i64(),
            order.price.map(|p| p.to_i64()),
            account.balance.to_i64(),
            position.size,
            mark_price.to_i64()
        )
    }

    /// Explain a trade execution
    pub fn explain_trade(
        trade: &TradeEvent,
        maker_account: &Account,
        taker_account: &Account,
    ) -> String {
        format!(
            "Trade {} executed:\n\
             Price: {}, Quantity: {}\n\
             Maker: {:?} (balance: {})\n\
             Taker: {:?} (balance: {})\n\
             Maker fee: {}, Taker fee: {}",
            trade.trade_id,
            trade.price.to_i64(),
            trade.quantity.to_i64(),
            trade.maker_user_id,
            maker_account.balance.to_i64(),
            trade.taker_user_id,
            taker_account.balance.to_i64(),
            trade.maker_fee.amount.to_i64(),
            trade.taker_fee.amount.to_i64()
        )
    }

    /// Explain balance change
    pub fn explain_balance_change(
        user_id: UserId,
        old_balance: Balance,
        new_balance: Balance,
        reason: &str,
    ) -> String {
        let change = new_balance - old_balance;
        format!(
            "Balance change for {:?}:\n\
             Old: {}, New: {}, Change: {}\n\
             Reason: {}",
            user_id,
            old_balance.to_i64(),
            new_balance.to_i64(),
            change.to_i64(),
            reason
        )
    }
}