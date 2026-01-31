use crate::events::order::OrderSubmit;
use crate::types::ids::OrderId;
use crate::error::Result;

pub trait OrderSubmitter {
    fn submit_order(&mut self, order: OrderSubmit) -> Result<OrderId>;
}