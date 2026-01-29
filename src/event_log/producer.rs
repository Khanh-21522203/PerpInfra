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
}

impl KafkaEventProducer {
    pub fn new(brokers: &str, topic: &str) -> Result<Self> {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("message.timeout.ms", "5000")
            .set("compression.type", "lz4")
            .create()
            .map_err(|e| Error::KafkaError(e.to_string()))?;

        Ok(KafkaEventProducer {
            producer,
            topic: topic.to_string(),
            sequence_counter: std::sync::atomic::AtomicU64::new(0),
        })
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

        // Create record
        let record = FutureRecord::to(&self.topic)
            .payload(&payload)
            .key(&key);

        // Send to Kafka
        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| Error::KafkaError(e.to_string()))?;

        Ok(sequence)
    }
}