use crate::types::position::Position;
use crate::events::base::{BaseEvent, EventPayload, EventType};
use crate::event_log::snapshot::Snapshot;
use crate::settlement::balance_manager::BalanceManager;
use crate::matching::order_book::{Order, OrderBook};
use crate::error::{Error, Result};
use std::collections::HashMap;
use crate::interfaces::balance_provider::BalanceProvider;
use crate::types::ids::{MarketId, UserId};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::RwLock;
use crate::config::market::MarketConfig;
use crate::event_log::producer::KafkaEventProducer;
use crate::events::balance::BalanceUpdateType;
use crate::events::liquidation::LiquidationType;
use crate::events::order::Side;
use crate::events::trade::TradeEvent;
use crate::funding::applicator::FundingApplicator;
use crate::interfaces::event_producer::EventProducer;
use crate::liquidation::detector::LiquidationCandidate;
use crate::liquidation::executor::LiquidationExecutor;
use crate::matching::matcher::Matcher;
use crate::matching::validator::OrderValidator;
use crate::observability::metrics::{LIQUIDATIONS_EXECUTED, LIQUIDATION_VOLUME, ORDERS_SUBMITTED};
use crate::risk::margin::MarginCalculator;
use crate::settlement::position_manager::PositionManager;
use crate::types::balance::Balance;
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::ratio::Ratio;
use crate::utils::helper::alert_operations_team_critical;

pub struct EventProcessor {
    // Core state
    market_id: MarketId,
    last_sequence: u64,
    last_mark_price: Price,
    halted: AtomicBool,

    market_config: MarketConfig,

    // Shared dependencies (injected)
    balance_manager: Arc<RwLock<BalanceManager>>,
    position_manager: Arc<RwLock<PositionManager>>,
    order_book: Arc<RwLock<OrderBook>>,
    matcher: Arc<RwLock<Matcher>>,
    margin_calculator: Arc<MarginCalculator>,
    funding_applicator: Arc<FundingApplicator>,
    liquidation_executor: Arc<LiquidationExecutor>,
    event_producer: Arc<KafkaEventProducer>,
}

impl EventProcessor {
    pub fn new_with_dependencies(
        market_id: MarketId,
        market_config: MarketConfig,
        balance_manager: Arc<RwLock<BalanceManager>>,
        position_manager: Arc<RwLock<PositionManager>>,
        order_book: Arc<RwLock<OrderBook>>,
        matcher: Arc<RwLock<Matcher>>,
        margin_calculator: Arc<MarginCalculator>,
        funding_applicator: Arc<FundingApplicator>,
        liquidation_executor: Arc<LiquidationExecutor>,
        event_producer: Arc<KafkaEventProducer>,
    ) -> Self {
        EventProcessor {
            market_id,
            last_sequence: 0,
            last_mark_price: Price::from_i64(50000_00000000), // Default BTC price $50k
            halted: AtomicBool::new(false),
            market_config,
            balance_manager,
            position_manager,
            order_book,
            matcher,
            margin_calculator,
            funding_applicator,
            liquidation_executor,
            event_producer,
        }
    }

    pub async fn restore_from_snapshot(&mut self, snapshot: &Snapshot) -> Result<()> {
        tracing::info!("Restoring state from snapshot at sequence {}", snapshot.sequence);

        // Restore accounts
        let mut balance_mgr = self.balance_manager.write().await;
        for account in &snapshot.accounts {
            balance_mgr.create_account(account.user_id)?;
            balance_mgr.adjust_balance(account.user_id, account.balance)?;
        }
        drop(balance_mgr);

        // Restore positions
        let mut position_mgr = self.position_manager.write().await;
        for position in &snapshot.positions {
            position_mgr.set_position(position.user_id, position.clone());
        }
        drop(position_mgr);

        self.last_sequence = snapshot.sequence;

        tracing::info!("State restored successfully");
        Ok(())
    }

