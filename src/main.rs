use tokio::signal;
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};
use axum::Server;
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use std::net::SocketAddr;
use PerpInfra::config::loader::AppConfig;
use PerpInfra::core::event_processor::EventProcessor;
use PerpInfra::error::{Error, Result};
use PerpInfra::events::base::{BaseEvent, EventType};
use PerpInfra::events::price::PriceSnapshot;
use PerpInfra::funding::ticker::FundingTicker;
use PerpInfra::liquidation::insurance_fund::InsuranceFund;
use PerpInfra::price_infra::aggregator::PriceAggregator;
use PerpInfra::price_infra::connectors::binance::BinanceConnector;
use PerpInfra::price_infra::connectors::coinbase::CoinbaseConnector;
use PerpInfra::price_infra::connectors::kraken::KrakenConnector;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting PerpInfra v1.0.0");

    // Load configuration
    let env = std::env::var("ENV").unwrap_or_else(|_| "development".to_string());
    info!("Loading configuration for environment: {}", env);
    let config = ConfigLoader::load(&env)?;

    // Validate configuration
    validate_config(&config)?;
    info!("Configuration loaded and validated");

    // Initialize market
    let market_id = MarketId::from_string(&config.market.symbol);
    info!("Initializing market: {}", config.market.symbol);

    // ============================================================================
    // PHASE 1: CREATE CORE COMPONENTS
    // ============================================================================

    // Task supervisor for monitoring background tasks
    let mut task_supervisor = TaskSupervisor::new();
    info!("Task supervisor initialized");

    // Kill switch for emergency shutdown
    let kill_switch = Arc::new(KillSwitch::new());
    info!("Kill switch initialized");

    // Event log (Kafka)
    info!("Connecting to Kafka at {}", config.kafka.brokers);
    let event_consumer = EventConsumer::new(
        &config.kafka.brokers,
        &config.kafka.topic,
        &config.kafka.group_id,
    ).await?;

    let event_producer = Arc::new(KafkaEventProducer::new(
        &config.kafka.brokers,
        &config.kafka.topic,
    )?);
    info!("Kafka connection established");

    // Snapshot manager for fast recovery
    let snapshot_manager = Arc::new(SnapshotManager::new("./snapshots"));

    // ============================================================================
    // PHASE 2: CREATE ENGINE COMPONENTS
    // ============================================================================

    // Settlement layer
    let balance_manager = Arc::new(RwLock::new(BalanceManager::new()));
    let position_manager = Arc::new(RwLock::new(PositionManager::new()));
    info!("Settlement layer initialized");

    // Matching engine
    let order_book = Arc::new(RwLock::new(OrderBook::new()));
    let matcher = Arc::new(RwLock::new(Matcher::new(
        OrderBook::new(),
        config.fees.clone(),
        market_id,
    )));
    info!("Matching engine initialized");

    // Risk engine
    let margin_calculator = Arc::new(MarginCalculator::new(config.risk.clone()));
    info!("Risk engine initialized");

    // Funding engine
    let funding_rate_calculator = FundingRateCalculator::new(config.funding.clone());
    let funding_applicator = Arc::new(FundingApplicator::new(
        funding_rate_calculator,
        Duration::from_secs(28800), // 8 hours
    ));
    info!("Funding engine initialized");

    // Liquidation engine
    let insurance_fund = Arc::new(InsuranceFund::new());
    let liquidation_detector = Arc::new(LiquidationDetector::new(margin_calculator.clone()));
    let liquidation_executor = Arc::new(LiquidationExecutor::new(
        market_id,
        insurance_fund.clone(),
    ));
    info!("Liquidation engine initialized");

    // ============================================================================
    // PHASE 3: CREATE EVENT PROCESSOR
    // ============================================================================

    let mut event_processor = EventProcessor::new_with_dependencies(
        market_id,
        balance_manager.clone(),
        position_manager.clone(),
        order_book.clone(),
        matcher.clone(),
        margin_calculator.clone(),
        funding_applicator.clone(),
        liquidation_executor.clone(),
        event_producer.clone(),
    );

    // Try to restore from snapshot
    match snapshot_manager.load_latest(market_id).await {
        Ok(snapshot) => {
            info!("Restoring from snapshot at sequence {}", snapshot.sequence);
            event_processor.restore_from_snapshot(&snapshot)?;
            info!("State restored from snapshot");
        }
        Err(_) => {
            info!("No snapshot found, starting from beginning");
        }
    }

    info!("Event processor initialized");

    // ============================================================================
    // PHASE 4: START PRICE INFRASTRUCTURE
    // ============================================================================

    info!("Connecting to price sources...");
    let mut binance = BinanceConnector::new("btcusdt");
    let mut coinbase = CoinbaseConnector::new("BTC-USD");
    let mut kraken = KrakenConnector::new("XBTUSD");

    binance.connect().await?;
    coinbase.connect().await?;
    kraken.connect().await?;

    let price_aggregator = Arc::new(RwLock::new(PriceAggregator::new(vec![
        Box::new(binance),
        Box::new(coinbase),
        Box::new(kraken),
    ])));
    info!("Price infrastructure connected");

    // Channel for price updates (broadcast for multiple consumers)
    let (price_tx, _) = tokio::sync::broadcast::channel(100);

    // Spawn price aggregation task
    let price_agg_clone = price_aggregator.clone();
    let price_producer = event_producer.clone();
    let price_market_id = market_id;
    task_supervisor.spawn("price_aggregation", async move {
        let mut interval = interval(Duration::from_millis(100)); // 10 Hz
        loop {
            interval.tick().await;

            let aggregator = price_agg_clone.read().await;
            match aggregator.aggregate().await {
                Ok(snapshot) => {
                    // Send to price channel (broadcast)
                    let _ = price_tx.send(snapshot.clone());

                    // Emit price event
                    let price_event = PriceSnapshot {
                        base: BaseEvent::new(
                            EventType::PriceUpdate,
                            price_market_id,
                        ),
                        mark_price: snapshot.mark_price,
                        index_price: snapshot.index_price,
                        source_prices: snapshot.source_prices,
                        aggregation_method: snapshot.aggregation_method,
                        staleness_flags: snapshot.staleness_flags,
                    };

                    if let Err(e) = price_producer.produce(price_event.base).await {
                        error!("Failed to produce price event: {:?}", e);
                    }
                }
                Err(e) => {
                    error!("Price aggregation failed: {:?}", e);
                }
            }
        }
    });

    // ============================================================================
    // PHASE 5: START FUNDING TICKER
    // ============================================================================

    let funding_ticker = FundingTicker::new(
        funding_applicator.clone(),
        Duration::from_secs(28800), // 8 hours
    );

    let funding_balance_mgr = balance_manager.clone();
    let funding_position_mgr = position_manager.clone();
    let funding_market_id = market_id;
    let mut funding_price_rx = price_tx.subscribe();
    task_supervisor.spawn("funding_ticker", async move {
        let mut interval = interval(Duration::from_secs(28800)); // 8 hours
        loop {
            interval.tick().await;

            info!("Applying funding payments");
            let positions = funding_position_mgr.read().await;
            let mut balance_mgr = funding_balance_mgr.write().await;

            // Get current mark and index prices
            match funding_price_rx.try_recv() {
                Ok(price_snapshot) => {
                    let positions_vec: Vec<_> = positions.positions.values().cloned().collect();
                    match funding_ticker.applicator.apply_funding(
                        &positions_vec,
                        price_snapshot.mark_price,
                        price_snapshot.index_price,
                        &mut *balance_mgr,
                        funding_market_id,
                    ) {
                        Ok(funding_event) => {
                            info!("Funding applied: rate={:.6}, payments={}", 
                                  funding_event.funding_rate.to_f64(),
                                  funding_event.payments.len());
                        }
                        Err(e) => {
                            error!("Funding application failed: {:?}", e);
                        }
                    }
                }
                Err(_) => {
                    warn!("No price data available for funding");
                }
            }
        }
    });

    // ============================================================================
    // PHASE 6: START LIQUIDATION MONITOR
    // ============================================================================

    let liq_detector = liquidation_detector.clone();
    let liq_executor = liquidation_executor.clone();
    let liq_balance_mgr = balance_manager.clone();
    let liq_position_mgr = position_manager.clone();
    let liq_matcher = matcher.clone();
    let liq_producer = event_producer.clone();
    let liq_market_id = market_id;
    let mut liq_price_rx = price_tx.subscribe();
    task_supervisor.spawn("liquidation_monitor", async move {
        let mut interval = interval(Duration::from_secs(1)); // Check every second
        loop {
            interval.tick().await;

            // Get current price
            match liq_price_rx.try_recv() {
                Ok(price_snapshot) => {
                    let positions = liq_position_mgr.read().await;
                    let balance_mgr = liq_balance_mgr.read().await;
                    let positions_vec: Vec<_> = positions.positions.values().cloned().collect();

                    match liq_detector.detect_liquidations(
                        &positions_vec,
                        price_snapshot.mark_price,
                        &*balance_mgr,
                    ) {
                        Ok(candidates) => {
                            if !candidates.is_empty() {
                                warn!("Detected {} liquidation candidates", candidates.len());

                                // Emit liquidation events to Kafka (event-driven approach)
                                // This maintains single-writer principle - EventProcessor will handle execution
                                for candidate in candidates {
                                    let liquidation_event = crate::events::liquidation::LiquidationTriggered {
                                        base: crate::events::base::BaseEvent::new(
                                            crate::events::base::EventType::Liquidation,
                                            liq_market_id,
                                        ),
                                        user_id: candidate.user_id,
                                        position_size: candidate.position_size,
                                        mark_price: price_snapshot.mark_price,
                                        maintenance_margin: candidate.maintenance_margin,
                                        account_value: candidate.account_value,
                                    };

                                    if let Err(e) = liq_producer.produce(liquidation_event.base).await {
                                        error!("Failed to produce liquidation event: {:?}", e);
                                    } else {
                                        info!("Liquidation event emitted for user={:?}", candidate.user_id);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Liquidation detection failed: {:?}", e);
                        }
                    }
                }
                Err(_) => {
                    // No price update, skip this cycle
                }
            }
        }
    });

    // ============================================================================
    // PHASE 7: START INVARIANT MONITOR
    // ============================================================================

    let invariant_monitor = InvariantMonitor::new(kill_switch.clone());
    let inv_order_book = order_book.clone();
    let inv_balance_mgr = balance_manager.clone();
    let inv_position_mgr = position_manager.clone();
    let mut inv_price_rx = price_tx.subscribe();
    task_supervisor.spawn("invariant_monitor", async move {
        let mut interval = interval(Duration::from_secs(1)); // Check every second
        loop {
            interval.tick().await;

            let order_book_guard = inv_order_book.read().await;
            let balance_mgr_guard = inv_balance_mgr.read().await;
            let position_mgr_guard = inv_position_mgr.read().await;

            // Get current price
            match inv_price_rx.try_recv() {
                Ok(price_snapshot) => {
                    let positions_vec: Vec<_> = position_mgr_guard.positions.values().cloned().collect();

                    if let Err(e) = invariant_monitor.check_all_invariants(
                        &*order_book_guard,
                        &*balance_mgr_guard,
                        &positions_vec,
                        price_snapshot.mark_price,
                    ) {
                        error!("INVARIANT VIOLATION: {:?}", e);
                        kill_switch.activate(format!("Invariant violation: {:?}", e));
                    }
                }
                Err(_) => {
                    // No price update, skip this cycle
                }
            }
        }
    });

    // ============================================================================
    // PHASE 8: START REST API SERVER
    // ============================================================================

    let api_state = Arc::new(ApiState {
        balance_manager: balance_manager.clone(),
        position_manager: position_manager.clone(),
    });

    let app = create_router(api_state);
    let api_addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();

    task_supervisor.spawn("rest_api_server", async move {
        info!("REST API listening on {}", api_addr);
        Server::bind(&api_addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    // ============================================================================
    // PHASE 9: START METRICS EXPORTER
    // ============================================================================

    let metrics_app = axum::Router::new()
        .route("/metrics", axum::routing::get(metrics_handler));
    let metrics_addr: SocketAddr = "0.0.0.0:9090".parse().unwrap();

    task_supervisor.spawn("metrics_exporter", async move {
        info!("Metrics endpoint listening on {}/metrics", metrics_addr);
        Server::bind(&metrics_addr)
            .serve(metrics_app.into_make_service())
            .await
            .unwrap();
    });

    // ============================================================================
    // PHASE 10: START SNAPSHOT CREATION TASK
    // ============================================================================

    let snapshot_mgr = snapshot_manager.clone();
    let snapshot_balance_mgr = balance_manager.clone();
    let snapshot_position_mgr = position_manager.clone();
    let snapshot_market_id = market_id;
    let mut snapshot_price_rx = price_tx.subscribe();

    // Create a channel to get last_sequence from event processor
    let (snapshot_seq_tx, mut snapshot_seq_rx) = mpsc::channel::<u64>(1);

    task_supervisor.spawn("snapshot_creator", async move {
        let mut interval = interval(Duration::from_secs(3600)); // Every hour
        loop {
            interval.tick().await;

            info!("Creating snapshot");
            let balance_mgr = snapshot_balance_mgr.read().await;
            let position_mgr = snapshot_position_mgr.read().await;

            // Get current price
            match snapshot_price_rx.try_recv() {
                Ok(price_snapshot) => {
                    let positions_vec: Vec<_> = position_mgr.positions.values().cloned().collect();

                    // Get last sequence from channel (sent by main loop)
                    let last_sequence = snapshot_seq_rx.try_recv().unwrap_or(0);

                    match snapshot_mgr.create_snapshot(
                        last_sequence,
                        snapshot_market_id,
                        &*balance_mgr,
                        &positions_vec,
                        price_snapshot.mark_price,
                        price_snapshot.index_price,
                    ) {
                        Ok(snapshot) => {
                            match snapshot_mgr.save_snapshot(&snapshot).await {
                                Ok(_) => {
                                    info!("Snapshot saved at sequence {}", snapshot.sequence);
                                }
                                Err(e) => {
                                    error!("Failed to save snapshot: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to create snapshot: {:?}", e);
                        }
                    }
                }
                Err(_) => {
                    warn!("No price data available for snapshot");
                }
            }
        }
    });

    // ============================================================================
    // PHASE 11: MAIN EVENT LOOP
    // ============================================================================

    info!("System ready - starting event processing loop");

    let mut shutdown_signal = signal::ctrl_c();

    loop {
        tokio::select! {
            // Handle shutdown signal
            _ = &mut shutdown_signal => {
                info!("Shutdown signal received");
                break;
            }
            
            // Check kill switch and task health
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                if kill_switch.is_active() {
                    error!("Kill switch activated - shutting down");
                    break;
                }
                
                // Check task health every 100ms
                if let Err(e) = task_supervisor.check_health().await {
                    error!("Task health check failed: {:?}", e);
                    kill_switch.activate(format!("Background task failure: {:?}", e));
                    break;
                }
            }
            
            // Process events
            event_result = event_consumer.fetch_next_event() => {
                match event_result {
                    Ok(event) => {
                        // Process event
                        if let Err(e) = event_processor.process_event(event) {
                            error!("Event processing failed: {:?}", e);
                            
                            // Check if error is fatal
                            if is_fatal_error(&e) {
                                error!("Fatal error detected - activating kill switch");
                                kill_switch.activate(format!("Fatal error: {:?}", e));
                                break;
                            }
                        } else {
                            // Send sequence update to snapshot task
                            let _ = snapshot_seq_tx.try_send(event_processor.last_sequence);
                        }
                    }
                    Err(e) => {
                        error!("Event consumption failed: {:?}", e);
                        // Retry with backoff
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }

    // ============================================================================
    // PHASE 12: GRACEFUL SHUTDOWN
    // ============================================================================

    info!("Starting graceful shutdown");

    // Shutdown all background tasks
    info!("Shutting down background tasks");
    task_supervisor.shutdown_all().await;

    // Create final snapshot
    info!("Creating final snapshot");
    let balance_mgr = balance_manager.read().await;
    let position_mgr = position_manager.read().await;

    // Subscribe to get latest price
    let mut final_price_rx = price_tx.subscribe();
    if let Ok(price_snapshot) = final_price_rx.try_recv() {
        let positions_vec: Vec<_> = position_mgr.positions.values().cloned().collect();

        if let Ok(snapshot) = snapshot_manager.create_snapshot(
            event_processor.last_sequence,
            market_id,
            &*balance_mgr,
            &positions_vec,
            price_snapshot.mark_price,
            price_snapshot.index_price,
        ) {
            let _ = snapshot_manager.save_snapshot(&snapshot).await;
            info!("Final snapshot saved");
        }
    }

    info!("Shutdown complete");
    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn validate_config(config: &AppConfig) -> Result<()> {
    // Validate market config
    if config.market.tick_size.to_i64() <= 0 {
        return Err(Error::ConfigError("Invalid tick_size".to_string()));
    }

    if config.market.lot_size.to_i64() <= 0 {
        return Err(Error::ConfigError("Invalid lot_size".to_string()));
    }

    // Validate risk config
    if config.risk.max_leverage <= 0.0 || config.risk.max_leverage > 125.0 {
        return Err(Error::ConfigError("Invalid max_leverage".to_string()));
    }

    if config.risk.maintenance_margin_rate <= 0.0 || config.risk.maintenance_margin_rate >= 1.0 {
        return Err(Error::ConfigError("Invalid maintenance_margin_rate".to_string()));
    }

    // Validate Kafka config
    if config.kafka.brokers.is_empty() {
        return Err(Error::ConfigError("Kafka brokers not configured".to_string()));
    }

    if config.kafka.topic.is_empty() {
        return Err(Error::ConfigError("Kafka topic not configured".to_string()));
    }

    info!("Configuration validation passed");
    Ok(())
}

fn is_fatal_error(error: &Error) -> bool {
    matches!(error,
        Error::InvariantViolation(_) |
        Error::FundingNotZeroSum { .. } |
        Error::InsuranceFundDepleted { .. } |
        Error::InvalidChecksum
    )
}

async fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}