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

pub struct CoinbaseConnector {
    source_id: String,
    symbol: String,
    ws_url: String,
    stream: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl CoinbaseConnector {
    pub fn new(symbol: &str) -> Self {
        CoinbaseConnector {
            source_id: "coinbase".to_string(),
            symbol: symbol.to_uppercase(),
            ws_url: "wss://ws-feed.exchange.coinbase.com".to_string(),
            stream: None,
        }
    }
}

#[async_trait]
impl PriceConnector for CoinbaseConnector {
    async fn connect(&mut self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.ws_url)
            .await
            .map_err(|e| Error::KafkaError(format!("WebSocket connection failed: {}", e)))?;

        self.stream = Some(ws_stream);
        tracing::info!("Connected to Coinbase: {}", self.symbol);
        Ok(())
    }

    async fn next_price(&mut self) -> Result<RawPriceUpdate> {
        let stream = self.stream.as_mut().ok_or(Error::NotConnected)?;

        loop {
            if let Some(msg) = stream.next().await {
                let msg = msg.map_err(|e| Error::KafkaError(e.to_string()))?;

                if let Message::Text(text) = msg {
                    let data: CoinbaseTickerData = serde_json::from_str(&text)
                        .map_err(|e| Error::DeserializationError(e.to_string()))?;

                    if data.type_field == "ticker" {
                        return Ok(RawPriceUpdate {
                            source_id: self.source_id.clone(),
                            symbol: self.symbol.clone(),
                            price: data.price.parse()
                                .map_err(|_| Error::InvalidPrice)?,
                            volume: data.volume_24h.and_then(|v| v.parse().ok()),
                            timestamp: data.time.parse().unwrap_or(0),
                            received_at: current_timestamp_ms(),
                        });
                    }
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
struct CoinbaseTickerData {
    #[serde(rename = "type")]
    type_field: String,
    price: String,
    volume_24h: Option<String>,
    time: String,
}