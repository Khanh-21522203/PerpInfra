use serde::{Deserialize, Serialize};
use crate::types::balance::Balance;
use crate::types::ids::{AccountId, UserId};
use crate::types::timestamp::Timestamp;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub account_id: AccountId,
    pub user_id: UserId,
    pub balance: Balance,
    pub reserved_margin: Balance,
    pub realized_pnl: Balance,
    pub unrealized_pnl: Balance,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Account {
    pub fn new(user_id: UserId) -> Self {
        let now = Timestamp::now();
        Account {
            account_id: AccountId::from_user(user_id),
            user_id,
            balance: Balance::zero(),
            reserved_margin: Balance::zero(),
            realized_pnl: Balance::zero(),
            unrealized_pnl: Balance::zero(),  // FIX IGD-S-001
            created_at: now,
            updated_at: now,
        }
    }

    pub fn available_balance(&self) -> Balance {
        self.balance - self.reserved_margin
    }

    /// Calculate total equity (balance + unrealized PnL)
    /// Per docs/architecture/risk-engine.md Section 4.2
    pub fn equity(&self) -> Balance {
        Balance::from_i64(self.balance.to_i64() + self.unrealized_pnl.to_i64())
    }

    /// Update unrealized PnL based on current mark price
    pub fn update_unrealized_pnl(&mut self, pnl: Balance) {
        self.unrealized_pnl = pnl;
        self.updated_at = Timestamp::now();
    }
}