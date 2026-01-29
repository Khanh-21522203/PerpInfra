use crate::config::fees::FeeConfig;
use crate::error::Result;
use crate::events::base::BaseEvent;
use crate::events::order::Side;
use crate::events::trade::{Fee, TradeEvent};
use crate::interfaces::balance_provider::BalanceProvider;
use crate::matching::order_book::{Order, OrderBook};
use crate::matching::self_trade::{check_self_trade, SelfTradeAction};
use crate::types::balance::Balance;
use crate::types::ids::MarketId;
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::ratio::Ratio;
use std::cmp::Reverse;
use crate::observability::metrics::{MATCHING_LATENCY, TRADES_EXECUTED, TRADE_VOLUME};

pub struct Matcher {
    order_book: OrderBook,
    fee_config: FeeConfig,
    market_id: MarketId,
}

impl Matcher {
    pub fn new(order_book: OrderBook, fee_config: FeeConfig, market_id: MarketId) -> Self {
        Matcher { order_book, fee_config, market_id }
    }

    pub fn match_order(&mut self, order: Order, balance_provider: &mut dyn BalanceProvider, mark_price: Price) -> Result<Vec<TradeEvent>> {
        // Observability: Start timing
        let _timer = MATCHING_LATENCY.start_timer();

        let mut trades = Vec::new();
        let mut remaining = order.quantity;

        while remaining > Quantity::zero() {
            // Get best opposite price
            let best_price = match order.side {
                Side::Buy => self.order_book.best_ask(),
                Side::Sell => self.order_book.best_bid(),
            };

            let best_price = match best_price {
                Some(p) => p,
                None => break,  // No more liquidity
            };

            // Check if price crosses
            if !self.price_crosses(order.side, order.price, best_price) {
                break;  // No match
            }

            // Get price level (CORRECTED: handle Reverse wrapper)
            let level_orders = if order.side == Side::Buy {
                self.order_book.asks.get_mut(&best_price)
            } else {
                self.order_book.bids.get_mut(&Reverse(best_price))
            };

            let level = match level_orders {
                Some(l) => l,
                None => break,
            };

            // Match against orders at this level (FIFO)
            while remaining > Quantity::zero() && !level.orders.is_empty() {
                let maker_order = level.orders.front_mut().unwrap();

                // Check self-trade
                let self_trade_action = check_self_trade(maker_order, &order);
                match self_trade_action {
                    SelfTradeAction::CancelMaker => {
                        let cancelled = level.orders.pop_front().unwrap();
                        self.order_book.orders.remove(&cancelled.order_id);
                        level.total_quantity = level.total_quantity - (cancelled.quantity - cancelled.filled);
                        continue;
                    }
                    SelfTradeAction::CancelTaker => {
                        return Ok(trades);  // Stop matching
                    }
                    SelfTradeAction::CancelBoth => {
                        let cancelled = level.orders.pop_front().unwrap();
                        self.order_book.orders.remove(&cancelled.order_id);
                        level.total_quantity = level.total_quantity - (cancelled.quantity - cancelled.filled);
                        return Ok(trades);
                    }
                    SelfTradeAction::Allow => {
                        // Continue with matching
                    }
                }

                // Calculate fill quantity
                let maker_remaining = maker_order.quantity - maker_order.filled;
                let fill_qty = remaining.min(maker_remaining);

                // Calculate fees
                let maker_fee = self.calculate_maker_fee(fill_qty, maker_order.price);
                let taker_fee = self.calculate_taker_fee(fill_qty, maker_order.price);

                // Create trade
                let trade = TradeEvent {
                    base: BaseEvent::new(crate::events::base::EventType::Trade, self.market_id),
                    trade_id: crate::utils::helper::generate_trade_id(),
                    maker_order_id: maker_order.order_id,
                    taker_order_id: order.order_id,
                    maker_user_id: maker_order.user_id,
                    taker_user_id: order.user_id,
                    price: maker_order.price,  // Maker price (price-time priority)
                    quantity: fill_qty,
                    maker_side: maker_order.side,
                    maker_fee,
                    taker_fee,
                    liquidation: false,
                };

                trades.push(trade);

                // Observability: Record trade metrics
                TRADES_EXECUTED.inc();
                TRADE_VOLUME.inc_by(fill_qty.to_i64() as f64);

                // Update orders
                maker_order.filled = maker_order.filled + fill_qty;
                remaining = remaining - fill_qty;

                // Remove maker if fully filled
                if maker_order.filled == maker_order.quantity {
                    let filled_order = level.orders.pop_front().unwrap();
                    self.order_book.orders.remove(&filled_order.order_id);
                }

                level.total_quantity = level.total_quantity - fill_qty;
            }

            // Remove price level if empty
            if level.orders.is_empty() {
                if order.side == Side::Buy {
                    self.order_book.asks.remove(&best_price);
                } else {
                    self.order_book.bids.remove(&Reverse(best_price));
                }
            }
        }

        // CORRECTED: Add remaining quantity to book with margin reservation
        if remaining > Quantity::zero() && order.time_in_force == crate::events::order::TimeInForce::GTC {
            let mut book_order = order.clone();
            book_order.filled = order.quantity - remaining;

            // Calculate required margin for resting order
            let required_margin = self.calculate_order_margin(&book_order, mark_price);

            // Reserve margin
            balance_provider.reserve_margin(order.user_id, required_margin)?;

            // Add to book
            self.order_book.add_order(book_order)?;
        }

        Ok(trades)
    }

    fn price_crosses(&self, side: Side, order_price: Price, level_price: Price) -> bool {
        match side {
            Side::Buy => order_price >= level_price,
            Side::Sell => order_price <= level_price,
        }
    }

    fn calculate_maker_fee(&self, quantity: Quantity, price: Price) -> Fee {
        let notional = quantity * price;
        let amount = notional * Balance::from_f64(self.fee_config.maker_fee_rate);
        Fee {
            amount,
            rate: Ratio::from(self.fee_config.maker_fee_rate),
        }
    }

    fn calculate_taker_fee(&self, quantity: Quantity, price: Price) -> Fee {
        let notional = quantity * price;
        let mut amount = notional * Balance::from_f64(self.fee_config.taker_fee_rate);
        // Round up taker fees
        amount = Balance::from_i64((amount.to_f64().ceil()) as i64);
        Fee {
            amount,
            rate: Ratio::from(self.fee_config.taker_fee_rate),
        }
    }

    fn calculate_order_margin(&self, order: &Order, mark_price: Price) -> Balance {
        let notional = order.quantity * mark_price;
        notional / Balance::from_i64(20)  // Assuming 20x max leverage
    }
}