use crate::events::trade::TradeEvent;
use crate::types::balance::Balance;
use crate::types::ids::{TradeId, UserId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeReport {
    pub trade_id: TradeId,
    pub timestamp: Timestamp,
    pub symbol: String,
    pub price: Price,
    pub quantity: Quantity,
    pub buyer_id: UserId,
    pub seller_id: UserId,
    pub buyer_fee: Balance,
    pub seller_fee: Balance,
}

pub struct RegulatoryReporter;

impl RegulatoryReporter {
    pub fn generate_trade_report(
        trades: &[TradeEvent],
        symbol: &str,
    ) -> Vec<TradeReport> {
        trades.iter()
            .map(|t| TradeReport {
                trade_id: t.trade_id,
                timestamp: t.base.timestamp,
                symbol: symbol.to_string(),
                price: t.price,
                quantity: t.quantity,
                buyer_id: if t.maker_side == crate::events::order::Side::Sell {
                    t.taker_user_id
                } else {
                    t.maker_user_id
                },
                seller_id: if t.maker_side == crate::events::order::Side::Buy {
                    t.taker_user_id
                } else {
                    t.maker_user_id
                },
                buyer_fee: if t.maker_side == crate::events::order::Side::Sell {
                    t.taker_fee.amount
                } else {
                    t.maker_fee.amount
                },
                seller_fee: if t.maker_side == crate::events::order::Side::Buy {
                    t.taker_fee.amount
                } else {
                    t.maker_fee.amount
                },
            })
            .collect()
    }

    pub fn export_to_json(reports: &[TradeReport]) -> String {
        serde_json::to_string_pretty(reports).unwrap()
    }
}