use crate::event_log::snapshot::Snapshot;
use crate::replay::replayer::Replayer;
use crate::error::Result;
use crate::types::balance::Balance;
use crate::types::timestamp::Timestamp;

pub struct ComplianceAuditor {
    replayer: Replayer,
    snapshot_dir: String,
}

impl ComplianceAuditor {
    pub fn new(replayer: Replayer, snapshot_dir: String) -> Self {
        ComplianceAuditor { replayer, snapshot_dir }
    }

    /// Audit a specific time range
    pub async fn audit_time_range(
        &mut self,
        start_time: Timestamp,
        end_time: Timestamp,
    ) -> Result<AuditReport> {
        tracing::info!(
            "Starting compliance audit from {:?} to {:?}",
            start_time,
            end_time
        );

        // Find snapshot before start_time
        let snapshot = self.find_snapshot_before(start_time)?;

        // Replay to end_time
        self.replayer.replay_from_snapshot(snapshot, None).await?;

        // Generate report
        Ok(AuditReport {
            start_time,
            end_time,
            total_trades: 0,  // Would be populated from replay
            total_volume: Balance::zero(),
            violations: Vec::new(),
        })
    }

    fn find_snapshot_before(&self, timestamp: Timestamp) -> Result<Snapshot> {
        use std::fs;

        // List all snapshot files in directory
        let entries = fs::read_dir(&self.snapshot_dir)
            .map_err(|e| crate::error::Error::IoError(e))?;

        let mut best_snapshot: Option<Snapshot> = None;
        let mut best_timestamp = Timestamp::from_millis(0);

        for entry in entries {
            let entry = entry.map_err(|e| crate::error::Error::IoError(e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("snapshot") {
                // Read and deserialize snapshot
                let data = fs::read(&path)
                    .map_err(|e| crate::error::Error::IoError(e))?;

                let snapshot: Snapshot = bincode::deserialize(&data)
                    .map_err(|e| crate::error::Error::DeserializationError(e.to_string()))?;

                // Check if this snapshot is before target and better than current best
                if snapshot.timestamp < timestamp && snapshot.timestamp > best_timestamp {
                    best_timestamp = snapshot.timestamp;
                    best_snapshot = Some(snapshot);
                }
            }
        }

        best_snapshot.ok_or(crate::error::Error::NoSnapshotFound)
    }

    /// Save snapshot to disk
    pub fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let filename = format!(
            "{}/snapshot_{}_{}.snapshot",
            self.snapshot_dir,
            snapshot.sequence,
            snapshot.timestamp.physical
        );

        let data = bincode::serialize(snapshot)
            .map_err(|e| crate::error::Error::SerializationError(e.to_string()))?;

        let mut file = File::create(&filename)
            .map_err(|e| crate::error::Error::IoError(e))?;

        file.write_all(&data)
            .map_err(|e| crate::error::Error::IoError(e))?;

        tracing::info!("Snapshot saved to {}", filename);
        Ok(())
    }
}

#[derive(Debug)]
pub struct AuditReport {
    pub start_time: Timestamp,
    pub end_time: Timestamp,
    pub total_trades: u64,
    pub total_volume: Balance,
    pub violations: Vec<String>,
}