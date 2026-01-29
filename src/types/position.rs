use serde::{Deserialize, Serialize};
use crate::types::*;
use crate::types::balance::Balance;
use crate::types::ids::{MarketId, UserId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::timestamp::Timestamp;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub user_id: UserId,
    pub market_id: MarketId,
    pub size: i64,  // CORRECTED: Signed - positive = long, negative = short
    pub entry_price: Price,
    pub realized_pnl: Balance,
    pub last_funding_timestamp: Timestamp,
}

impl Position {
    pub fn new(user_id: UserId, market_id: MarketId) -> Self {
        Position {
            user_id,
            market_id,
            size: 0,
            entry_price: Price::zero(),
            realized_pnl: Balance::zero(),
            last_funding_timestamp: Timestamp::now(),
        }
    }

    pub fn is_long(&self) -> bool {
        self.size > 0
    }

    pub fn is_short(&self) -> bool {
        self.size < 0
    }

    pub fn is_flat(&self) -> bool {
        self.size == 0
    }

    pub fn abs_size(&self) -> Quantity {
        Quantity::from_i64(self.size.abs())
    }
}