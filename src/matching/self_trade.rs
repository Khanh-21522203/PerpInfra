use crate::matching::order_book::Order;

#[derive(Clone, Copy, Debug)]
pub enum SelfTradeAction {
    Allow,
    CancelMaker,
    CancelTaker,
    CancelBoth,
}

pub fn check_self_trade(maker: &Order, taker: &Order) -> SelfTradeAction {
    if maker.user_id == taker.user_id {
        SelfTradeAction::CancelMaker  // Default policy
    } else {
        SelfTradeAction::Allow
    }
}

// ADDED: Configurable self-trade policy
pub struct SelfTradePolicy {
    action: SelfTradeAction,
}

impl SelfTradePolicy {
    pub fn new(action: SelfTradeAction) -> Self {
        SelfTradePolicy { action }
    }

    pub fn check(&self, maker: &Order, taker: &Order) -> SelfTradeAction {
        if maker.user_id == taker.user_id {
            self.action
        } else {
            SelfTradeAction::Allow
        }
    }
}