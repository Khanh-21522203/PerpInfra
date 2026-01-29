use crate::types::*;
use serde::{Deserialize, Serialize};
use crate::types::balance::Balance;
use crate::types::ids::{AccountId, EntryId};
use crate::types::timestamp::Timestamp;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: EntryId,
    pub timestamp: Timestamp,
    pub entry_type: EntryType,
    pub account_id: AccountId,
    pub amount: Balance,  // Signed: positive = debit, negative = credit
    pub balance_after: Balance,
    pub reference_id: String,
    pub description: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EntryType {
    Deposit,
    Withdrawal,
    Trade,
    Fee,
    Funding,
    Liquidation,
    ReserveMargin,
    ReleaseMargin,
}

pub struct Ledger {
    entries: Vec<LedgerEntry>,
}

impl Ledger {
    pub fn new() -> Self {
        Ledger {
            entries: Vec::new(),
        }
    }

    pub fn record_entry(&mut self, entry: LedgerEntry) {
        self.entries.push(entry);
    }

    pub fn get_entries_for_account(&self, account_id: AccountId) -> Vec<&LedgerEntry> {
        self.entries.iter()
            .filter(|e| e.account_id == account_id)
            .collect()
    }

    pub fn verify_balance(&self, account_id: AccountId, expected: Balance) -> bool {
        let calculated: i64 = self.entries.iter()
            .filter(|e| e.account_id == account_id)
            .map(|e| e.amount.to_i64())
            .sum();

        Balance::from_i64(calculated) == expected
    }
}