use crate::events::base::BaseEvent;
use crate::error::{Error, Result};
use crate::interfaces::event_producer::EventProducer;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::config::ClientConfig;
use async_trait::async_trait;
use std::time::Duration;

pub struct KafkaEventProducer {
    producer: FutureProducer,
    topic: String,
    sequence_counter: std::sync::atomic::AtomicU64,
    max_retries: u32,
}

impl KafkaEventProducer {
    pub fn new(brokers: &str, topic: &str) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "lz4")
            .set("retries", "10")  // Kafka client-level retries
            .set("retry.backoff.ms", "100")
            .create()
            .map_err(|e| Error::KafkaError(e.to_string()))?;

        Ok(KafkaEventProducer {
            producer,
            topic: topic.to_string(),
            sequence_counter: std::sync::atomic::AtomicU64::new(0),
            max_retries: 5,
        })
    }

    /// Retry with exponential backoff
    /// Per docs/architecture/event-model.md Section 11.1
    async fn produce_with_retry(&self, key: &str, payload: &[u8]) -> Result<()> {
        let mut attempt = 0;

        while attempt < self.max_retries {
            attempt += 1;
            // Create record inside loop since FutureRecord is not Clone
            let record = FutureRecord::to(&self.topic)
                .payload(payload)
                .key(key);

            match self.producer.send(record, Duration::from_secs(5)).await {
                Ok(_) => return Ok(()),
                Err((e, _)) => {
                    if attempt >= self.max_retries {
                        return Err(Error::KafkaError(e.to_string()));
                    }

                    // Exponential backoff: 100ms, 200ms, 400ms, 800ms, 1600ms
                    let backoff_ms = 100 * 2u64.pow(attempt - 1);
                    tracing::warn!(
                        "Kafka produce failed (attempt {}/{}), retrying in {}ms: {:?}",
                        attempt,
                        self.max_retries,
                        backoff_ms,
                        e
                    );

                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }

        Err(Error::KafkaError("Max retries exceeded".to_string()))
    }
}

#[async_trait]
impl EventProducer for KafkaEventProducer {
    async fn produce(&self, mut event: BaseEvent) -> Result<u64> {
        // Assign sequence number
        let sequence = self.sequence_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        event.sequence = sequence;

        // Serialize event
        let payload = bincode::serialize(&event)
            .map_err(|e| Error::SerializationError(e.to_string()))?;

        let key = sequence.to_string();

        // Send to Kafka
        self.produce_with_retry(&key, &payload).await?;

        Ok(sequence)
    }
}