use serde::{Deserialize, Serialize};
use crate::events::base::BaseEvent;
use crate::events::order::Side;
use crate::types::balance::Balance;
use crate::types::ids::{OrderId, TradeId, UserId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::ratio::Ratio;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TradeEvent {
    pub base: BaseEvent,
    pub trade_id: TradeId,
    pub maker_order_id: OrderId,
    pub taker_order_id: OrderId,
    pub maker_user_id: UserId,
    pub taker_user_id: UserId,
    pub price: Price,
    pub quantity: Quantity,
    pub maker_side: Side,
    pub maker_fee: Fee,
    pub taker_fee: Fee,
    pub liquidation: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Fee {
    pub amount: Balance,
    pub rate: Ratio,
}