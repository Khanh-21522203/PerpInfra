use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    response::Response,
    extract::State,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct WsState {
    pub event_tx: broadcast::Sender<WsEvent>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum WsEvent {
    OrderUpdate { order_id: String, status: String },
    TradeUpdate { trade_id: String, price: i64, quantity: i64 },
    PositionUpdate { user_id: String, position: i64 },
    PriceUpdate { symbol: String, price: f64 },
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsState>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut event_rx = state.event_tx.subscribe();

    // Spawn task to send events to client
    let mut send_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = serde_json::to_string(&event).unwrap();
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Handle subscription requests
                    tracing::debug!("Received: {}", text);
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
}