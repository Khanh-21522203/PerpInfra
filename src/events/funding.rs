use serde::{Deserialize, Serialize};
use crate::events::base::BaseEvent;
use crate::types::balance::Balance;
use crate::types::funding_rate::FundingRate;
use crate::types::ids::UserId;
use crate::types::price::Price;
use crate::types::quantity::Quantity;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FundingEvent {
    pub base: BaseEvent,
    pub funding_rate: FundingRate,
    pub mark_price: Price,
    pub index_price: Price,
    pub premium: Price,
    pub funding_interval: std::time::Duration,
    pub payments: Vec<FundingPayment>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FundingPayment {
    pub user_id: UserId,
    pub position_size: Quantity,
    pub payment: Balance,  // Signed: positive = receive, negative = pay
}