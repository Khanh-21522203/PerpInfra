use crate::error::{Error, Result};
use crate::interfaces::balance_provider::BalanceProvider;
use crate::settlement::accounts::Account;
use crate::settlement::ledger::{EntryType, Ledger, LedgerEntry};
use crate::types::balance::Balance;
use crate::types::ids::{AccountId, UserId};
use crate::types::timestamp::Timestamp;
use std::collections::HashMap;

pub struct BalanceManager {
    pub accounts: HashMap<UserId, Account>,
    pub ledger: Ledger,
}

impl BalanceManager {
    pub fn new() -> Self {
        BalanceManager {
            accounts: HashMap::new(),
            ledger: Ledger::new(),
        }
    }

    pub fn create_account(&mut self, user_id: UserId) -> Result<Account> {
        if self.accounts.contains_key(&user_id) {
            return Err(Error::AccountAlreadyExists(AccountId::from_user(user_id)));
        }

        let account = Account::new(user_id);
        self.accounts.insert(user_id, account.clone());

        Ok(account)
    }

    fn record_ledger_entry(
        &mut self,
        account_id: AccountId,
        entry_type: EntryType,
        amount: Balance,
        balance_after: Balance,
        reference_id: String,
        description: String,
    ) {
        let entry = LedgerEntry {
            entry_id: crate::utils::helper::generate_entry_id(),
            timestamp: Timestamp::now(),
            entry_type,
            account_id,
            amount,
            balance_after,
            reference_id,
            description,
        };

        self.ledger.record_entry(entry);
    }
}

impl BalanceProvider for BalanceManager {
    fn get_account(&self, user_id: UserId) -> Result<&Account> {
        self.accounts.get(&user_id)
            .ok_or(Error::AccountNotFound(AccountId::from_user(user_id)))
    }

    fn adjust_balance(&mut self, user_id: UserId, amount: Balance) -> Result<()> {
        let (account_id, balance_after);
        {
            let account = self.accounts.get_mut(&user_id)
                .ok_or(Error::AccountNotFound(AccountId::from_user(user_id)))?;

            account.balance = account.balance + amount;
            account.updated_at = Timestamp::now();

            account_id = account.account_id;
            balance_after = account.balance;
        }

        self.record_ledger_entry(
            account_id,
            EntryType::Trade,
            amount,
            balance_after,
            "adjustment".to_string(),
            "Balance adjustment".to_string(),
        );

        Ok(())
    }

    fn reserve_margin(&mut self, user_id: UserId, amount: Balance) -> Result<()> {
        let (account_id, balance_after);
        {
            let account = self
                .accounts
                .get_mut(&user_id)
                .ok_or(Error::AccountNotFound(AccountId::from_user(user_id)))?;

            if account.available_balance() < amount {
                return Err(Error::InsufficientAvailableBalance);
            }

            account.reserved_margin = account.reserved_margin + amount;
            account.updated_at = Timestamp::now();

            account_id = account.account_id;
            balance_after = account.balance;
        }

        self.record_ledger_entry(
            account_id,
            EntryType::ReserveMargin,
            amount,
            balance_after,
            "reserve".to_string(),
            "Margin reservation".to_string(),
        );

        Ok(())
    }

    fn release_margin(&mut self, user_id: UserId, amount: Balance) -> Result<()> {
        let (account_id, balance_after);
        {
            let account = self.accounts.get_mut(&user_id)
                .ok_or(Error::AccountNotFound(AccountId::from_user(user_id)))?;

            account.reserved_margin = account.reserved_margin - amount;
            account.updated_at = Timestamp::now();
            account_id = account.account_id;
            balance_after = account.balance;
        }


        self.record_ledger_entry(
            account_id,
            EntryType::ReleaseMargin,
            -amount,
            balance_after,
            "release".to_string(),
            "Margin release".to_string(),
        );

        Ok(())
    }
}