use crate::core::event_processor::EventProcessor;
use crate::event_log::snapshot::Snapshot;
use crate::error::{Error, Result};
use crate::event_log::consumer::EventConsumer;

pub struct Replayer {
    event_consumer: EventConsumer,
    event_processor: EventProcessor,
}

impl Replayer {
    pub fn new(
        event_consumer: EventConsumer,
        event_processor: EventProcessor,
    ) -> Self {
        Replayer {
            event_consumer,
            event_processor,
        }
    }

    pub async fn replay_from_snapshot(
        &mut self,
        snapshot: Snapshot,
        target_sequence: Option<u64>,
    ) -> Result<()> {
        // Verify snapshot integrity
        if !snapshot.verify_checksum() {
            return Err(Error::InvalidChecksum);
        }

        tracing::info!(
            "Starting replay from snapshot at sequence {}",
            snapshot.sequence
        );

        // Restore state from snapshot
        self.event_processor.restore_from_snapshot(&snapshot)?;

        // Replay events from snapshot sequence to target
        let start_sequence = snapshot.sequence + 1;
        let end_sequence = target_sequence.unwrap_or(u64::MAX);

        let mut replayed = 0;
        for seq in start_sequence..=end_sequence {
            match self.event_consumer.fetch_event(seq).await {
                Ok(event) => {
                    self.event_processor.process_event(event)?;
                    replayed += 1;

                    if replayed % 1000 == 0 {
                        tracing::info!("Replayed {} events", replayed);
                    }
                }
                Err(Error::NoMoreEvents) => break,
                Err(e) => return Err(e),
            }
        }

        tracing::info!("Replay complete: {} events replayed", replayed);
        Ok(())
    }

    pub async fn replay_from_beginning(
        &mut self,
        target_sequence: Option<u64>,
    ) -> Result<()> {
        tracing::info!("Starting replay from beginning");

        let end_sequence = target_sequence.unwrap_or(u64::MAX);
        let mut replayed = 0;

        for seq in 0..=end_sequence {
            match self.event_consumer.fetch_event(seq).await {
                Ok(event) => {
                    self.event_processor.process_event(event)?;
                    replayed += 1;

                    if replayed % 1000 == 0 {
                        tracing::info!("Replayed {} events", replayed);
                    }
                }
                Err(Error::NoMoreEvents) => break,
                Err(e) => return Err(e),
            }
        }

        tracing::info!("Replay complete: {} events replayed", replayed);
        Ok(())
    }
}