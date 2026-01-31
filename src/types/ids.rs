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
define_id_type!(AccountId);

impl UserId {
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(UserId(Uuid::parse_str(s)?))
    }
}

impl MarketId {
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(MarketId(Uuid::parse_str(s)?))
    }

    pub fn btc_perp() -> Self {
        MarketId(Uuid::from_u128(1))
    }
}

impl AccountId {
    pub fn from_user(user_id: UserId) -> Self {
        // Deterministic derivation: use the same UUID as the user
        // This ensures consistent account lookup across system restarts
        AccountId(user_id.0)
    }
}