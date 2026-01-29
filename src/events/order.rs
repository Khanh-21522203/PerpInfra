use serde::{Deserialize, Serialize};
use crate::events::base::BaseEvent;
use crate::types::ids::{OrderId, UserId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::ratio::Ratio;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum OrderEvent {
    Submit(OrderSubmit),
    Cancel(OrderCancel),
    Amend(OrderAmend),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderSubmit {
    pub base: BaseEvent,
    pub order_id: OrderId,
    pub user_id: UserId,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<Price>,
    pub quantity: Quantity,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    pub post_only: bool,
    pub slippage_limit: Option<Ratio>,  // For market orders
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderCancel {
    pub base: BaseEvent,
    pub order_id: OrderId,
    pub user_id: UserId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderAmend {
    pub base: BaseEvent,
    pub order_id: OrderId,
    pub user_id: UserId,
    pub new_price: Option<Price>,
    pub new_quantity: Option<Quantity>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderAccepted {
    pub base: BaseEvent,
    pub order_id: OrderId,
    pub user_id: UserId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderRejected {
    pub base: BaseEvent,
    pub order_id: OrderId,
    pub user_id: UserId,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        }
    }

    pub fn sign(&self) -> i64 {
        match self {
            Side::Buy => 1,
            Side::Sell => -1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeInForce {
    GTC,  // Good Till Cancel
    IOC,  // Immediate Or Cancel
    FOK,  // Fill Or Kill
}