    pub async fn process_event(&mut self, event: BaseEvent) -> Result<()> {
        if self.halted.load(Ordering::SeqCst) {
            tracing::warn!("EventProcessor is halted, rejecting event");
            return Err(Error::KillSwitchActive);
        }

        // FIX IGD-S-040: Verify sequence with proper gap handling
        let expected_sequence = self.last_sequence + 1;

        if event.sequence < expected_sequence {
            // Duplicate event - already processed (idempotent)
            tracing::warn!(
                "Duplicate event received: seq={}, already at={}",
                event.sequence, self.last_sequence
            );
            return Ok(()); // Skip duplicate
        }

        if event.sequence > expected_sequence {
            // Gap detected - MUST halt processing per docs/
            tracing::error!(
                "SEQUENCE GAP DETECTED: expected={}, received={}. HALTING PROCESSING.",
                expected_sequence, event.sequence
            );

            // Activate kill switch for sequence gap
            crate::KILL_SWITCH.store(true, Ordering::SeqCst);

            // Alert operations team
            alert_operations_team_critical(
                format!(
                    "Sequence gap detected: expected={}, received={}. Processing halted.",
                    expected_sequence, event.sequence
                )
            );

            return Err(Error::SequenceGap {
                expected: expected_sequence,
                actual: event.sequence,
            });
        }

        // Verify event checksum before processing
        if !event.verify_checksum() {
            tracing::error!("Event checksum verification failed: {:?}", event.event_id);
            return Err(Error::ChecksumMismatch {
                event_id: event.event_id,
            });
        }

        let event_sequence = event.sequence;

        // Process based on event type
        match event.event_type {
            EventType::OrderSubmit => self.process_order_submit(event).await?,
            EventType::OrderCancel => self.process_order_cancel(event).await?,
            EventType::Trade => self.process_trade(event).await?,
            EventType::Funding => self.process_funding(event).await?,
            EventType::Liquidation => self.process_liquidation(event).await?,
            EventType::BalanceUpdate => self.process_balance_update(event).await?,
            EventType::PriceSnapshot => self.process_price_update(event).await?,
            _ => {
                tracing::debug!("Skipping event type: {:?}", event.event_type);
            }
        }

        self.last_sequence = event_sequence;
        Ok(())
    }

