use lazy_static::lazy_static;
use prometheus::{
    Counter, Histogram, HistogramOpts, IntGauge, Registry,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // Order metrics
    pub static ref ORDERS_SUBMITTED: Counter = Counter::new(
        "orders_submitted_total",
        "Total number of orders submitted"
    ).unwrap();

    pub static ref ORDERS_MATCHED: Counter = Counter::new(
        "orders_matched_total",
        "Total number of orders matched"
    ).unwrap();

    pub static ref ORDERS_REJECTED: Counter = Counter::new(
        "orders_rejected_total",
        "Total number of orders rejected"
    ).unwrap();

    // Trade metrics
    pub static ref TRADES_EXECUTED: Counter = Counter::new(
        "trades_executed_total",
        "Total number of trades executed"
    ).unwrap();

    pub static ref TRADE_VOLUME: Counter = Counter::new(
        "trade_volume_total",
        "Total trading volume"
    ).unwrap();

    // Liquidation metrics
    pub static ref LIQUIDATIONS_EXECUTED: Counter = Counter::new(
        "liquidations_executed_total",
        "Total number of liquidations"
    ).unwrap();

    pub static ref INSURANCE_FUND_BALANCE: IntGauge = IntGauge::new(
        "insurance_fund_balance",
        "Current insurance fund balance"
    ).unwrap();

    // Latency metrics
    pub static ref ORDER_PROCESSING_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "order_processing_latency_seconds",
            "Order processing latency"
        ).buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0])
    ).unwrap();

    pub static ref MATCHING_LATENCY: Histogram = Histogram::with_opts(
        HistogramOpts::new(
            "matching_latency_seconds",
            "Order matching latency"
        ).buckets(vec![0.0001, 0.0005, 0.001, 0.005, 0.01])
    ).unwrap();
}

pub fn register_metrics() {
    REGISTRY.register(Box::new(ORDERS_SUBMITTED.clone())).unwrap();
    REGISTRY.register(Box::new(ORDERS_MATCHED.clone())).unwrap();
    REGISTRY.register(Box::new(ORDERS_REJECTED.clone())).unwrap();
    REGISTRY.register(Box::new(TRADES_EXECUTED.clone())).unwrap();
    REGISTRY.register(Box::new(TRADE_VOLUME.clone())).unwrap();
    REGISTRY.register(Box::new(LIQUIDATIONS_EXECUTED.clone())).unwrap();
    REGISTRY.register(Box::new(INSURANCE_FUND_BALANCE.clone())).unwrap();
    REGISTRY.register(Box::new(ORDER_PROCESSING_LATENCY.clone())).unwrap();
    REGISTRY.register(Box::new(MATCHING_LATENCY.clone())).unwrap();
}