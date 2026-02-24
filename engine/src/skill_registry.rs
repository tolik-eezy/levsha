//! Skill registry client — discovers skills from a remote git-based registry.
//!
//! The registry is a git repository containing an `index.yaml` that maps
//! skill names to their metadata and source repositories.

use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// An entry in the skill registry index.
#[derive(Debug, Clone, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub repository: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Index file structure as read from the registry repo.
#[derive(Debug, Deserialize)]
struct RegistryIndex {
    #[serde(default)]
    skills: Vec<RegistryEntry>,
}

/// Client for interacting with the skill registry.
pub struct RegistryClient {
    registry_url: String,
    cache_dir: PathBuf,
}

impl RegistryClient {
    pub fn new(registry_url: &str, cache_dir: PathBuf) -> Self {
        Self {
            registry_url: registry_url.to_string(),
            cache_dir,
        }
    }

    /// Refresh the local cache of the registry.
    /// Clones on first run, pulls on subsequent runs.
    pub async fn refresh(&self) -> Result<(), Box<dyn std::error::Error>> {
        let repo_dir = self.cache_dir.join("registry");

        if repo_dir.join(".git").exists() {
            // Pull latest changes.
            info!("Pulling registry updates from {}", self.registry_url);
            let output = tokio::time::timeout(
                Duration::from_secs(30),
                Command::new("git")
                    .args(["pull", "--ff-only"])
                    .current_dir(&repo_dir)
                    .output(),
            )
            .await
            .map_err(|_| "Registry pull timed out after 30s")??;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Registry pull failed: {}", stderr);
                return Err(format!("git pull failed: {}", stderr).into());
            }
            debug!("Registry pull succeeded");
        } else {
            // Clone the repository.
            info!("Cloning registry from {}", self.registry_url);
            std::fs::create_dir_all(&self.cache_dir)?;

            let output = tokio::time::timeout(
                Duration::from_secs(30),
                Command::new("git")
                    .args([
                        "clone",
                        "--depth",
                        "1",
                        &self.registry_url,
                        &repo_dir.to_string_lossy(),
                    ])
                    .output(),
            )
            .await
            .map_err(|_| "Registry clone timed out after 30s")??;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("Registry clone failed: {}", stderr);
                return Err(format!("git clone failed: {}", stderr).into());
            }
            debug!("Registry clone succeeded");
        }

        Ok(())
    }

    /// Search for skills matching a query string (case-insensitive substring).
    pub fn search(&self, query: &str) -> Vec<RegistryEntry> {
        let entries = match self.load_index() {
            Ok(entries) => entries,
            Err(e) => {
                warn!("Failed to load registry index: {}", e);
                return Vec::new();
            }
        };

        let query_lower = query.to_lowercase();
        entries
            .into_iter()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&query_lower)
                    || entry.description.to_lowercase().contains(&query_lower)
                    || entry
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Find a skill by exact name.
    pub fn find(&self, name: &str) -> Option<RegistryEntry> {
        let entries = self.load_index().ok()?;
        entries.into_iter().find(|e| e.name == name)
    }

    /// Load and parse the index.yaml from the cached registry.
    fn load_index(&self) -> Result<Vec<RegistryEntry>, Box<dyn std::error::Error>> {
        let index_path = self.cache_dir.join("registry").join("index.yaml");
        if !index_path.exists() {
            return Err("Registry not cached. Run refresh first.".into());
        }
        let content = std::fs::read_to_string(&index_path)?;
        let index: RegistryIndex = serde_yaml::from_str(&content)?;
        Ok(index.skills)
    }
}
