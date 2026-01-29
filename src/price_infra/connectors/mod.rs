pub mod binance;
pub mod coinbase;
pub mod kraken;

use async_trait::async_trait;
use crate::price_infra::RawPriceUpdate;
use crate::error::Result;

#[async_trait]
pub trait PriceConnector: Send + Sync {
    async fn connect(&mut self) -> Result<()>;
    async fn next_price(&mut self) -> Result<RawPriceUpdate>;
    fn is_healthy(&self) -> bool;
    fn source_id(&self) -> &str;
}