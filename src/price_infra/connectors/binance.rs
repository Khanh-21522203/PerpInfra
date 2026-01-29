use async_trait::async_trait;
use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
use tokio_tungstenite::MaybeTlsStream;
use tokio::net::TcpStream;
use futures_util::StreamExt;
use serde::Deserialize;
use crate::price_infra::connectors::PriceConnector;
use crate::price_infra::RawPriceUpdate;
use crate::error::{Error, Result};
use crate::utils::helper::current_timestamp_ms;

pub struct BinanceConnector {
    source_id: String,
    symbol: String,
    ws_url: String,
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl BinanceConnector {
    pub fn new(symbol: &str) -> Self {
        BinanceConnector {
            source_id: "binance".to_string(),
            symbol: symbol.to_string(),
            ws_url: format!("wss://stream.binance.com:9443/ws/{}@trade", symbol.to_lowercase()),
            stream: None,
        }
    }
}

#[async_trait]
impl PriceConnector for BinanceConnector {
    async fn connect(&mut self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.ws_url)
            .await
            .map_err(|e| Error::KafkaError(format!("WebSocket connection failed: {}", e)))?;
        self.stream = Some(ws_stream);
        tracing::info!("Connected to Binance: {}", self.symbol);
        Ok(())
    }

    async fn next_price(&mut self) -> Result<RawPriceUpdate> {
        let stream = self.stream.as_mut().ok_or(Error::NotConnected)?;

        loop {
            if let Some(msg) = stream.next().await {
                let msg = msg.map_err(|e| Error::KafkaError(e.to_string()))?;

                if let Message::Text(text) = msg {
                    let data: BinanceTradeData = serde_json::from_str(&text)
                        .map_err(|e| Error::DeserializationError(e.to_string()))?;

                    return Ok(RawPriceUpdate {
                        source_id: self.source_id.clone(),
                        symbol: self.symbol.clone(),
                        price: data.p.parse()
                            .map_err(|_| Error::InvalidPrice)?,
                        volume: None,
                        timestamp: data.T,
                        received_at: current_timestamp_ms(),
                    });
                }
            } else {
                return Err(Error::ConnectionClosed);
            }
        }
    }

    fn is_healthy(&self) -> bool {
        self.stream.is_some()
    }

    fn source_id(&self) -> &str {
        &self.source_id
    }
}

#[derive(Deserialize)]
struct BinanceTradeData {
    p: String,  // Price
    T: u64,     // Trade time
}