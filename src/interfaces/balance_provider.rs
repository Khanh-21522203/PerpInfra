use crate::settlement::accounts::Account;
use crate::error::Result;
use crate::types::balance::Balance;
use crate::types::ids::UserId;

pub trait BalanceProvider {
    fn get_account(&self, user_id: UserId) -> Result<&Account>;
    fn adjust_balance(&mut self, user_id: UserId, amount: Balance) -> Result<()>;
    fn reserve_margin(&mut self, user_id: UserId, amount: Balance) -> Result<()>;
    fn release_margin(&mut self, user_id: UserId, amount: Balance) -> Result<()>;
}
