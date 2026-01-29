use crate::types::*;
use crate::types::position::Position;
use crate::events::base::{BaseEvent, EventType};
use crate::events::order::*;
use crate::events::trade::TradeEvent;
use crate::events::funding::FundingEvent;
use crate::events::liquidation::LiquidationEvent;
use crate::event_log::snapshot::Snapshot;
use crate::settlement::balance_manager::BalanceManager;
use crate::settlement::accounts::Account;
use crate::matching::order_book::OrderBook;
use crate::error::{Error, Result};
use std::collections::HashMap;
use crate::interfaces::balance_provider::BalanceProvider;
use crate::types::ids::{MarketId, UserId};

pub struct EventProcessor {
    balance_manager: BalanceManager,
    order_book: OrderBook,
    positions: HashMap<UserId, Position>,
    market_id: MarketId,
    last_sequence: u64,
}

impl EventProcessor {
    pub fn new(market_id: MarketId) -> Self {
        EventProcessor {
            balance_manager: BalanceManager::new(),
            order_book: OrderBook::new(),
            positions: HashMap::new(),
            market_id,
            last_sequence: 0,
        }
    }

    pub fn restore_from_snapshot(&mut self, snapshot: &Snapshot) -> Result<()> {
        tracing::info!("Restoring state from snapshot at sequence {}", snapshot.sequence);

        // Restore accounts
        for account in &snapshot.accounts {
            self.balance_manager.create_account(account.user_id)?;
            self.balance_manager.adjust_balance(account.user_id, account.balance)?;
        }

        // Restore positions
        for position in &snapshot.positions {
            self.positions.insert(position.user_id, position.clone());
        }

        self.last_sequence = snapshot.sequence;

        tracing::info!("State restored successfully");
        Ok(())
    }

    pub fn process_event(&mut self, event: BaseEvent) -> Result<()> {
        // Verify sequence
        if event.sequence != self.last_sequence + 1 {
            return Err(Error::SequenceGap {
                expected: self.last_sequence + 1,
                actual: event.sequence,
            });
        }
        
        let event_seq = event.sequence.clone();

        // Process based on event type
        match event.event_type {
            EventType::OrderSubmit => self.process_order_submit(event)?,
            EventType::OrderCancel => self.process_order_cancel(event)?,
            EventType::Trade => self.process_trade(event)?,
            EventType::Funding => self.process_funding(event)?,
            EventType::Liquidation => self.process_liquidation(event)?,
            EventType::BalanceUpdate => self.process_balance_update(event)?,
            _ => {
                tracing::debug!("Skipping event type: {:?}", event.event_type);
            }
        }

        self.last_sequence = event_seq;
        Ok(())
    }

    fn process_order_submit(&mut self, event: BaseEvent) -> Result<()> {
        // TODO: Order submission doesn't change state until matched
        Ok(())
    }

    fn process_order_cancel(&mut self, event: BaseEvent) -> Result<()> {
        // TODO: Order cancellation handled by order book
        Ok(())
    }

    fn process_trade(&mut self, event: BaseEvent) -> Result<()> {
        // TODO: Trade events would contain the trade details
        // Update balances and positions based on trade
        Ok(())
    }

    fn process_funding(&mut self, event: BaseEvent) -> Result<()> {
        // TODO: Funding events contain payment details
        // Update balances based on funding payments
        Ok(())
    }

    fn process_liquidation(&mut self, event: BaseEvent) -> Result<()> {
        // TODO:Liquidation events contain liquidation details
        // Update positions and balances
        Ok(())
    }

    fn process_balance_update(&mut self, event: BaseEvent) -> Result<()> {
        // TODO: Direct balance updates (deposits/withdrawals)
        Ok(())
    }

    pub fn get_balance_manager(&self) -> &BalanceManager {
        &self.balance_manager
    }

    pub fn get_positions(&self) -> &HashMap<UserId, Position> {
        &self.positions
    }

    pub fn get_last_sequence(&self) -> u64 {
        self.last_sequence
    }
}