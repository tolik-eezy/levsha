//! Build orchestration for self-improvement (Track E).
//!
//! Runs cargo build and test commands for the Levsha OS components.

use std::path::PathBuf;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Target component to build or test.
#[derive(Debug, Clone)]
pub enum BuildTarget {
    /// Build the chat-shell crate.
    ChatShell,
    /// Build the engine crate.
    Engine,
    /// Build the entire workspace.
    Full,
}

impl BuildTarget {
    /// Parse a build target from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "chat-shell" | "chat_shell" | "shell" => BuildTarget::ChatShell,
            "engine" => BuildTarget::Engine,
            _ => BuildTarget::Full,
        }
    }

    /// Get the cargo package name for this target.
    fn package_arg(&self) -> Option<&str> {
        match self {
            BuildTarget::ChatShell => Some("levsha-chat"),
            BuildTarget::Engine => Some("levsha-engine"),
            BuildTarget::Full => None,
        }
    }
}

/// Result of a build or test operation.
#[derive(Debug, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub artifact_path: Option<PathBuf>,
    pub output: String,
}

/// Parse cargo stderr output into (warnings, errors) vectors.
pub fn parse_cargo_output(stderr: &str) -> (Vec<String>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    for line in stderr.lines() {
        if line.contains("warning:") || line.contains("warning[") {
            warnings.push(line.trim().to_string());
        }
        if line.contains("error:") || line.contains("error[") {
            errors.push(line.trim().to_string());
        }
    }

    (warnings, errors)
}

/// Orchestrates cargo build and test operations.
pub struct BuildOrchestrator {
    source_root: PathBuf,
}

impl BuildOrchestrator {
    /// Create a new build orchestrator.
    pub fn new(source_root: PathBuf) -> Self {
        Self { source_root }
    }

    /// Build a target in release mode.
    pub async fn build(&self, target: BuildTarget) -> BuildResult {
        let start = Instant::now();
        info!("Building {:?}", target);

        let mut cmd = Command::new("cargo");
        cmd.arg("build").arg("--release");

        if let Some(pkg) = target.package_arg() {
            cmd.arg("-p").arg(pkg);
        }

        cmd.current_dir(&self.source_root);

        let result = self.run_cargo(cmd, &target).await;
        let duration = start.elapsed();
        if result.success {
            info!(
                "Build {:?} succeeded in {:.1}s ({} warnings)",
                target,
                duration.as_secs_f64(),
                result.warnings.len()
            );
        } else {
            warn!(
                "Build {:?} failed in {:.1}s ({} errors)",
                target,
                duration.as_secs_f64(),
                result.errors.len()
            );
        }
        result
    }

    /// Check a target (fast compile check, no codegen).
    pub async fn check(&self, target: BuildTarget) -> BuildResult {
        let start = Instant::now();
        info!("Checking {:?}", target);

        let mut cmd = Command::new("cargo");
        cmd.arg("check");

        if let Some(pkg) = target.package_arg() {
            cmd.arg("-p").arg(pkg);
        }

        cmd.current_dir(&self.source_root);

        let result = self.run_cargo(cmd, &target).await;
        let duration = start.elapsed();
        info!(
            "Check {:?} {} in {:.1}s",
            target,
            if result.success { "passed" } else { "failed" },
            duration.as_secs_f64()
        );
        result
    }

    /// Build a target with streaming stderr output.
    /// Each line of cargo's stderr is sent through `progress_tx`.
    pub async fn build_streaming(
        &self,
        target: BuildTarget,
        progress_tx: mpsc::Sender<String>,
    ) -> BuildResult {
        let start = Instant::now();
        info!("Building {:?} (streaming)", target);

        let mut cmd = Command::new("cargo");
        cmd.arg("build").arg("--release");

        if let Some(pkg) = target.package_arg() {
            cmd.arg("-p").arg(pkg);
        }

        cmd.current_dir(&self.source_root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to spawn cargo build: {}", e);
                return BuildResult {
                    success: false,
                    warnings: Vec::new(),
                    errors: vec![format!("Failed to run cargo: {}", e)],
                    artifact_path: None,
                    output: String::new(),
                };
            }
        };

