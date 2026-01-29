use crate::settlement::accounts::Account;
use crate::types::ids::MarketId;
use crate::types::position::Position;
use crate::types::price::Price;
use crate::types::timestamp::Timestamp;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub sequence: u64,
    pub timestamp: Timestamp,
    pub market_id: MarketId,
    pub accounts: Vec<Account>,
    pub positions: Vec<Position>,
    pub mark_price: Price,
    pub index_price: Price,
    pub checksum: String,
}

impl Snapshot {
    pub fn new(
        sequence: u64,
        market_id: MarketId,
        accounts: Vec<Account>,
        positions: Vec<Position>,
        mark_price: Price,
        index_price: Price,
    ) -> Self {
        let mut snapshot = Snapshot {
            version: crate::SNAPSHOT_VERSION,
            sequence,
            timestamp: Timestamp::now(),
            market_id,
            accounts,
            positions,
            mark_price,
            index_price,
            checksum: String::new(),
        };

        snapshot.checksum = snapshot.calculate_checksum();
        snapshot
    }

    fn calculate_checksum(&self) -> String {
        let mut hasher = Sha256::new();

        // Hash all state
        hasher.update(self.sequence.to_le_bytes());
        hasher.update(self.timestamp.physical.to_le_bytes());

        for account in &self.accounts {
            hasher.update(account.balance.to_i64().to_le_bytes());
        }

        for position in &self.positions {
            hasher.update(position.size.to_le_bytes());
        }

        let result = hasher.finalize();
        hex::encode(result)
    }

    pub fn verify_checksum(&self) -> bool {
        let calculated = self.calculate_checksum();
        calculated == self.checksum
    }
}