use crate::error::Result;
use crate::events::order::Side;
use crate::types::ids::{MarketId, UserId};
use crate::types::position::Position;
use crate::types::price::Price;
use crate::types::quantity::Quantity;
use std::collections::HashMap;

pub struct PositionManager {
    positions: HashMap<UserId, Position>,
    market_id: MarketId,
}

impl PositionManager {
    pub fn new() -> Self {
        PositionManager {
            positions: HashMap::new(),
            market_id: MarketId::from_string("BTC-PERP").expect("REASON"), // Default, should be passed in constructor
        }
    }

    pub fn new_with_market(market_id: MarketId) -> Self {
        PositionManager {
            positions: HashMap::new(),
            market_id,
        }
    }

    pub fn get_position(&self, user_id: &UserId) -> Option<&Position> {
        self.positions.get(user_id)
    }

    pub fn get_position_mut(&mut self, user_id: &UserId) -> Option<&mut Position> {
        self.positions.get_mut(user_id)
    }

    pub fn get_or_create_position(&mut self, user_id: UserId) -> &mut Position {
        let market_id = self.market_id;
        self.positions.entry(user_id).or_insert_with(|| Position::new(user_id, market_id))
    }

    pub fn set_position(&mut self, user_id: UserId, position: Position) {
        self.positions.insert(user_id, position);
    }

    pub fn remove_position(&mut self, user_id: &UserId) -> Option<Position> {
        self.positions.remove(user_id)
    }

    pub fn update_position(
        &mut self,
        user_id: UserId,
        trade_side: Side,
        trade_quantity: Quantity,
        trade_price: Price,
    ) -> Result<()> {
        let position = self.get_or_create_position(user_id);

        use crate::risk::pnl::PnLCalculator;
        PnLCalculator::update_position(position, trade_side, trade_quantity, trade_price);

        Ok(())
    }

    pub fn get_all_positions(&self) -> Vec<&Position> {
        self.positions.values().collect()
    }

    pub fn get_all_positions_mut(&mut self) -> Vec<&mut Position> {
        self.positions.values_mut().collect()
    }
}