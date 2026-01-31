use crate::events::base::BaseEvent;
use crate::types::balance::Balance;
use crate::types::ids::UserId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BalanceUpdate {
    pub base: BaseEvent,
    pub user_id: UserId,
    pub amount: Balance,
    pub update_type: BalanceUpdateType,
    pub reference_id: Option<String>,  // External transaction ID
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BalanceUpdateType {
    Deposit,
    Withdrawal,
}