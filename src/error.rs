use thiserror::Error;
use crate::types::balance::Balance;
use crate::types::ids::{AccountId, EventId, OrderId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;

#[derive(Error, Debug)]
pub enum Error {
    // Event Log Errors
    #[error("Kafka error: {0}")]
    KafkaError(String),

    #[error("Event deserialization failed: {0}")]
    DeserializationError(String),

    #[error("Empty payload")]
    EmptyPayload,

    #[error("Invalid correlation ID")]
    InvalidCorrelationId,

    #[error("Invalid checksum")]
    InvalidChecksum,

    #[error("Checksum mismatch for event: {event_id:?}")]
    ChecksumMismatch {
        event_id: EventId,
    },

    #[error("Invalid event payload: expected {expected}, found {found}")]
    InvalidEventPayload {
        expected: String,
        found: String,
    },
    
    #[error("No more events available")]
    NoMoreEvents,

    #[error("Sequence gap: expected {expected}, got {actual}")]
    SequenceGap {
        expected: u64,
        actual: u64,
    },

    #[error("Sequence mismatch: expected {expected}, got {actual}")]
    SequenceMismatch {
        expected: u64,
        actual: u64,
    },

    #[error("Serialization error: {0}")]
    SerializationError(String),

    // Price Infrastructure Errors
    #[error("Insufficient fresh prices: need at least 2, got {0}")]
    InsufficientFreshPrices(usize),

    #[error("All prices are outliers")]
    AllPricesAreOutliers,

    #[error("Weighted median calculation failed")]
    WeightedMedianFailed,

    #[error("Price connector not connected")]
    NotConnected,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Circuit breaker triggered: {0:?}")]
    CircuitBreakerTriggered(CircuitBreakerReason),

    #[error("Max reconnect attempts exceeded")]
    MaxReconnectAttemptsExceeded,

    // Order Validation Errors
    #[error("Invalid tick size")]
    InvalidTickSize,

    #[error("Invalid lot size")]
    InvalidLotSize,

    #[error("Invalid price")]
    InvalidPrice,

    #[error("Invalid quantity")]
    InvalidQuantity,

    #[error("Below minimum order size")]
    BelowMinOrderSize,

    #[error("Above maximum order size")]
    AboveMaxOrderSize,

    #[error("Market order cannot be post-only")]
    MarketOrderCannotBePostOnly,

    #[error("Market order requires slippage limit")]
    MarketOrderRequiresSlippageLimit,

    #[error("Limit order requires price")]
    LimitOrderRequiresPrice,

    // Order Book Errors
    #[error("Duplicate order ID: {0}")]
    DuplicateOrderId(OrderId),

    #[error("Order not found: {0}")]
    OrderNotFound(OrderId),

    #[error("Order overfilled: order_id={order_id}, filled={filled}, quantity={quantity}")]
    OrderOverfilled {
        order_id: OrderId,
        filled: Quantity,
        quantity: Quantity,
    },

    #[error("Price level inconsistent: price={price}, expected={expected}, actual={actual}")]
    PriceLevelInconsistent {
        price: Price,
        expected: Quantity,
        actual: Quantity,
    },

    // Risk Errors
    #[error("Insufficient margin: required={required}, available={available}")]
    InsufficientMargin {
        required: Balance,
        available: Balance,
    },

    #[error("Leverage exceeded: leverage={leverage}, max={max}")]
    LeverageExceeded {
        leverage: f64,
        max: f64,
    },

    #[error("Position limit exceeded")]
    PositionLimitExceeded,

    #[error("Reduce-only violation")]
    ReduceOnlyViolation,

    // Liquidation Errors
    #[error("Liquidation failed: no liquidity")]
    LiquidationFailedNoLiquidity,

    #[error("Liquidation rate limit exceeded")]
    LiquidationRateLimitExceeded,

    #[error("Insurance fund depleted: required={required}, available={available}")]
    InsuranceFundDepleted {
        required: Balance,
        available: Balance,
    },

    // Funding Errors
    #[error("Funding not zero-sum: sum={sum}")]
    FundingNotZeroSum { sum: i64 },

    // Settlement Errors
    #[error("Account not found: {0:?}")]
    AccountNotFound(AccountId),

    #[error("Account already exists: {0:?}")]
    AccountAlreadyExists(AccountId),

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Insufficient available balance")]
    InsufficientAvailableBalance,

    #[error("Double-entry imbalance: debits={debits}, credits={credits}")]
    DoubleEntryImbalance {
        debits: i64,
        credits: i64,
    },

    #[error("Reconciliation failed: expected={expected}, actual={actual}")]
    ReconciliationFailed {
        expected: Balance,
        actual: Balance,
    },

    // Invariant Errors
    #[error("Invariant violation: {0}")]
    InvariantViolation(InvariantViolation),

    #[error("Conservation of value violated: expected={expected}, actual={actual}")]
    ConservationOfValueViolation {
        expected: Balance,
        actual: Balance,
    },

    // System Errors
    #[error("Kill switch active")]
    KillSwitchActive,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("No snapshot found")]
    NoSnapshotFound,

    #[error("Configuration error: {0}")]
    ConfigError(String),

    // Arithmetic Errors
    #[error("Overflow in {operation}")]
    Overflow { operation: String },

    #[error("Division by zero")]
    DivisionByZero,

    // IO Errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    // Event Version Errors
    #[error("Unsupported event version: {event_version}, max supported: {max_supported}")]
    UnsupportedEventVersion {
        event_version: u32,
        max_supported: u32,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub invariant: &'static str,
    pub details: String,
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.invariant, self.details)
    }
}

#[derive(Debug, Clone)]
pub enum CircuitBreakerReason {
    PriceMovement(f64),
    MarkIndexDeviation(f64),
    AllSourcesStale,
}