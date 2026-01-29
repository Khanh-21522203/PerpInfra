use crate::invariants::checks::InvariantChecks;
use crate::invariants::kill_switch::KillSwitch;
use crate::matching::order_book::OrderBook;
use crate::settlement::balance_manager::BalanceManager;
use crate::types::*;
use crate::error::Result;
use tokio::time::{interval, Duration};
use crate::types::price::Price;

pub struct InvariantMonitor {
    kill_switch: KillSwitch,
    check_interval: Duration,
}

impl InvariantMonitor {
    pub fn new(kill_switch: KillSwitch) -> Self {
        InvariantMonitor {
            kill_switch,
            check_interval: Duration::from_secs(1),
        }
    }

    pub async fn run(
        &self,
        order_book: &OrderBook,
        balance_manager: &BalanceManager,
        positions: &[position::Position],
        mark_price: Price,
    ) {
        let mut ticker = interval(self.check_interval);

        loop {
            ticker.tick().await;

            if let Err(e) = self.check_all_invariants(
                order_book,
                balance_manager,
                positions,
                mark_price,
            ) {
                tracing::error!("Invariant violation detected: {:?}", e);
                self.kill_switch.activate(format!("{:?}", e));
            }
        }
    }

    fn check_all_invariants(
        &self,
        order_book: &OrderBook,
        balance_manager: &BalanceManager,
        positions: &[crate::types::position::Position],
        mark_price: Price,
    ) -> Result<()> {
        InvariantChecks::check_order_book_consistency(order_book)?;
        InvariantChecks::check_no_negative_balances(balance_manager)?;
        InvariantChecks::check_margin_requirements(balance_manager, positions, mark_price)?;

        Ok(())
    }
}