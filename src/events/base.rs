use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use uuid::Uuid;
use crate::error::{Error, Result};
use crate::types::ids::{EventId, MarketId, UserId};
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
    pub payload: EventPayload,      // FIX IGD-S-037: Typed payload for event-specific data
    pub checksum: String,           // FIX IGD-S-002: Integrity verification
}

impl BaseEvent {
    pub fn new(event_type: EventType, market_id: MarketId) -> Self {
        let mut event = BaseEvent {
            event_id: EventId::new(),
            event_type,
            version: 1,
            timestamp: Timestamp::now(),
            market_id,
            sequence: 0, // Set by event log
            correlation_id: CorrelationId::new(),
            metadata: EventMetadata::default(),
            payload: EventPayload::Empty,  // FIX IGD-S-037
            checksum: String::new(),       // FIX IGD-S-002
        };
        event.checksum = event.calculate_checksum();
        event
    }

    pub fn calculate_checksum(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.event_id.0.as_bytes());
        hasher.update(self.sequence.to_le_bytes());
        hasher.update(self.timestamp.physical.to_le_bytes());
        hasher.update(format!("{:?}", self.event_type).as_bytes());
        hasher.finalize()
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect()
    }

    /// Verify event checksum
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }

    /// Create event with typed payload
    pub fn with_payload(event_type: EventType, market_id: MarketId, payload: EventPayload) -> Self {
        let mut event = Self::new(event_type, market_id);
        event.payload = payload;
        event.checksum = event.calculate_checksum();
        event
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EventPayload {
    Empty,
    OrderSubmit(Box<crate::events::order::OrderSubmit>),
    OrderCancel(Box<crate::events::order::OrderCancel>),
    Trade(Box<crate::events::trade::TradeEvent>),
    PriceSnapshot(Box<crate::events::price::PriceSnapshot>),
    Funding(Box<crate::events::funding::FundingEvent>),
    Liquidation(Box<crate::events::liquidation::LiquidationTriggered>),
    BalanceUpdate(Box<crate::events::balance::BalanceUpdate>),
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