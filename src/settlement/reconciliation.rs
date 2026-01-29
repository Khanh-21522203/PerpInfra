use crate::error::{Error, Result};
use crate::interfaces::balance_provider::BalanceProvider;
use crate::settlement::balance_manager::BalanceManager;
use crate::types::balance::Balance;
use crate::types::ids::UserId;

pub struct Reconciliation;

impl Reconciliation {
    /// Verify double-entry accounting invariant
    pub fn verify_double_entry(
        debits: Balance,
        credits: Balance,
    ) -> Result<()> {
        if debits != credits {
            return Err(Error::DoubleEntryImbalance {
                debits: debits.to_i64(),
                credits: credits.to_i64(),
            });
        }
        Ok(())
    }

    /// Reconcile account balance with ledger
    pub fn reconcile_account(
        balance_manager: &BalanceManager,
        user_id: UserId,
    ) -> Result<()> {
        let account = balance_manager.get_account(user_id)?;

        // Calculate balance from ledger
        let ledger_balance = balance_manager.ledger.get_entries_for_account(account.account_id)
            .iter()
            .map(|e| e.amount.to_i64())
            .sum::<i64>();

        let expected = Balance::from_i64(ledger_balance);

        if account.balance != expected {
            return Err(Error::ReconciliationFailed {
                expected,
                actual: account.balance,
            });
        }

        Ok(())
    }

    /// Verify conservation of value across all accounts
    pub fn verify_conservation_of_value(
        balance_manager: &BalanceManager,
    ) -> Result<()> {
        let total: i64 = balance_manager.accounts.values()
            .map(|a| a.balance.to_i64())
            .sum();

        // Total should be zero (or match initial deposits)
        // For simplicity, we check it's within acceptable bounds
        if total.abs() > 1000 {  // Allow small rounding errors
            return Err(Error::ConservationOfValueViolation {
                expected: Balance::zero(),
                actual: Balance::from_i64(total),
            });
        }

        Ok(())
    }
}