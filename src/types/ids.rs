use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                $name(Uuid::new_v4())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id_type!(OrderId);
define_id_type!(TradeId);
define_id_type!(UserId);
define_id_type!(MarketId);
define_id_type!(LiquidationId);
define_id_type!(EntryId);
define_id_type!(EventId);
define_id_type!(OperatorId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub Uuid);

impl AccountId {
    pub fn new() -> Self {
        AccountId(Uuid::new_v4())
    }

    pub fn from_user(user_id: UserId) -> Self {
        // In practice, might derive deterministically from user_id
        AccountId(Uuid::new_v4())
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}