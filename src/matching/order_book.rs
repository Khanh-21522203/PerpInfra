use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, VecDeque};
use crate::error::{Error, Result};
use crate::events::order::{Side, TimeInForce};
use crate::types::ids::{OrderId, UserId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use crate::types::timestamp::Timestamp;

pub struct OrderBook {
    pub bids: BTreeMap<Reverse<Price>, PriceLevel>,     // Sorted descending
    pub asks: BTreeMap<Price, PriceLevel>,              // Sorted ascending
    pub orders: HashMap<OrderId, Order>,
}

pub struct PriceLevel {
    pub price: Price,
    pub orders: VecDeque<Order>,
    pub total_quantity: Quantity,
}

#[derive(Clone, Debug)]
pub struct Order {
    pub order_id: OrderId,
    pub user_id: UserId,
    pub side: Side,
    pub price: Price,
    pub quantity: Quantity,
    pub filled: Quantity,
    pub timestamp: Timestamp,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    pub post_only: bool,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
        }
    }

    pub fn add_order(&mut self, order: Order) -> Result<()> {
        // Check for duplicate
        if self.orders.contains_key(&order.order_id) {
            return Err(Error::DuplicateOrderId(order.order_id));
        }

        // CORRECTED: Proper handling of Reverse wrapper
        let level = if order.side == Side::Buy {
            self.bids.entry(Reverse(order.price)).or_insert_with(|| PriceLevel {
                price: order.price,
                orders: VecDeque::new(),
                total_quantity: Quantity::zero(),
            })
        } else {
            self.asks.entry(order.price).or_insert_with(|| PriceLevel {
                price: order.price,
                orders: VecDeque::new(),
                total_quantity: Quantity::zero(),
            })
        };

        level.total_quantity = level.total_quantity + (order.quantity - order.filled);
        level.orders.push_back(order.clone());

        // Add to orders map
        self.orders.insert(order.order_id, order);

        Ok(())
    }

    pub fn remove_order(&mut self, order_id: &OrderId) -> Result<Order> {
        let order = self.orders.remove(order_id).ok_or(Error::OrderNotFound(*order_id))?;

        // Remove from price level
        if order.side == Side::Buy {
            if let Some(level) = self.bids.get_mut(&Reverse(order.price)) {
                level.orders.retain(|o| o.order_id != *order_id);
                level.total_quantity = level.total_quantity - (order.quantity - order.filled);

                if level.orders.is_empty() {
                    self.bids.remove(&Reverse(order.price));
                }
            }
        } else {
            if let Some(level) = self.asks.get_mut(&order.price) {
                level.orders.retain(|o| o.order_id != *order_id);
                level.total_quantity = level.total_quantity - (order.quantity - order.filled);

                if level.orders.is_empty() {
                    self.asks.remove(&order.price);
                }
            }
        }

        Ok(order)
    }

    pub fn best_bid(&self) -> Option<Price> {
        self.bids.keys().next().map(|Reverse(p)| *p)
    }

    pub fn best_ask(&self) -> Option<Price> {
        self.asks.keys().next().copied()
    }

    pub fn spread(&self) -> Option<Price> {
        match (self.best_ask(), self.best_bid()) {
            (Some(ask), Some(bid)) => Some(ask - bid),
            _ => None,
        }
    }

    pub fn get_order(&self, order_id: &OrderId) -> Option<&Order> {
        self.orders.get(order_id)
    }

    /// Lấy tham chiếu mutable tới PriceLevel tốt nhất ở phía đối diện
    /// (Taker Buy -> Lấy Best Ask, Taker Sell -> Lấy Best Bid)
    pub fn get_best_level_mut(&mut self, taker_side: Side) -> Option<&mut PriceLevel> {
        match taker_side {
            Side::Buy => {
                // Taker mua -> Cần lấy Ask thấp nhất (First Entry of Asks)
                let best_price = self.asks.keys().next().copied()?;
                self.asks.get_mut(&best_price)
            },
            Side::Sell => {
                // Taker bán -> Cần lấy Bid cao nhất (First Entry of Bids - nhờ Reverse)
                let best_price = *self.bids.keys().next()?; // best_price ở đây là Reverse<Price>
                self.bids.get_mut(&best_price)
            }
        }
    }

    /// Hàm dọn dẹp sau khi khớp lệnh: Xóa order khỏi map lookup và xóa level rỗng
    pub fn cleanup_after_match(&mut self, filled_order_id: OrderId, price: Price, side: Side, filled_qty: Quantity) {
        // 1. Xóa order khỏi hashmap tra cứu nhanh
        self.orders.remove(&filled_order_id);

        // 2. Cập nhật total_quantity của level (việc pop order khỏi queue đã làm ở matcher)
        // Tuy nhiên, để an toàn và chuẩn logic, ta nên để Matcher gọi hàm này
        // Ở đây ta giả định Matcher đã pop order ra rồi, ta chỉ cần check xem level có rỗng không để xóa

        match side {
            Side::Buy => { // Maker là Buy (nằm trong Bids)
                if let Some(level) = self.bids.get_mut(&Reverse(price)) {
                    // Logic update quantity nên làm lúc match, ở đây chỉ check empty
                    if level.orders.is_empty() {
                        self.bids.remove(&Reverse(price));
                    }
                }
            },
            Side::Sell => { // Maker là Sell (nằm trong Asks)
                if let Some(level) = self.asks.get_mut(&price) {
                    if level.orders.is_empty() {
                        self.asks.remove(&price);
                    }
                }
            }
        }
    }

    // Hàm helper cập nhật số lượng level (tránh matcher chọc trực tiếp field)
    pub fn decrease_level_quantity(&mut self, price: Price, side: Side, amount: Quantity) {
        match side {
            Side::Buy => {
                if let Some(level) = self.bids.get_mut(&Reverse(price)) {
                    level.total_quantity = level.total_quantity - amount;
                }
            },
            Side::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.total_quantity = level.total_quantity - amount;
                }
            }
        }
    }
}