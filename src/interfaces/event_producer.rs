use crate::events::base::BaseEvent;
use crate::error::Result;
use async_trait::async_trait;

#[async_trait]
pub trait EventProducer {
    async fn produce(&self, event: BaseEvent) -> Result<u64>;
}