use crate::error::{Error, Result};
use crate::events::base::BaseEvent;
use crate::events::liquidation::{LiquidationEvent, LiquidationType};
use crate::events::order::Side;
use crate::interfaces::balance_provider::BalanceProvider;
use crate::liquidation::detector::LiquidationCandidate;
use crate::liquidation::insurance_fund::InsuranceFund;
use crate::liquidation::priority_queue::LiquidationPriorityQueue;
use crate::liquidation::rate_limiter::RateLimiter;
use crate::matching::matcher::Matcher;
use crate::matching::order_book::Order;
use crate::types::balance::Balance;
use crate::types::ids::MarketId;
use crate::types::quantity::Quantity;
use crate::types::timestamp::Timestamp;
use std::time::Duration;

pub struct LiquidationExecutor {
    queue: LiquidationPriorityQueue,
    rate_limiter: RateLimiter,
    insurance_fund: InsuranceFund,
    market_id: MarketId,
}

impl LiquidationExecutor {
    pub fn new(market_id: MarketId) -> Self {
        LiquidationExecutor {
            queue: LiquidationPriorityQueue::new(),
            rate_limiter: RateLimiter::new(10, Duration::from_secs(1)),
            insurance_fund: InsuranceFund::new(),
            market_id,
        }
    }

    pub fn add_candidate(&mut self, candidate: LiquidationCandidate) {
        self.queue.push(candidate);
    }

    pub fn execute_next(
        &mut self,
        matcher: &mut Matcher,
        balance_provider: &mut dyn BalanceProvider,
    ) -> Result<Option<LiquidationEvent>> {
        // Check rate limit
        if !self.rate_limiter.check_and_record() {
            return Err(Error::LiquidationRateLimitExceeded);
        }

        // Get next candidate
        let candidate = match self.queue.pop() {
            Some(c) => c,
            None => return Ok(None),
        };

        // Create liquidation order (opposite side of position)
        let liquidation_side = if candidate.position.is_long() {
            Side::Sell
        } else {
            Side::Buy
        };

        let liquidation_order = Order {
            order_id: crate::utils::helper::generate_order_id(),
            user_id: crate::LIQUIDATION_ENGINE_USER_ID,
            side: liquidation_side,
            price: candidate.mark_price,
            quantity: candidate.position.abs_size(),
            filled: Quantity::zero(),
            timestamp: Timestamp::now(),
            time_in_force: crate::events::order::TimeInForce::IOC,
            reduce_only: false,
            post_only: false,
        };

        // Execute liquidation through matcher
        let trades = matcher.match_order(
            liquidation_order,
            balance_provider,
            candidate.mark_price,
        )?;

        // Calculate liquidated size
        let liquidated_size: Quantity = trades.iter()
            .map(|t| t.quantity)
            .sum();

        if liquidated_size == Quantity::zero() {
            return Err(Error::LiquidationFailedNoLiquidity);
        }

        // Calculate loss
        let account = balance_provider.get_account(candidate.user_id)?;
        let loss = if account.balance < Balance::zero() {
            account.balance.abs()
        } else {
            Balance::zero()
        };

        // Cover loss with insurance fund
        if loss > Balance::zero() {
            self.insurance_fund.cover_loss(loss)?;
        }

        // Determine liquidation type
        let liquidation_type = if liquidated_size == candidate.position.abs_size() {
            LiquidationType::Full
        } else {
            LiquidationType::Partial
        };

        // Create event
        let event = LiquidationEvent {
            base: BaseEvent::new(crate::events::base::EventType::Liquidation, self.market_id),
            liquidation_id: crate::utils::helper::generate_liquidation_id(),
            user_id: candidate.user_id,
            position_size: candidate.position.abs_size(),
            liquidated_size,
            liquidation_price: candidate.mark_price,
            margin_ratio: candidate.margin_ratio,
            maintenance_margin: candidate.maintenance_margin,
            insurance_fund_loss: loss,
            liquidation_type,
        };

        // Observability: Record liquidation metrics
        use crate::observability::metrics::*;
        LIQUIDATIONS_EXECUTED.inc();
        INSURANCE_FUND_BALANCE.set(self.insurance_fund.get_balance().to_i64());

        Ok(Some(event))
    }
}