    async fn process_order_submit(&mut self, event: BaseEvent) -> Result<()> {
        tracing::debug!("Processing order submit event: {:?}", event.event_id);

        // Extract OrderSubmit from typed payload (FIX: use payload instead of metadata string)
        let order_submit = match event.payload {
            crate::events::base::EventPayload::OrderSubmit(payload) => *payload,
            _ => {
                return Err(Error::InvalidEventPayload {
                    expected: "OrderSubmit".to_string(),
                    found: format!("{:?}", event.event_type),
                });
            }
        };

        // 1. Validate order parameters
        let validator = OrderValidator::new(self.market_config.clone());
        validator.validate(&order_submit)?;

        // 2. Check margin requirements
        let balance_mgr = self.balance_manager.blocking_read();
        let account = balance_mgr.get_account(order_submit.user_id)?;

        let position_mgr = self.position_manager.blocking_read();
        let position = position_mgr.get_position(&order_submit.user_id);

        let required_margin = self.margin_calculator.calculate_initial_margin(
            order_submit.quantity,
            self.last_mark_price,
        );

        let available_balance = account.available_balance();
        if available_balance < required_margin {
            return Err(Error::InsufficientMargin {
                required: required_margin,
                available: available_balance,
            });
        }
        drop(balance_mgr);
        drop(position_mgr);

        // 3. Reserve margin
        let mut balance_mgr = self.balance_manager.blocking_write();
        balance_mgr.reserve_margin(order_submit.user_id, required_margin)?;
        drop(balance_mgr);

        // 4. Add order to order book
        let mut order_book = self.order_book.blocking_write();
        let order = Order {
            order_id: order_submit.order_id,
            user_id: order_submit.user_id,
            side: order_submit.side,
            order_type: order_submit.order_type,
            price: order_submit.price.unwrap_or(Price::zero()),
            quantity: order_submit.quantity,
            filled: Quantity::zero(),
            timestamp: order_submit.base.timestamp,
            time_in_force: order_submit.time_in_force,
            reduce_only: order_submit.reduce_only,
            post_only: order_submit.post_only,
            slippage_limit: order_submit.slippage_limit,
        };
        order_book.add_order(order.clone())?;
        drop(order_book);

        // 5. Attempt matching
        let mut matcher = self.matcher.write().await;
        let mut balance_mgr = self.balance_manager.write().await;
        let trades = matcher.match_order(&order, &mut *balance_mgr, self.last_mark_price)?;
        drop(balance_mgr);
        drop(matcher);

        // 6. Update positions and balances based on trades
        if !trades.is_empty() {
            let mut position_mgr = self.position_manager.blocking_write();
            let mut balance_mgr = self.balance_manager.blocking_write();

            for trade in &trades {
                // Update maker position (opposite side of trade)
                let maker_trade_side = match trade.maker_side {
                    Side::Buy => Side::Sell,  // Maker was buying, so they receive
                    Side::Sell => Side::Buy,  // Maker was selling, so they deliver
                };
                position_mgr.update_position(
                    trade.maker_user_id,
                    maker_trade_side,
                    trade.quantity,
                    trade.price,
                )?;

                // Update taker position (same side as trade)
                let taker_trade_side = match trade.maker_side {
                    Side::Buy => Side::Buy,   // Taker was selling to maker's buy
                    Side::Sell => Side::Sell, // Taker was buying from maker's sell
                };
                position_mgr.update_position(
                    trade.taker_user_id,
                    taker_trade_side,
                    trade.quantity,
                    trade.price,
                )?;

                // Apply fees
                balance_mgr.adjust_balance(
                    trade.maker_user_id,
                    Balance::from_i64(-trade.maker_fee.amount.to_i64()),
                )?;
                balance_mgr.adjust_balance(
                    trade.taker_user_id,
                    Balance::from_i64(-trade.taker_fee.amount.to_i64()),
                )?;

                // Emit trade event
                let trade_event = TradeEvent {
                    base: BaseEvent::new(
                        EventType::Trade,
                        self.market_id,
                    ),
                    trade_id: trade.trade_id,
                    maker_order_id: trade.maker_order_id,
                    taker_order_id: trade.taker_order_id,
                    maker_user_id: trade.maker_user_id,
                    taker_user_id: trade.taker_user_id,
                    price: trade.price,
                    quantity: trade.quantity,
                    maker_side: trade.maker_side,
                    maker_fee: trade.maker_fee,
                    taker_fee: trade.taker_fee,
                    liquidation: trade.liquidation,
                };

                // Emit trade event to event log
                let base = trade_event.base.clone();
                let base_event = BaseEvent {
                    payload: EventPayload::Trade(Box::new(trade_event)),
                    ..base
                };
                self.event_producer.produce(base_event).await?;
                
                // In production, collect events and emit in batch
                tracing::info!("Trade executed: {:?}", trade.trade_id);
            }
        }

        let side = match order_submit.side {
            Side::Buy => "buy",
            Side::Sell => "sell",
        };
        let order_type = if order_submit.price.is_some() { "limit" } else { "market" };
        ORDERS_SUBMITTED.with_label_values(&[side, order_type]).inc();

        Ok(())
    }

    async fn process_order_cancel(&mut self, event: BaseEvent) -> Result<()> {
        tracing::debug!("Processing order cancel event: {:?}", event.event_id);

        let order_cancel = match event.payload {
            EventPayload::OrderCancel(payload) => *payload,
            _ => {
                return Err(Error::InvalidEventPayload {
                    expected: "OrderCancel".to_string(),
                    found: format!("{:?}", event.event_type),
                });
            }
        };

        // 1. Find order in order book
        let mut order_book = self.order_book.blocking_write();
        let order = order_book.get_order(&order_cancel.order_id)
            .ok_or(Error::OrderNotFound(order_cancel.order_id))?;

        // Verify user owns this order
        if order.user_id != order_cancel.user_id {
            return Err(Error::Unauthorized);
        }

        // 2. Calculate unfilled quantity
        let unfilled_quantity = order.quantity - order.filled;

        // 3. Remove order from order book
        order_book.remove_order(&order_cancel.order_id)?;
        drop(order_book);

        // 4. Release reserved margin
        if unfilled_quantity > Quantity::zero() {
            let mut balance_mgr = self.balance_manager.blocking_write();

            // Calculate margin to release based on unfilled quantity
            let position_mgr = self.position_manager.blocking_read();
            let position = position_mgr.get_position(&order_cancel.user_id);

            let margin_to_release = self.margin_calculator.calculate_initial_margin(
                unfilled_quantity,
                self.last_mark_price,
            );

            balance_mgr.release_margin(order_cancel.user_id, margin_to_release)?;
        }

        // Observability
        use crate::observability::metrics::*;
        ORDERS_CANCELLED.inc();

        tracing::info!("Order cancelled: {:?}, unfilled: {}", 
                      order_cancel.order_id, unfilled_quantity.to_i64());

        Ok(())
    }

