use crate::error::{Error, Result};
use crate::events::base::BaseEvent;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;

pub struct EventConsumer {
    consumer: StreamConsumer,
    topic: String,
}

impl EventConsumer {
    pub fn new(brokers: &str, topic: &str, group_id: &str) -> Result<Self> {
        let consumer: StreamConsumer = ClientConfig::new()
            .set("bootstrap.servers", brokers)
            .set("group.id", group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .map_err(|e| Error::KafkaError(e.to_string()))?;

        consumer.subscribe(&[topic])
            .map_err(|e| Error::KafkaError(e.to_string()))?;

        Ok(EventConsumer {
            consumer,
            topic: topic.to_string(),
        })
    }

    pub async fn fetch_event(&self, sequence: u64) -> Result<BaseEvent> {
        // In a real implementation, this would:
        // 1. Seek to the specific offset/sequence
        // 2. Read the message
        // 3. Deserialize to BaseEvent

        // For now, we'll poll for the next message
        match self.consumer.recv().await {
            Ok(message) => {
                let payload = message.payload()
                    .ok_or(Error::EmptyPayload)?;

                let event: BaseEvent = bincode::deserialize(payload)
                    .map_err(|e| Error::DeserializationError(e.to_string()))?;

                // Verify sequence matches
                if event.sequence != sequence {
                    return Err(Error::SequenceMismatch {
                        expected: sequence,
                        actual: event.sequence,
                    });
                }

                Ok(event)
            }
            Err(e) => Err(Error::KafkaError(e.to_string())),
        }
    }

    pub async fn fetch_next_event(&self) -> Result<BaseEvent> {
        match self.consumer.recv().await {
            Ok(message) => {
                let payload = message.payload()
                    .ok_or(Error::EmptyPayload)?;

                let event: BaseEvent = bincode::deserialize(payload)
                    .map_err(|e| Error::DeserializationError(e.to_string()))?;

                Ok(event)
            }
            Err(e) => Err(Error::KafkaError(e.to_string())),
        }
    }

    pub async fn fetch_events_range(&self, start: u64, end: u64) -> Result<Vec<BaseEvent>> {
        let mut events = Vec::new();

        for seq in start..=end {
            match self.fetch_event(seq).await {
                Ok(event) => events.push(event),
                Err(Error::NoMoreEvents) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(events)
    }
}