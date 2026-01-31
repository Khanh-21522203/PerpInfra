use std::sync::Arc;
use crate::core::event_processor::EventProcessor;
use crate::event_log::snapshot::Snapshot;
use crate::error::{Error, Result};
use crate::event_log::consumer::EventConsumer;
use crate::event_log::snapshot_manager::SnapshotManager;
use crate::types::ids::MarketId;
use crate::types::timestamp::Timestamp;

pub struct Replayer {
    event_consumer: EventConsumer,
    event_processor: EventProcessor,
    snapshot_manager: Arc<SnapshotManager>,
    market_id: MarketId,
}

impl Replayer {
    pub fn new(
        event_consumer: EventConsumer,
        event_processor: EventProcessor,
        snapshot_manager: Arc<SnapshotManager>,
        market_id: MarketId,
    ) -> Self {
        Replayer {
            event_consumer,
            event_processor,
            snapshot_manager,
            market_id,
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
        self.event_processor.restore_from_snapshot(&snapshot).await?;

        // Replay events from snapshot sequence to target
        let start_sequence = snapshot.sequence + 1;
        let end_sequence = target_sequence.unwrap_or(u64::MAX);

        let mut replayed = 0;
        for seq in start_sequence..=end_sequence {
            match self.event_consumer.fetch_event(seq).await {
                Ok(event) => {
                    self.event_processor.process_event(event).await?;
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
                    self.event_processor.process_event(event).await?;
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

    pub async fn replay_to_timestamp(
        &mut self,
        snapshot: Snapshot,
        target_timestamp: Timestamp,
    ) -> Result<()> {
        self.event_processor.restore_from_snapshot(&snapshot).await?;

        let start_sequence = snapshot.sequence + 1;
        let mut current_sequence = start_sequence;

        loop {
            match self.event_consumer.fetch_event(current_sequence).await {
                Ok(event) => {
                    if event.timestamp > target_timestamp {
                        break;
                    }
                    self.event_processor.process_event(event).await?;
                    current_sequence += 1;
                }
                Err(Error::NoMoreEvents) => break,
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }
}