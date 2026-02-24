//! Checkpoint management for self-improvement (Track E).
//!
//! Creates, lists, restores, and prunes snapshots of built binaries
//! and git state to enable safe rollback after self-modifications.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// A checkpoint snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub git_hash: String,
    pub timestamp: i64,
    pub binaries: Vec<PathBuf>,
}

/// Manages checkpoint creation, listing, and rollback.
pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
    max_checkpoints: usize,
}

impl CheckpointManager {
    /// Create a new checkpoint manager.
    pub fn new(checkpoint_dir: PathBuf, max_checkpoints: usize) -> Self {
        Self {
            checkpoint_dir,
            max_checkpoints,
        }
    }

    /// Create a new checkpoint from the current state.
    pub fn create(
        &self,
        git_hash: &str,
        binaries: &[PathBuf],
    ) -> Result<Checkpoint, String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let id = format!("ckpt-{}-{}", &git_hash[..8.min(git_hash.len())], timestamp);
        let ckpt_dir = self.checkpoint_dir.join(&id);

        debug!(
            "Creating checkpoint {} (git: {}, {} binaries)",
            id,
            git_hash,
            binaries.len()
        );

        std::fs::create_dir_all(&ckpt_dir)
            .map_err(|e| format!("Failed to create checkpoint directory: {}", e))?;

        // Copy binaries into checkpoint.
        let mut saved_binaries = Vec::new();
        let mut total_bytes: u64 = 0;
        for bin_path in binaries {
            if bin_path.exists() {
                let dest = ckpt_dir.join(
                    bin_path
                        .file_name()
                        .unwrap_or_default(),
                );
                let bytes = std::fs::copy(bin_path, &dest)
                    .map_err(|e| format!("Failed to copy binary: {}", e))?;
                total_bytes += bytes;
                saved_binaries.push(dest);
            } else {
                debug!("Skipping missing binary: {:?}", bin_path);
            }
        }

        let checkpoint = Checkpoint {
            id: id.clone(),
            git_hash: git_hash.to_string(),
            timestamp,
            binaries: saved_binaries,
        };

        // Save metadata.
        let metadata_path = ckpt_dir.join("checkpoint.json");
        let metadata = serde_json::to_string_pretty(&checkpoint)
            .map_err(|e| format!("Failed to serialize checkpoint: {}", e))?;
        std::fs::write(&metadata_path, metadata)
            .map_err(|e| format!("Failed to write checkpoint metadata: {}", e))?;

        info!(
            "Created checkpoint: {} ({} binaries, {} bytes copied)",
            id,
            checkpoint.binaries.len(),
            total_bytes
        );
        self.prune().ok();

        Ok(checkpoint)
    }

    /// List all available checkpoints, newest first.
    pub fn list(&self) -> Vec<Checkpoint> {
        let mut checkpoints = Vec::new();

        if !self.checkpoint_dir.is_dir() {
            debug!("Checkpoint directory does not exist: {:?}", self.checkpoint_dir);
            return checkpoints;
        }

        let entries = match std::fs::read_dir(&self.checkpoint_dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read checkpoint directory: {}", e);
                return checkpoints;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let metadata_path = path.join("checkpoint.json");
            if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                if let Ok(ckpt) = serde_json::from_str::<Checkpoint>(&content) {
                    checkpoints.push(ckpt);
                }
            }
        }

        // Sort newest first.
        checkpoints.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        debug!("Listed {} checkpoints", checkpoints.len());
        checkpoints
    }

    /// Restore a checkpoint by ID. Returns the checkpoint's binaries.
    pub fn restore(&self, id: &str) -> Result<Checkpoint, String> {
        let ckpt_dir = self.checkpoint_dir.join(id);
        if !ckpt_dir.is_dir() {
            return Err(format!("Checkpoint '{}' not found", id));
        }

        let metadata_path = ckpt_dir.join("checkpoint.json");
        let content = std::fs::read_to_string(&metadata_path)
            .map_err(|e| format!("Failed to read checkpoint: {}", e))?;
        let checkpoint: Checkpoint = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse checkpoint: {}", e))?;

        info!(
            "Restored checkpoint: {} (git: {}, {} binaries)",
            id,
            checkpoint.git_hash,
            checkpoint.binaries.len()
        );
        Ok(checkpoint)
    }

    /// Prune old checkpoints beyond the max limit.
    pub fn prune(&self) -> Result<usize, String> {
        let checkpoints = self.list();
        let mut removed = 0;

        if checkpoints.len() > self.max_checkpoints {
            let to_remove = &checkpoints[self.max_checkpoints..];
            debug!(
                "Pruning {} checkpoints (have {}, max {})",
                to_remove.len(),
                checkpoints.len(),
                self.max_checkpoints
            );
            for ckpt in to_remove {
                let ckpt_dir = self.checkpoint_dir.join(&ckpt.id);
                if std::fs::remove_dir_all(&ckpt_dir).is_ok() {
                    info!("Pruned old checkpoint: {}", ckpt.id);
                    removed += 1;
                } else {
                    warn!("Failed to prune checkpoint: {}", ckpt.id);
                }
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
