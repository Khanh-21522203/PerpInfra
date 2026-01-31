use serde::{Deserialize, Serialize};
use crate::events::base::BaseEvent;
use crate::types::balance::Balance;
use crate::types::ids::{LiquidationId, UserId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::ratio::Ratio;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiquidationTriggered {
    pub base: BaseEvent,
    pub user_id: UserId,
    pub position_size: Quantity,
    pub mark_price: Price,
    pub maintenance_margin: Balance,
    pub account_value: Balance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiquidationEvent {
    pub base: BaseEvent,
    pub liquidation_id: LiquidationId,
    pub user_id: UserId,
    pub position_size: Quantity,
    pub liquidated_size: Quantity,
    pub liquidation_price: Price,
    pub margin_ratio: Ratio,
    pub maintenance_margin: Balance,
    pub insurance_fund_loss: Balance,
    pub liquidation_type: LiquidationType,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LiquidationType {
    Partial,
    Full,
}