use crate::config::market::MarketConfig;
use crate::events::order::{OrderSubmit, OrderType, Side};
use crate::error::{Error, Result};
use crate::types::price::Price;
use crate::types::quantity::Quantity;

pub struct OrderValidator {
    config: MarketConfig,
}

impl OrderValidator {
    pub fn new(config: MarketConfig) -> Self {
        OrderValidator { config }
    }

    pub fn validate(&self, order: &OrderSubmit) -> Result<()> {
        // Observability: Record order submission
        use crate::observability::metrics::*;
        let side = match order.side {
            Side::Buy => "buy",
            Side::Sell => "sell",
        };
        let order_type = if order.price.is_some() { "limit" } else { "market" };
        ORDERS_SUBMITTED.with_label_values(&[side, order_type]).inc();

        // Validate price (if limit order)
        if let Some(price) = order.price {
            self.validate_price(price)?;
        }

        // Validate quantity
        self.validate_quantity(order.quantity)?;

        // Validate order type constraints
        match self.validate_order_type_constraints(order) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Observability: Record rejection
                let reason = match &e {
                    Error::InsufficientMargin { .. } => "insufficient_margin",
                    Error::InvalidPrice => "invalid_price",
                    Error::InvalidQuantity => "invalid_quantity",
                    _ => "other",
                };
                ORDERS_REJECTED.with_label_values(&[reason]).inc();
                Err(e)
            }
        }
    }

    fn validate_price(&self, price: Price) -> Result<()> {
        // Check tick size
        let tick_size = self.config.tick_size;
        if price.raw_value() % tick_size.raw_value() != 0 {
            return Err(Error::InvalidTickSize);
        }

        // Check price bounds
        if price <= Price::zero() {
            return Err(Error::InvalidPrice);
        }

        Ok(())
    }

    fn validate_quantity(&self, quantity: Quantity) -> Result<()> {
        // Check lot size
        let lot_size = self.config.lot_size;
        if quantity.raw_value() % lot_size.raw_value() != 0 {
            return Err(Error::InvalidLotSize);
        }

        // Check min/max bounds
        if quantity < self.config.min_order_size {
            return Err(Error::BelowMinOrderSize);
        }

        if quantity > self.config.max_order_size {
            return Err(Error::AboveMaxOrderSize);
        }

        Ok(())
    }

    fn validate_order_type_constraints(&self, order: &OrderSubmit) -> Result<()> {
        match order.order_type {
            OrderType::Market => {
                if order.post_only {
                    return Err(Error::MarketOrderCannotBePostOnly);
                }
                if order.slippage_limit.is_none() {
                    return Err(Error::MarketOrderRequiresSlippageLimit);
                }
            }
            OrderType::Limit => {
                if order.price.is_none() {
                    return Err(Error::LimitOrderRequiresPrice);
                }
            }
        }

        Ok(())
    }
}