    async fn process_trade(&mut self, event: BaseEvent) -> Result<()> {
        tracing::debug!("Processing trade event: {:?}", event.event_id);

        // Deserialize TradeEvent from event.metadata
        let trade_event = match event.payload {
            crate::events::base::EventPayload::Trade(payload) => *payload,
            _ => {
                return Err(Error::InvalidEventPayload {
                    expected: "Trade".to_string(),
                    found: format!("{:?}", event.event_type),
                });
            }
        };

        // 1. Update maker position
        let mut position_mgr = self.position_manager.blocking_write();

        position_mgr.update_position(
            trade_event.maker_user_id,
            trade_event.maker_side,
            trade_event.quantity,
            trade_event.price,
        )?;

        // 2. Update taker position (opposite side of maker)
        let taker_side = match trade_event.maker_side {
            Side::Buy => Side::Sell,
            Side::Sell => Side::Buy,
        };

        position_mgr.update_position(
            trade_event.taker_user_id,
            taker_side,
            trade_event.quantity,
            trade_event.price,
        )?;

        drop(position_mgr);

        // 3. Apply maker and taker fees
        let mut balance_mgr = self.balance_manager.blocking_write();
        balance_mgr.adjust_balance(
            trade_event.maker_user_id,
            Balance::from_i64(-trade_event.maker_fee.amount.to_i64()),
        )?;
        balance_mgr.adjust_balance(
            trade_event.taker_user_id,
            Balance::from_i64(-trade_event.taker_fee.amount.to_i64()),
        )?;
        drop(balance_mgr);

        // 4. Update margin requirements (recalculate after position change)
        let position_mgr = self.position_manager.blocking_read();
        let maker_position = position_mgr.get_position(&trade_event.maker_user_id);
        let taker_position = position_mgr.get_position(&trade_event.taker_user_id);

        if let Some(pos) = maker_position {
            let required_margin = self.margin_calculator.calculate_maintenance_margin(
                Quantity::from_i64(pos.size.abs()),
                trade_event.price,
            );
            tracing::debug!("Maker margin requirement: {}", required_margin.to_i64());
        }

        if let Some(pos) = taker_position {
            let required_margin = self.margin_calculator.calculate_maintenance_margin(
                Quantity::from_i64(pos.size.abs()),
                trade_event.price,
            );
            tracing::debug!("Taker margin requirement: {}", required_margin.to_i64());
        }

        // 5. Remove fully filled orders from order book
        let mut order_book = self.order_book.blocking_write();

        if let Some(maker_order) = order_book.get_order(&trade_event.maker_order_id) {
            if maker_order.filled >= maker_order.quantity {
                order_book.remove_order(&trade_event.maker_order_id)?;
            }
        }

        if let Some(taker_order) = order_book.get_order(&trade_event.taker_order_id) {
            if taker_order.filled >= taker_order.quantity {
                order_book.remove_order(&trade_event.taker_order_id)?;
            }
        }

        // Observability
        use crate::observability::metrics::*;
        TRADES_EXECUTED.inc();
        VOLUME_TRADED.inc_by(trade_event.quantity.to_i64() as f64);

        tracing::info!("Trade processed: {:?}, qty: {}, price: {}", 
                      trade_event.trade_id,
                      trade_event.quantity.to_i64(),
                      trade_event.price.to_f64());

        TRADES_PROCESSED.inc();
    
        Ok(())
    }

