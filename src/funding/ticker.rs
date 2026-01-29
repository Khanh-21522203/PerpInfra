use tokio::time::{interval, Duration};
use crate::funding::applicator::FundingApplicator;
use crate::types::*;
use crate::types::position::Position;
use crate::interfaces::balance_provider::BalanceProvider;
use crate::error::Result;
use crate::types::ids::MarketId;
use crate::types::price::Price;

pub struct FundingTicker {
    applicator: FundingApplicator,
    interval: Duration,
}

impl FundingTicker {
    pub fn new(applicator: FundingApplicator, interval: Duration) -> Self {
        FundingTicker { applicator, interval }
    }

    pub async fn run(
        &self,
        mut positions: Vec<Position>,
        mark_price: Price,
        index_price: Price,
        balance_provider: &mut dyn BalanceProvider,
        market_id: MarketId,
    ) -> Result<()> {
        let mut ticker = interval(self.interval);

        loop {
            ticker.tick().await;

            // Apply funding
            let event = self.applicator.apply_funding(
                &mut positions,
                mark_price,
                index_price,
                balance_provider,
                market_id,
            )?;

            tracing::info!(
                "Funding applied: rate={:.6}, payments={}",
                event.funding_rate.to_f64(),
                event.payments.len()
            );

            // Emit event (would be sent to event log in production)
        }
    }
}