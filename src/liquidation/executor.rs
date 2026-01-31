use std::sync::atomic::{AtomicBool, Ordering};
use crate::error::{Error, Result};
use crate::events::base::BaseEvent;
use crate::events::liquidation::{LiquidationEvent, LiquidationType};
use crate::events::order::{OrderType, Side, TimeInForce};
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
use crate::LIQUIDATION_ENGINE_USER_ID;
use crate::observability::metrics::{INSURANCE_FUND_BALANCE, LIQUIDATIONS_EXECUTED};
use crate::types::position::Position;
use crate::types::price::Price;

pub struct LiquidationExecutor {
    queue: LiquidationPriorityQueue,
    rate_limiter: RateLimiter,
    insurance_fund: InsuranceFund,
    market_id: MarketId,
    halted: AtomicBool,
}

impl LiquidationExecutor {
    pub fn new(market_id: MarketId) -> Self {
        LiquidationExecutor {
            queue: LiquidationPriorityQueue::new(),
            rate_limiter: RateLimiter::new(10, Duration::from_secs(1)),
            insurance_fund: InsuranceFund::new(),
            market_id,
            halted: AtomicBool::new(false),
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

        if self.halted.load(Ordering::SeqCst) {
            tracing::warn!("LiquidationExecutor is halted, skipping execution");
            return Ok(None);
        }

        // Check rate limit
        if !self.rate_limiter.check_and_record() {
            return Err(Error::LiquidationRateLimitExceeded);
        }

        // Get next candidate
        let candidate = match self.queue.pop() {
            Some(c) => c,
            None => return Ok(None),
        };

        // Calculate liquidation size (partial or full)
        let liquidation_size = self.calculate_liquidation_size(
            &candidate,
            balance_provider,
        )?;

        // Create liquidation order (opposite side of position)
        let liquidation_side = if candidate.position.is_long() {
            Side::Sell
        } else {
            Side::Buy
        };

        let liquidation_order = Order {
            order_id: crate::utils::helper::generate_order_id(),
            user_id: *LIQUIDATION_ENGINE_USER_ID,
            side: liquidation_side,
            order_type: OrderType::Limit,
            price: candidate.mark_price,
            quantity: liquidation_size,
            filled: Quantity::zero(),
            timestamp: Timestamp::now(),
            time_in_force: TimeInForce::IOC,
            reduce_only: false,
            post_only: false,
            slippage_limit: None,
        };

        // Execute liquidation through matcher
        let trades = matcher.match_order(
            &liquidation_order,
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
        let liq_type = match liquidation_type {
            LiquidationType::Full => "full",
            LiquidationType::Partial => "partial",
        };        LIQUIDATIONS_EXECUTED.with_label_values(&[liq_type]).inc();
        INSURANCE_FUND_BALANCE.set(self.insurance_fund.get_balance().to_i64());

        Ok(Some(event))
    }

    /// Calculate partial liquidation size to restore margin health
    /// Per docs/architecture/liquidation-engine.md Section 4.1
    fn calculate_partial_liquidation_size(
        &self,
        position: &Position,
        balance: Balance,
        mark_price: Price,
    ) -> Quantity {
        // Target: margin_ratio = 15% (above maintenance margin)
        const TARGET_MARGIN_RATIO: f64 = 0.15;
        const MIN_POSITION_SIZE: i64 = 1;

        let position_value = position.abs_size().to_i64() * mark_price.to_i64();
        let unrealized_pnl = (mark_price.to_i64() - position.entry_price.to_i64()) * position.size;
        let collateral = balance.to_i64() + unrealized_pnl;

        // Solve for liquidation_size:
        // (collateral + liquidation_pnl) / (position_value - liquidation_value) = target_ratio
        // Simplified: target_position_value = collateral / target_ratio
        let target_position_value = (collateral as f64 / TARGET_MARGIN_RATIO) as i64;

        if target_position_value <= 0 {
            // Full liquidation required
            return position.abs_size();
        }

        let liquidation_value = position_value - target_position_value;
        let liquidation_size = liquidation_value / mark_price.to_i64();

        // Clamp to position size
        let clamped_size = liquidation_size.max(0).min(position.abs_size().to_i64());

        // If remaining position would be too small, liquidate fully
        let remaining_size = position.abs_size().to_i64() - clamped_size;
        if remaining_size < MIN_POSITION_SIZE && remaining_size > 0 {
            return position.abs_size();
        }

        Quantity::from_i64(clamped_size)
    }

    /// Determine liquidation size (partial or full)
    fn calculate_liquidation_size(
        &self,
        candidate: &LiquidationCandidate,
        balance_provider: &dyn BalanceProvider,
    ) -> Result<Quantity> {
        let account = balance_provider.get_account(candidate.user_id)?;

        // If margin ratio is extremely low (< 5%), do full liquidation
        const EMERGENCY_MARGIN_RATIO: f64 = 0.05;
        if candidate.margin_ratio.to_f64() < EMERGENCY_MARGIN_RATIO {
            return Ok(candidate.position.abs_size());
        }

        // Calculate partial liquidation size
        let partial_size = self.calculate_partial_liquidation_size(
            &candidate.position,
            account.balance,
            candidate.mark_price,
        );

        // If partial size >= 90% of position, do full liquidation
        let position_size = candidate.position.abs_size().to_i64();
        if partial_size.to_i64() >= (position_size * 9 / 10) {
            return Ok(candidate.position.abs_size());
        }

        Ok(partial_size)
    }

    pub fn halt(&self) {
        self.halted.store(true, Ordering::SeqCst);
        tracing::warn!("LiquidationExecutor HALTED");
    }

    pub fn resume(&self) {
        self.halted.store(false, Ordering::SeqCst);
        tracing::info!("LiquidationExecutor RESUMED");
    }

    pub fn is_halted(&self) -> bool {
        self.halted.load(Ordering::SeqCst)
    }
}