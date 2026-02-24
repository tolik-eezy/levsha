//! Self-improvement system (Track E).
//!
//! Provides the engine with the ability to modify, build, test, and deploy
//! its own source code. Delegates source editing to an external coding agent
//! (Claude Code or OpenCode) instead of managing file operations directly.

pub mod builder;
pub mod checkpoint;
pub mod coding_agent;
pub mod deployer;
pub mod git;

pub use coding_agent::*;

use std::path::PathBuf;

/// Top-level manager for self-improvement operations.
pub struct SelfImproveManager {
    pub git: git::GitManager,
    pub builder: builder::BuildOrchestrator,
    pub checkpoint: checkpoint::CheckpointManager,
    pub deployer: deployer::DeployManager,
    pub agent_config: CodingAgentConfig,
    /// The currently active coding agent session, if any.
    pub active_session: Option<CodingAgentSession>,
}

impl SelfImproveManager {
    /// Create a new self-improvement manager.
    pub fn new(
        source_root: &str,
        checkpoint_dir: &str,
        max_checkpoints: usize,
        agent_config: CodingAgentConfig,
    ) -> Self {
        let source_root = PathBuf::from(source_root);
        let checkpoint_dir = PathBuf::from(checkpoint_dir);

        Self {
            git: git::GitManager::new(source_root.clone()),
            builder: builder::BuildOrchestrator::new(source_root),
            checkpoint: checkpoint::CheckpointManager::new(checkpoint_dir, max_checkpoints),
            deployer: deployer::DeployManager::new(),
            agent_config,
            active_session: None,
        }
    }
}

#[cfg(test)]
#[path = "integration_tests.rs"]
mod integration_tests;