    async fn process_funding(&mut self, event: BaseEvent) -> Result<()> {
        tracing::debug!("Processing funding event: {:?}", event.event_id);

        // Deserialize FundingEvent from event.metadata
        let funding_event = match &event.payload {
            EventPayload::Funding(payload) => payload.as_ref().clone(),
            _ => {
                return Err(Error::InvalidEventPayload {
                    expected: "Funding".to_string(),
                    found: format!("{:?}", event.event_type),
                });
            }
        };

        // 1. Apply each funding payment
        let mut balance_mgr = self.balance_manager.blocking_write();
        let mut total_payments: i64 = 0;

        for payment in &funding_event.payments {
            balance_mgr.adjust_balance(payment.user_id, payment.payment)?;
            total_payments += payment.payment.to_i64();

            tracing::debug!("Applied funding payment: user={:?}, amount={}", 
                          payment.user_id, payment.payment.to_i64());
        }

        drop(balance_mgr);

        // 2. Verify zero-sum property (critical invariant)
        if total_payments != 0 {
            return Err(Error::FundingNotZeroSum { sum: total_payments });
        }

        // 3. Update position funding timestamps
        let mut position_mgr = self.position_manager.blocking_write();
        for payment in &funding_event.payments {
            if let Some(position) = position_mgr.get_position_mut(&payment.user_id) {
                position.last_funding_timestamp = funding_event.base.timestamp;
            }
        }

        // Observability
        use crate::observability::metrics::*;
        FUNDING_EVENTS_PROCESSED.inc();
        FUNDING_RATE.with_label_values(&["default"]).set(funding_event.funding_rate.to_f64());

        tracing::info!("Funding applied: rate={:.6}, payments={}", 
                      funding_event.funding_rate.to_f64(),
                      funding_event.payments.len());

        Ok(())
    }

