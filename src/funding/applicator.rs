use std::sync::atomic::{AtomicBool, Ordering};
use crate::error::{Error, Result};
use crate::events::base::BaseEvent;
use crate::events::funding::FundingEvent;
use crate::funding::payment_calculator::FundingPaymentCalculator;
use crate::funding::rate_calculator::FundingRateCalculator;
use crate::interfaces::balance_provider::BalanceProvider;
use crate::types::ids::MarketId;
use crate::types::position::Position;
use crate::types::price::Price;
use crate::types::timestamp::Timestamp;
use std::time::Duration;

pub struct FundingApplicator {
    rate_calculator: FundingRateCalculator,
    funding_interval: Duration,
    halted: AtomicBool,
}

impl FundingApplicator {
    pub fn new(
        rate_calculator: FundingRateCalculator,
        funding_interval: Duration,
    ) -> Self {
        FundingApplicator {
            rate_calculator,
            funding_interval,
            halted: AtomicBool::new(false),
        }
    }

    pub fn apply_funding(
        &self,
        positions: &mut [Position],
        mark_price: Price,
        index_price: Price,
        balance_provider: &mut dyn BalanceProvider,
        market_id: MarketId,
    ) -> Result<FundingEvent> {

        if self.halted.load(Ordering::SeqCst) {
            tracing::warn!("FundingApplicator is halted, skipping funding");
            return Err(Error::KillSwitchActive);
        }

        // Calculate funding rate
        let premium = self.rate_calculator.calculate_premium(mark_price, index_price);
        let funding_rate = self.rate_calculator.calculate_rate(premium, index_price);

        // Calculate payments
        let payments = FundingPaymentCalculator::calculate_all_payments(
            positions,
            mark_price,
            funding_rate,
        );

        // Verify zero-sum
        if !FundingPaymentCalculator::verify_zero_sum(&payments) {
            let sum: i64 = payments.iter().map(|p| p.payment.to_i64()).sum();
            return Err(Error::FundingNotZeroSum { sum });
        }

        // Apply payments to balances
        for payment in &payments {
            balance_provider.adjust_balance(payment.user_id, payment.payment)?;
        }

        // Update position timestamps
        let now = Timestamp::now();
        for position in positions.iter_mut() {
            position.last_funding_timestamp = now;
        }

        // Create event
        Ok(FundingEvent {
            base: BaseEvent::new(crate::events::base::EventType::Funding, market_id),
            funding_rate,
            mark_price,
            index_price,
            premium,
            funding_interval: self.funding_interval,
            payments,
        })
    }

    pub fn halt(&self) {
        self.halted.store(true, Ordering::SeqCst);
        tracing::warn!("FundingApplicator HALTED");
    }

    pub fn resume(&self) {
        self.halted.store(false, Ordering::SeqCst);
        tracing::info!("FundingApplicator RESUMED");
    }

    pub fn is_halted(&self) -> bool {
        self.halted.load(Ordering::SeqCst)
    }
}