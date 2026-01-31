use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, HistogramOpts, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec, Opts, register_counter, register_counter_vec, register_gauge,
    register_gauge_vec, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge, register_int_gauge_vec,
};

lazy_static! {
    // Order metrics
    pub static ref ORDERS_SUBMITTED: IntCounterVec = register_int_counter_vec!(
        "perpinfra_orders_submitted_total",
        "Total number of orders submitted",
        &["side", "order_type"]
    ).unwrap();

    pub static ref ORDERS_REJECTED: IntCounterVec = register_int_counter_vec!(
        "perpinfra_orders_rejected_total",
        "Total number of orders rejected",
        &["reason"]
    ).unwrap();

    pub static ref ORDERS_ACCEPTED: IntCounter = register_int_counter!(
        "perpinfra_orders_accepted_total",
        "Total number of orders accepted"
    ).unwrap();

    pub static ref ORDERS_CANCELLED: IntCounter = register_int_counter!(
        "perpinfra_orders_cancelled_total",
        "Total number of orders cancelled"
    ).unwrap();

    pub static ref TRADES_PROCESSED: IntCounter = register_int_counter!(
        "perpinfra_trades_processed_total",
        "Total number of trades processed by event processor"
    ).unwrap();

    pub static ref FUNDING_EVENTS_PROCESSED: IntCounter = register_int_counter!(
        "perpinfra_funding_events_processed_total",
        "Total number of funding events processed"
    ).unwrap();

    pub static ref DEPOSITS_PROCESSED: IntCounter = register_int_counter!(
        "perpinfra_deposits_processed_total",
        "Total number of deposits processed"
    ).unwrap();

    pub static ref WITHDRAWALS_PROCESSED: IntCounter = register_int_counter!(
        "perpinfra_withdrawals_processed_total",
        "Total number of withdrawals processed"
    ).unwrap();

    pub static ref VOLUME_TRADED: Counter = register_counter!(
        "perpinfra_volume_traded_total",
        "Total volume traded"
    ).unwrap();

    pub static ref DEPOSIT_VOLUME: Counter = register_counter!(
        "perpinfra_deposit_volume_total",
        "Total deposit volume"
    ).unwrap();

    pub static ref WITHDRAWAL_VOLUME: Counter = register_counter!(
        "perpinfra_withdrawal_volume_total",
        "Total withdrawal volume"
    ).unwrap();

    // Trade metrics
    pub static ref TRADES_EXECUTED: IntCounter = register_int_counter!(
        "perpinfra_trades_executed_total",
        "Total number of trades executed"
    ).unwrap();

    pub static ref TRADE_VOLUME: CounterVec = register_counter_vec!(
        Opts::new("perpinfra_trade_volume_usd", "Total trade volume in USD"),
        &["market"]
    ).unwrap();

    // Matching metrics
    pub static ref MATCHING_LATENCY: HistogramVec = register_histogram_vec!(
        HistogramOpts::new("perpinfra_matching_latency_seconds", "Order matching latency"),
        &["order_type"]
    ).unwrap();

    // Liquidation metrics
    pub static ref LIQUIDATIONS_EXECUTED: IntCounterVec = register_int_counter_vec!(
        "perpinfra_liquidations_executed_total",
        "Total number of liquidations executed",
        &["type"]  // "full" or "partial"
    ).unwrap();

    pub static ref LIQUIDATION_VOLUME: Counter = register_counter!(
        "perpinfra_liquidation_volume_usd",
        "Total liquidation volume in USD"
    ).unwrap();

    // Insurance fund metrics
    pub static ref INSURANCE_FUND_BALANCE: IntGauge = register_int_gauge!(
        "perpinfra_insurance_fund_balance",
        "Current insurance fund balance"
    ).unwrap();

    // Price metrics
    pub static ref MARK_PRICE: GaugeVec = register_gauge_vec!(
        Opts::new("perpinfra_mark_price", "Current mark price"),
        &["market"]
    ).unwrap();

    pub static ref INDEX_PRICE: GaugeVec = register_gauge_vec!(
        Opts::new("perpinfra_index_price", "Current index price"),
        &["market"]
    ).unwrap();

    pub static ref PRICE_STALENESS: IntGaugeVec = register_int_gauge_vec!(
        "perpinfra_price_staleness_seconds",
        "Price staleness in seconds",
        &["source"]
    ).unwrap();

    // Funding metrics
    pub static ref FUNDING_RATE: GaugeVec = register_gauge_vec!(
        Opts::new("perpinfra_funding_rate", "Current funding rate"),
        &["market"]
    ).unwrap();

    // System metrics
    pub static ref CIRCUIT_BREAKER_STATUS: IntGaugeVec = register_int_gauge_vec!(
        "perpinfra_circuit_breaker_status",
        "Circuit breaker status (0=normal, 1=triggered)",
        &["type"]
    ).unwrap();

    pub static ref KILL_SWITCH_ACTIVE: IntGauge = register_int_gauge!(
        "perpinfra_kill_switch_active",
        "Kill switch status (0=inactive, 1=active)"
    ).unwrap();

    // Order book metrics
    pub static ref ORDER_BOOK_DEPTH: IntGaugeVec = register_int_gauge_vec!(
        "perpinfra_order_book_depth",
        "Order book depth (number of orders)",
        &["side"]
    ).unwrap();

    pub static ref ORDER_BOOK_SPREAD: Gauge = register_gauge!(
        "perpinfra_order_book_spread",
        "Current bid-ask spread"
    ).unwrap();
}

/// Record order submission
pub fn record_order_submitted(side: &str, order_type: &str) {
    ORDERS_SUBMITTED
        .with_label_values(&[side, order_type])
        .inc();
}

/// Record order rejection
pub fn record_order_rejected(reason: &str) {
    ORDERS_REJECTED.with_label_values(&[reason]).inc();
}

/// Record trade execution
pub fn record_trade(volume_usd: f64, market: &str) {
    TRADES_EXECUTED.inc();
    TRADE_VOLUME.with_label_values(&[market]).inc_by(volume_usd);
}

/// Record liquidation
pub fn record_liquidation(liquidation_type: &str, volume_usd: f64) {
    LIQUIDATIONS_EXECUTED
        .with_label_values(&[liquidation_type])
        .inc();
    LIQUIDATION_VOLUME.inc_by(volume_usd);
}

/// Update insurance fund balance
pub fn update_insurance_fund_balance(balance: i64) {
    INSURANCE_FUND_BALANCE.set(balance);
}

/// Update prices
pub fn update_prices(market: &str, mark: f64, index: f64) {
    MARK_PRICE.with_label_values(&[market]).set(mark);
    INDEX_PRICE.with_label_values(&[market]).set(index);
}