    async fn process_liquidation(&mut self, event: BaseEvent) -> Result<()> {
        tracing::debug!("Processing liquidation event: {:?}", event.event_id);

        // Deserialize LiquidationTriggered from event.metadata
        let liquidation_event = match event.payload {
            crate::events::base::EventPayload::Liquidation(payload) => *payload,
            _ => {
                return Err(Error::InvalidEventPayload {
                    expected: "Liquidation".to_string(),
                    found: format!("{:?}", event.event_type),
                });
            }
        };


        // Get position to create proper liquidation candidate
        let position_mgr = self.position_manager.blocking_read();
        let position = position_mgr.get_position(&liquidation_event.user_id)
            .ok_or(Error::ConfigError("Position not found for liquidation".to_string()))?;

        // Create liquidation candidate from event
        // LiquidationCandidate fields: user_id, position, margin_ratio, maintenance_margin, mark_price
        let margin_ratio_value = if liquidation_event.account_value.to_i64() != 0 {
            liquidation_event.maintenance_margin.to_f64() / liquidation_event.account_value.to_f64()
        } else {
            0.0
        };

        let candidate = crate::liquidation::detector::LiquidationCandidate {
            user_id: liquidation_event.user_id,
            position: position.clone(),
            margin_ratio: Ratio::from_f64(margin_ratio_value),
            maintenance_margin: liquidation_event.maintenance_margin,
            mark_price: liquidation_event.mark_price,
        };
        drop(position_mgr);

        // Execute liquidation
        let mut matcher = self.matcher.blocking_write();
        let mut balance_mgr = self.balance_manager.blocking_write();

        // Add candidate to executor queue
        let mut executor = self.liquidation_executor.clone();
        executor.add_candidate(candidate);

        match executor.execute_next(&mut *matcher, &mut *balance_mgr) {
            Ok(Some(liq_event)) => {
                drop(matcher);
                drop(balance_mgr);

                // Update position
                let mut position_mgr = self.position_manager.blocking_write();

                if let Some(position) = position_mgr.get_position_mut(&liquidation_event.user_id) {
                    // Calculate new position size after liquidation
                    let liquidated_qty = liq_event.liquidated_size.to_i64();

                    if position.size > 0 {
                        // Long position
                        position.size = position.size.saturating_sub(liquidated_qty);
                    } else {
                        // Short position
                        position.size = position.size.saturating_add(liquidated_qty);
                    }

                    // Record insurance fund charge if any
                    if liq_event.insurance_fund_loss > Balance::zero() {
                        tracing::warn!("Insurance fund charged: {}", 
                                      liq_event.insurance_fund_loss.to_i64());
                    }

                    // Remove position if fully liquidated
                    if position.size == 0 {
                        position_mgr.remove_position(&liquidation_event.user_id);
                        tracing::info!("Position fully liquidated: {:?}", liquidation_event.user_id);
                    }
                }

                // Observability
                let liq_type = match liq_event.liquidation_type {
                    LiquidationType::Full => "full",
                    LiquidationType::Partial => "partial",
                };
                LIQUIDATIONS_EXECUTED.with_label_values(&[liq_type]).inc();
                LIQUIDATION_VOLUME.inc_by(liq_event.liquidated_size.to_i64() as f64);

                tracing::info!("Liquidation executed: user={:?}, size={}, price={}", 
                              liquidation_event.user_id,
                              liq_event.liquidated_size.to_i64(),
                              liq_event.liquidation_price.to_f64());
            }
            Ok(None) => {
                tracing::warn!("Liquidation execution returned no result");
            }
            Err(e) => {
                tracing::error!("Liquidation execution failed: {:?}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    async fn process_balance_update(&mut self, event: BaseEvent) -> Result<()> {
        tracing::debug!("Processing balance update event: {:?}", event.event_id);

        let balance_update = match event.payload {
            crate::events::base::EventPayload::BalanceUpdate(payload) => *payload,
            _ => {
                return Err(Error::InvalidEventPayload {
                    expected: "BalanceUpdate".to_string(),
                    found: format!("{:?}", event.event_type),
                });
            }
        };

        let mut balance_mgr = self.balance_manager.blocking_write();

        // 1. Apply balance change (deposit or withdrawal)
        match balance_update.update_type {
            BalanceUpdateType::Deposit => {
                // Create account if it doesn't exist
                if balance_mgr.get_account(balance_update.user_id).is_err() {
                    balance_mgr.create_account(balance_update.user_id)?;
                }

                balance_mgr.adjust_balance(balance_update.user_id, balance_update.amount)?;

                tracing::info!("Deposit processed: user={:?}, amount={}", 
                              balance_update.user_id, balance_update.amount.to_i64());
            }
            BalanceUpdateType::Withdrawal => {
                // Verify sufficient available balance
                let account = balance_mgr.get_account(balance_update.user_id)?;

                if account.available_balance() < balance_update.amount {
                    return Err(Error::InsufficientAvailableBalance);
                }

                balance_mgr.adjust_balance(
                    balance_update.user_id,
                    Balance::from_i64(-balance_update.amount.to_i64())
                )?;

                tracing::info!("Withdrawal processed: user={:?}, amount={}", 
                              balance_update.user_id, balance_update.amount.to_i64());
            }
        }

        // 2. Record ledger entry
        let account = balance_mgr.get_account(balance_update.user_id)?;

        // 3. Verify balance remains non-negative
        if account.balance < Balance::zero() {
            tracing::error!("Negative balance detected: user={:?}, balance={}", 
                          balance_update.user_id, account.balance.to_i64());
            return Err(Error::InsufficientBalance);
        }

        // Observability
        use crate::observability::metrics::*;
        match balance_update.update_type {
            BalanceUpdateType::Deposit => {
                DEPOSITS_PROCESSED.inc();
                DEPOSIT_VOLUME.inc_by(balance_update.amount.to_i64() as f64);
            }
            BalanceUpdateType::Withdrawal => {
                WITHDRAWALS_PROCESSED.inc();
                WITHDRAWAL_VOLUME.inc_by(balance_update.amount.to_i64() as f64);
            }
        }

        Ok(())
    }

    async fn process_price_update(&mut self, event: BaseEvent) -> Result<()> {
        tracing::debug!("Processing price update event: {:?}", event.event_id);

        // Extract PriceSnapshot from typed payload
        let price_snapshot = match event.payload {
            crate::events::base::EventPayload::PriceSnapshot(payload) => *payload,
            _ => {
                return Err(Error::InvalidEventPayload {
                    expected: "PriceSnapshot".to_string(),
                    found: format!("{:?}", event.event_type),
                });
            }
        };

        // Update last mark price
        self.last_mark_price = price_snapshot.mark_price;

        tracing::debug!("Mark price updated: {}", price_snapshot.mark_price.to_f64());

        Ok(())
    }

    /// Halt event processing per docs/architecture/invariants.md Section 4.3
    pub fn halt(&self) {
        self.halted.store(true, Ordering::SeqCst);
        tracing::warn!("EventProcessor HALTED");
    }

    /// Resume event processing per docs/architecture/invariants.md Section 4.3
    pub fn resume(&self) {
        self.halted.store(false, Ordering::SeqCst);
        tracing::info!("EventProcessor RESUMED");
    }

    /// Check if event processor is halted
    pub fn is_halted(&self) -> bool {
        self.halted.load(Ordering::SeqCst)
    }
}