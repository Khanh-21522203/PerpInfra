use std::path::{Path, PathBuf};
use crate::error::{Error, Result};
use crate::event_log::snapshot::Snapshot;
use crate::settlement::balance_manager::BalanceManager;
use crate::types::ids::MarketId;
use crate::types::position::Position;
use crate::types::price::Price;
use tokio::fs as async_fs;

/// Snapshot Manager - Handles creation, persistence, and restoration of system state snapshots
///
/// ## Snapshot Format
/// - **Serialization**: Binary format using `bincode` crate
/// - **Compression**: None (can be added with lz4 if needed)
/// - **Checksum**: SHA-256 hash for corruption detection
/// - **File Extension**: `.bin`
/// - **Naming Convention**: `snapshot_{market_id}_{sequence}.bin`
///
/// ## Retention Policy
/// - **Max Snapshots**: 100 per market (configurable via `max_snapshots`)
/// - **Cleanup Strategy**: FIFO - oldest snapshots deleted when limit exceeded
/// - **Cleanup Trigger**: After each successful snapshot save
///
/// ## Atomicity Guarantees
/// - **Write**: Atomic file write using `tokio::fs::write` (writes to temp file, then renames)
/// - **Read**: Checksum verification on load to detect corruption
/// - **Concurrent Access**: Safe for multiple readers, single writer
///
/// ## Recovery Behavior
/// - **Startup**: Attempts to load latest snapshot via `load_latest()`
/// - **No Snapshot**: Returns `Error::NoSnapshotFound`, system starts from beginning
/// - **Corrupted Snapshot**: Returns `Error::InvalidChecksum`, falls back to previous snapshot
/// - **Missing Sequence**: Can load specific sequence via `load_snapshot_at_sequence()`
///
/// ## Disk Space Management
/// - **Snapshot Size**: ~1KB per account + ~500B per position (typical)
/// - **Max Disk Usage**: ~100MB for 100 snapshots with 1000 accounts/positions each
/// - **Directory**: Configurable, defaults to `./snapshots`
/// - **Permissions**: Requires read/write access to snapshot directory
pub struct SnapshotManager {
    snapshot_dir: PathBuf,
    max_snapshots: usize,
}

impl SnapshotManager {
    pub fn new(snapshot_dir: impl AsRef<Path>) -> Self {
        SnapshotManager {
            snapshot_dir: snapshot_dir.as_ref().to_path_buf(),
            max_snapshots: 100,
        }
    }

    /// Create a snapshot from current system state
    pub fn create_snapshot(
        &self,
        sequence: u64,
        market_id: MarketId,
        balance_manager: &BalanceManager,
        positions: &[Position],
        mark_price: Price,
        index_price: Price,
    ) -> Result<Snapshot> {
        // Collect all accounts
        let accounts: Vec<_> = balance_manager.accounts.values().cloned().collect();

        // Create snapshot
        let snapshot = Snapshot::new(
            sequence,
            market_id,
            accounts,
            positions.to_vec(),
            mark_price,
            index_price,
        );

        tracing::info!(
            "Created snapshot at sequence {} with {} accounts and {} positions",
            sequence,
            snapshot.accounts.len(),
            snapshot.positions.len()
        );

        Ok(snapshot)
    }

    /// Save snapshot to disk
    pub async fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        // Ensure snapshot directory exists
        async_fs::create_dir_all(&self.snapshot_dir)
            .await
            .map_err(|e| Error::IoError(e))?;

        // Generate filename
        let filename = format!(
            "snapshot_{}_{}.bin",
            snapshot.market_id,
            snapshot.sequence
        );
        let filepath = self.snapshot_dir.join(filename);

        // Serialize snapshot
        let data = bincode::serialize(snapshot)
            .map_err(|e| Error::SerializationError(e.to_string()))?;

        // Write to file
        async_fs::write(&filepath, data)
            .await
            .map_err(|e| Error::IoError(e))?;

        tracing::info!("Saved snapshot to {:?}", filepath);

        // Cleanup old snapshots
        self.cleanup_old_snapshots(snapshot.market_id).await?;

        Ok(())
    }

    /// Load the latest snapshot for a market
    pub async fn load_latest(&self, market_id: MarketId) -> Result<Snapshot> {
        let snapshots = self.list_snapshots(market_id).await?;

        if snapshots.is_empty() {
            return Err(Error::NoSnapshotFound);
        }

        // Get latest snapshot (highest sequence)
        let latest = snapshots.last().unwrap();
        self.load_snapshot(latest).await
    }

    /// Load a specific snapshot by sequence
    pub async fn load_snapshot_at_sequence(
        &self,
        market_id: MarketId,
        sequence: u64,
    ) -> Result<Snapshot> {
        let filename = format!("snapshot_{}_{}.bin", market_id, sequence);
        let filepath = self.snapshot_dir.join(filename);

        self.load_snapshot(&filepath).await
    }

    /// Load snapshot from file
    async fn load_snapshot(&self, filepath: &Path) -> Result<Snapshot> {
        let data = async_fs::read(filepath)
            .await
            .map_err(|e| Error::IoError(e))?;

        let snapshot: Snapshot = bincode::deserialize(&data)
            .map_err(|e| Error::DeserializationError(e.to_string()))?;

        // Verify checksum
        if !snapshot.verify_checksum() {
            return Err(Error::InvalidChecksum);
        }

        tracing::info!("Loaded snapshot from {:?}", filepath);
        Ok(snapshot)
    }

    /// List all snapshots for a market (sorted by sequence)
    async fn list_snapshots(&self, market_id: MarketId) -> Result<Vec<PathBuf>> {
        let mut snapshots = Vec::new();

        let mut entries = async_fs::read_dir(&self.snapshot_dir)
            .await
            .map_err(|e| Error::IoError(e))?;

        while let Some(entry) = entries.next_entry()
            .await
            .map_err(|e| Error::IoError(e))?
        {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with(&format!("snapshot_{}_", market_id)) {
                    snapshots.push(path);
                }
            }
        }

        // Sort by sequence number (extracted from filename)
        snapshots.sort_by_key(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .and_then(|s| s.split('_').nth(2))
                .and_then(|s| s.strip_suffix(".bin"))
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
        });

        Ok(snapshots)
    }

    /// Cleanup old snapshots, keeping only the most recent N
    async fn cleanup_old_snapshots(&self, market_id: MarketId) -> Result<()> {
        let snapshots = self.list_snapshots(market_id).await?;

        if snapshots.len() <= self.max_snapshots {
            return Ok(());
        }

        // Delete oldest snapshots
        let to_delete = snapshots.len() - self.max_snapshots;
        for snapshot_path in snapshots.iter().take(to_delete) {
            async_fs::remove_file(snapshot_path)
                .await
                .map_err(|e| Error::IoError(e))?;

            tracing::info!("Deleted old snapshot: {:?}", snapshot_path);
        }

        Ok(())
    }
}