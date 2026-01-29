use crate::types::balance::Balance;
use crate::types::ids::{AccountId, UserId};
use crate::types::timestamp::Timestamp;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub account_id: AccountId,
    pub user_id: UserId,
    pub balance: Balance,
    pub reserved_margin: Balance,
    pub realized_pnl: Balance,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Account {
    pub fn new(user_id: UserId) -> Self {
        let now = Timestamp::now();
        Account {
            account_id: AccountId::new(),
            user_id,
            balance: Balance::zero(),
            reserved_margin: Balance::zero(),
            realized_pnl: Balance::zero(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn available_balance(&self) -> Balance {
        self.balance - self.reserved_margin
    }
}