        // Stream stderr line by line.
        let stderr = child.stderr.take();
        let mut all_stderr = String::new();

        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                all_stderr.push_str(&line);
                all_stderr.push('\n');
                progress_tx.send(line).await.ok();
            }
        }

        let status = match child.wait().await {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to wait for cargo build: {}", e);
                return BuildResult {
                    success: false,
                    warnings: Vec::new(),
                    errors: vec![format!("Failed to wait for cargo: {}", e)],
                    artifact_path: None,
                    output: all_stderr,
                };
            }
        };

        let (warnings, errors) = parse_cargo_output(&all_stderr);

        let artifact_path = if status.success() {
            let target_dir = self.source_root.join("target/release");
            match &target {
                BuildTarget::ChatShell => Some(target_dir.join("levsha-chat")),
                BuildTarget::Engine => Some(target_dir.join("levsha-engine")),
                BuildTarget::Full => Some(target_dir.join("levsha-chat")),
            }
        } else {
            None
        };

        let duration = start.elapsed();
        if status.success() {
            info!(
                "Streaming build {:?} succeeded in {:.1}s ({} warnings, artifact: {:?})",
                target,
                duration.as_secs_f64(),
                warnings.len(),
                artifact_path
            );
        } else {
            warn!(
                "Streaming build {:?} failed in {:.1}s ({} errors)",
                target,
                duration.as_secs_f64(),
                errors.len()
            );
        }

        BuildResult {
            success: status.success(),
            warnings,
            errors,
            artifact_path,
            output: all_stderr,
        }
    }

    /// Run tests for a target.
    pub async fn test(&self, target: BuildTarget) -> BuildResult {
        let start = Instant::now();
        info!("Testing {:?}", target);

        let mut cmd = Command::new("cargo");
        cmd.arg("test");

        if let Some(pkg) = target.package_arg() {
            cmd.arg("-p").arg(pkg);
        }

        cmd.current_dir(&self.source_root);

        let result = self.run_cargo(cmd, &target).await;
        let duration = start.elapsed();
        info!(
            "Tests {:?} {} in {:.1}s",
            target,
            if result.success { "passed" } else { "failed" },
            duration.as_secs_f64()
        );
        result
    }

    /// Run tests with streaming stderr output.
    /// Each line of cargo's stderr is sent through `progress_tx`.
    pub async fn test_streaming(
        &self,
        target: BuildTarget,
        progress_tx: mpsc::Sender<String>,
    ) -> BuildResult {
        let start = Instant::now();
        info!("Testing {:?} (streaming)", target);

        let mut cmd = Command::new("cargo");
        cmd.arg("test");

        if let Some(pkg) = target.package_arg() {
            cmd.arg("-p").arg(pkg);
        }

        cmd.current_dir(&self.source_root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to spawn cargo test: {}", e);
                return BuildResult {
                    success: false,
                    warnings: Vec::new(),
                    errors: vec![format!("Failed to run cargo: {}", e)],
                    artifact_path: None,
                    output: String::new(),
                };
            }
        };

        // Stream stderr line by line.
        let stderr = child.stderr.take();
        let mut all_stderr = String::new();

        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                all_stderr.push_str(&line);
                all_stderr.push('\n');
                progress_tx.send(line).await.ok();
            }
        }

        let status = match child.wait().await {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to wait for cargo test: {}", e);
                return BuildResult {
                    success: false,
                    warnings: Vec::new(),
                    errors: vec![format!("Failed to wait for cargo: {}", e)],
                    artifact_path: None,
                    output: all_stderr,
                };
            }
        };

        let (warnings, errors) = parse_cargo_output(&all_stderr);

        let duration = start.elapsed();
        info!(
            "Streaming tests {:?} {} in {:.1}s",
            target,
            if status.success() { "passed" } else { "failed" },
            duration.as_secs_f64()
        );

        BuildResult {
            success: status.success(),
            warnings,
            errors,
            artifact_path: None,
            output: all_stderr,
        }
    }

    /// Run a cargo command and parse the output.
    async fn run_cargo(&self, mut cmd: Command, target: &BuildTarget) -> BuildResult {
        debug!("Running cargo in {:?}", self.source_root);

        let output = match cmd.output().await {
            Ok(o) => o,
            Err(e) => {
                warn!("Failed to run cargo command: {}", e);
                return BuildResult {
                    success: false,
                    warnings: Vec::new(),
                    errors: vec![format!("Failed to run cargo: {}", e)],
                    artifact_path: None,
                    output: String::new(),
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}\n{}", stdout, stderr);

        debug!(
            "Cargo exited with code {:?} (stdout: {} bytes, stderr: {} bytes)",
            output.status.code(),
            stdout.len(),
            stderr.len()
        );

        let (warnings, errors) = parse_cargo_output(&stderr);

        let artifact_path = if output.status.success() {
            // Determine artifact path based on target.
            let target_dir = self.source_root.join("target/release");
            match target {
                BuildTarget::ChatShell => Some(target_dir.join("levsha-chat")),
                BuildTarget::Engine => Some(target_dir.join("levsha-engine")),
                BuildTarget::Full => Some(target_dir.join("levsha-chat")),
            }
        } else {
            None
        };

        BuildResult {
            success: output.status.success(),
            warnings,
            errors,
            artifact_path,
            output: combined,
        }
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
