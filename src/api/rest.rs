use axum::{
    Router,
    routing::{get, post, delete},
    extract::{Path, State, Json},
    http::StatusCode,
};
use crate::events::order::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::interfaces::balance_provider::BalanceProvider;
use crate::types::ids::{MarketId, OrderId, UserId};
use crate::types::price::Price;
use crate::types::quantity::Quantity;

pub struct ApiState {
    // Shared state with engine components
    pub balance_manager: Arc<RwLock<crate::settlement::balance_manager::BalanceManager>>,
    pub position_manager: Arc<RwLock<crate::settlement::position_manager::PositionManager>>,
}

pub fn create_router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/orders", post(submit_order))
        .route("/orders/:id", delete(cancel_order))
        .route("/orders", get(list_orders))
        .route("/positions", get(get_positions))
        .route("/balances", get(get_balances))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(serde::Deserialize)]
struct OrderRequest {
    user_id: String,
    market_id: String,
    side: Side,
    order_type: OrderType,
    price: Option<i64>,
    quantity: i64,
    time_in_force: TimeInForce,
    reduce_only: bool,
    post_only: bool,
}

async fn submit_order(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<OrderRequest>,
) -> Result<Json<OrderAccepted>, StatusCode> {
    let order_id = OrderId::new();

    // Validate request
    if req.quantity <= 0 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if req.order_type == OrderType::Limit && req.price.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check user balance
    let balance_manager = state.balance_manager.read().await;
    let account = balance_manager.get_account(UserId::from_string(&req.user_id))
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Basic margin check (simplified)
    let required_margin = req.quantity / 20; // Assuming 20x leverage
    if account.available_balance().to_i64() < required_margin {
        return Err(StatusCode::PAYMENT_REQUIRED);
    }

    drop(balance_manager);

    // Create OrderSubmit event
    let order_submit = OrderSubmit {
        base: crate::events::base::BaseEvent::new(
            crate::events::base::EventType::OrderSubmit,
            MarketId::from_string(&req.market_id),
        ),
        order_id,
        user_id: UserId::from_string(&req.user_id),
        side: req.side,
        order_type: req.order_type,
        price: req.price.map(Price::from_i64),
        quantity: Quantity::from_i64(req.quantity),
        time_in_force: req.time_in_force,
        reduce_only: req.reduce_only,
        post_only: req.post_only,
        slippage_limit: None,
    };

    // Publish to event log (would integrate with EventProducer)
    tracing::info!("Order submitted: {:?}", order_id);

    Ok(Json(OrderAccepted {
        base: crate::events::base::BaseEvent::new(
            crate::events::base::EventType::OrderAccepted,
            MarketId::from_string(&req.market_id),
        ),
        order_id,
        user_id: UserId::from_string(&req.user_id),
    }))
}

async fn cancel_order(
    State(state): State<Arc<ApiState>>,
    Path(order_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    // Parse order_id
    let order_id = OrderId::from_string(&order_id);

    // Create OrderCancel event
    let cancel_event = OrderCancel {
        base: crate::events::base::BaseEvent::new(
            crate::events::base::EventType::OrderCancel,
            MarketId::new(),
        ),
        order_id,
        user_id: UserId::new(), // Would get from auth context
    };

    // Publish to event log
    tracing::info!("Order cancelled: {:?}", order_id);

    Ok(StatusCode::OK)
}

#[derive(serde::Serialize)]
struct OrderResponse {
    order_id: String,
    user_id: String,
    side: String,
    price: Option<i64>,
    quantity: i64,
    filled: i64,
    status: String,
}

async fn list_orders(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<OrderResponse>>, StatusCode> {
    // In production, would query order book or database
    // For now, return empty list
    let orders = vec![];
    Ok(Json(orders))
}

#[derive(serde::Serialize)]
struct PositionResponse {
    user_id: String,
    market_id: String,
    size: i64,
    entry_price: i64,
    unrealized_pnl: i64,
    margin_ratio: f64,
}

async fn get_positions(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<PositionResponse>>, StatusCode> {
    let position_manager = state.position_manager.read().await;

    // Get all positions (in production, filter by user from auth)
    let positions: Vec<PositionResponse> = position_manager.positions.values()
        .map(|p| PositionResponse {
            user_id: format!("{:?}", p.user_id),
            market_id: format!("{:?}", p.market_id),
            size: p.size,
            entry_price: p.entry_price.to_i64(),
            unrealized_pnl: 0, // Would calculate from current mark price
            margin_ratio: 0.0, // Would calculate from balance and position
        })
        .collect();

    Ok(Json(positions))
}

#[derive(serde::Serialize)]
struct BalanceResponse {
    user_id: String,
    balance: i64,
    reserved_margin: i64,
    available_balance: i64,
    unrealized_pnl: i64,
}

async fn get_balances(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<Vec<BalanceResponse>>, StatusCode> {
    let balance_manager = state.balance_manager.read().await;

    // Get all balances (in production, filter by user from auth)
    let balances: Vec<BalanceResponse> = balance_manager.accounts.values()
        .map(|a| BalanceResponse {
            user_id: format!("{:?}", a.user_id),
            balance: a.balance.to_i64(),
            reserved_margin: a.reserved_margin.to_i64(),
            available_balance: a.available_balance().to_i64(),
            unrealized_pnl: 0, // Would calculate from positions
        })
        .collect();

    Ok(Json(balances))
}