use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::types::ids::{MarketId, UserId};
use crate::types::timestamp::Timestamp;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BaseEvent {
    pub event_id: EventId,
    pub event_type: EventType,
    pub version: u32,
    pub timestamp: Timestamp,
    pub market_id: MarketId,
    pub sequence: u64,
    pub correlation_id: CorrelationId,
    pub metadata: EventMetadata,
}

impl BaseEvent {
    pub fn new(event_type: EventType, market_id: MarketId) -> Self {
        BaseEvent {
            event_id: EventId::new(),
            event_type,
            version: 1,
            timestamp: Timestamp::now(),
            market_id,
            sequence: 0, // Set by event log
            correlation_id: CorrelationId::new(),
            metadata: EventMetadata::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
    pub fn new() -> Self {
        EventId(Uuid::new_v4())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CorrelationId(pub Uuid);

impl CorrelationId {
    pub fn new() -> Self {
        CorrelationId(Uuid::new_v4())
    }

    pub fn from_header(header: &str) -> Result<Self> {
        Uuid::parse_str(header)
            .map(CorrelationId)
            .map_err(|_| Error::InvalidCorrelationId)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventMetadata {
    pub source: String,
    pub user_id: Option<UserId>,
    pub idempotency_key: Option<String>,
}

impl Default for EventMetadata {
    fn default() -> Self {
        EventMetadata {
            source: "core".to_string(),
            user_id: None,
            idempotency_key: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    OrderSubmit,
    OrderCancel,
    OrderAmend,
    OrderAccepted,
    OrderRejected,
    Trade,
    PriceSnapshot,
    Funding,
    Liquidation,
    BalanceUpdate,
    InvariantViolation,
    KillSwitchActivated,
    CircuitBreakerTriggered,
}