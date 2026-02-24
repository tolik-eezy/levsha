//! Git operations for self-improvement (Track E).
//!
//! Wraps git CLI commands for source code version control within the
//! self-improvement workflow.

use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tracing::{debug, info};

/// Manages git operations on the source tree.
pub struct GitManager {
    source_root: PathBuf,
}

impl GitManager {
    /// Create a new git manager for the given source root.
    pub fn new(source_root: PathBuf) -> Self {
        Self { source_root }
    }

    /// Get the diff of unstaged working directory changes.
    pub async fn diff_working(&self) -> Result<String, String> {
        let diff = self.run_git(&["diff"]).await?;
        info!(diff_size = diff.len(), "diff_working");
        Ok(diff)
    }

    /// Get the diff of staged changes.
    pub async fn diff_staged(&self) -> Result<String, String> {
        let diff = self.run_git(&["diff", "--staged"]).await?;
        info!(diff_size = diff.len(), "diff_staged");
        Ok(diff)
    }

    /// Stage all changes and commit with the given message.
    pub async fn commit_all(&self, message: &str) -> Result<String, String> {
        self.run_git(&["add", "-A"]).await?;
        let result = self.run_git(&["commit", "-m", message]).await?;
        info!(message, "commit_all succeeded");
        Ok(result)
    }

    /// Get recent commit log.
    pub async fn log(&self, count: usize) -> Result<String, String> {
        info!(count, "fetching git log");
        self.run_git(&[
            "log",
            "--oneline",
            &format!("-{}", count),
        ])
        .await
    }

    /// Create a tag.
    pub async fn tag(&self, name: &str) -> Result<String, String> {
        let result = self.run_git(&["tag", name]).await?;
        info!(tag_name = name, "created tag");
        Ok(result)
    }

    /// Get the current commit hash.
    pub async fn current_hash(&self) -> Result<String, String> {
        let output = self.run_git(&["rev-parse", "HEAD"]).await?;
        let hash = output.trim().to_string();
        info!(hash = %hash, "current_hash");
        Ok(hash)
    }

    /// List files changed in the working directory.
    pub async fn diff_file_list(&self) -> Result<Vec<String>, String> {
        let output = self.run_git(&["diff", "--name-only"]).await?;
        Ok(output
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect())
    }

    /// Check if there are any uncommitted changes (staged or unstaged).
    pub async fn has_changes(&self) -> Result<bool, String> {
        let output = self.run_git(&["status", "--porcelain"]).await?;
        Ok(!output.trim().is_empty())
    }

    /// Show a file's content at HEAD. Returns empty string if the file
    /// doesn't exist at HEAD (e.g. newly added file).
    pub async fn show_file(&self, path: &str) -> Result<String, String> {
        let spec = format!("HEAD:{}", path);
        match self.run_git(&["show", &spec]).await {
            Ok(content) => Ok(content),
            Err(_) => Ok(String::new()),
        }
    }

    /// Run a git command in the source root directory.
    async fn run_git(&self, args: &[&str]) -> Result<String, String> {
        debug!(?args, root = %self.source_root.display(), "running git command");
        let start = Instant::now();

        let output = tokio::time::timeout(
            Duration::from_secs(30),
            Command::new("git")
                .args(args)
                .current_dir(&self.source_root)
                .output(),
        )
        .await
        .map_err(|_| "git command timed out after 30 seconds".to_string())?
        .map_err(|e| format!("Failed to run git: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let duration_ms = start.elapsed().as_millis() as u64;

        debug!(
            ?args,
            exit_code,
            stdout_len = stdout.len(),
            stderr_len = stderr.len(),
            duration_ms,
            "git command completed"
        );

        if !output.status.success() {
            if !stderr.is_empty() {
                return Err(format!("git error: {}", stderr.trim()));
            }
            return Err(format!(
                "git exited with code {}",
                exit_code
            ));
        }

        Ok(stdout)
    }
}

#[cfg(test)]
#[path = "git_tests.rs"]
mod tests;
