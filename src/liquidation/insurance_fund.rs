use crate::error::{Error, Result};
use std::sync::atomic::{AtomicI64, Ordering};
use crate::types::balance::Balance;

pub struct InsuranceFund {
    balance: AtomicI64,
}

impl InsuranceFund {
    pub fn new() -> Self {
        InsuranceFund {
            balance: AtomicI64::new(0),
        }
    }

    pub fn deposit(&self, amount: Balance) {
        self.balance.fetch_add(amount.to_i64(), Ordering::SeqCst);
        tracing::info!("Insurance fund deposit: {}", amount.to_i64());
    }

    pub fn cover_loss(&self, loss: Balance) -> Result<()> {
        let current = self.balance.load(Ordering::SeqCst);

        if current < loss.to_i64() {
            tracing::error!(
                "Insurance fund depleted: required={}, available={}",
                loss.to_i64(),
                current
            );

            // Alert operations team
            crate::utils::helper::alert_operations_team_critical(
                format!("Insurance fund depleted: required={}, available={}", loss.to_i64(), current)
            );

            return Err(Error::InsuranceFundDepleted {
                required: loss,
                available: Balance::from_i64(current),
            });
        }

        self.balance.fetch_sub(loss.to_i64(), Ordering::SeqCst);
        tracing::warn!("Insurance fund covered loss: {}", loss.to_i64());

        Ok(())
    }

    pub fn get_balance(&self) -> Balance {
        Balance::from_i64(self.balance.load(Ordering::SeqCst))
    }
}