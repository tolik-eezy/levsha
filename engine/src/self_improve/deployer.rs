//! Deployment manager for self-improvement (Track E).
//!
//! Handles deploying newly built binaries, with health checks and
//! automatic rollback on failure.

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Manages deployment of self-improved binaries.
pub struct DeployManager;

impl DeployManager {
    /// Create a new deploy manager.
    pub fn new() -> Self {
        Self
    }

    /// Deploy a new binary to the target path.
    ///
    /// Steps:
    /// 1. Backup current binary
    /// 2. Copy new binary to target
    /// 3. Restart the service
    /// 4. Health check
    /// 5. Rollback on failure
    pub async fn deploy(
        &self,
        target: &str,
        artifact_path: &Path,
    ) -> Result<String, String> {
        let target_path = self.resolve_target(target)?;
        let start = std::time::Instant::now();

        info!(
            target = target,
            artifact = %artifact_path.display(),
            target_path = %target_path.display(),
            "Starting deploy"
        );

        if !artifact_path.exists() {
            warn!(artifact = %artifact_path.display(), "Artifact not found");
            return Err(format!("Artifact not found: {}", artifact_path.display()));
        }

        let artifact_size = std::fs::metadata(artifact_path)
            .map(|m| m.len())
            .unwrap_or(0);
        debug!(artifact_size_bytes = artifact_size, "Artifact size");

        // Backup current binary (needs sudo since target is in /usr/bin/).
        let backup_path = target_path.with_extension("bak");
        if target_path.exists() {
            let out = Command::new("sudo")
                .args(["cp", "-f"])
                .arg(&target_path)
                .arg(&backup_path)
                .output()
                .await
                .map_err(|e| format!("Failed to backup current binary: {}", e))?;
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(format!("Failed to backup current binary: {}", stderr.trim()));
            }
            info!(backup = %backup_path.display(), "Backed up current binary");
        }

        // Copy new binary (needs sudo since target is in /usr/bin/).
        let out = Command::new("sudo")
            .args(["cp", "-f"])
            .arg(artifact_path)
            .arg(&target_path)
            .output()
            .await
            .map_err(|e| format!("Failed to copy new binary: {}", e))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("Failed to copy new binary: {}", stderr.trim()));
        }
        debug!(elapsed_ms = start.elapsed().as_millis() as u64, "Binary copied");

        // Set executable permission (needs sudo).
        let out = Command::new("sudo")
            .args(["chmod", "755"])
            .arg(&target_path)
            .output()
            .await
            .map_err(|e| format!("Failed to set permissions: {}", e))?;
        if !out.status.success() {
            warn!("chmod 755 failed: {}", String::from_utf8_lossy(&out.stderr).trim());
        }

        // Restart the service.
        // For chat-shell/full targets, the caller handles restart (pkill)
        // because the running process IS the chat shell.
        match target {
            "chat-shell" | "chat_shell" | "full" => {
                info!(
                    target = target,
                    elapsed_ms = start.elapsed().as_millis() as u64,
                    "Deployed chat-shell binary (caller will handle restart)"
                );
                Ok(format!(
                    "Successfully deployed {} (restart pending)",
                    target_path.display()
                ))
            }
            "engine" => {
                let service_name = "levsha-engine";
                match self.restart_service(service_name).await {
                    Ok(_) => {
                        info!(service = service_name, "Service restarted, waiting for health check");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                        if self.check_service_health(service_name).await {
                            info!(
                                service = service_name,
                                elapsed_ms = start.elapsed().as_millis() as u64,
                                "Deploy successful, service is healthy"
                            );
                            Ok(format!(
                                "Successfully deployed {} and restarted {}",
                                target_path.display(),
                                service_name
                            ))
                        } else {
                            warn!(service = service_name, "Service unhealthy after deploy, rolling back");
                            if backup_path.exists() {
                                Command::new("sudo").args(["cp", "-f"]).arg(&backup_path).arg(&target_path).output().await.ok();
                                self.restart_service(service_name).await.ok();
                            }
                            Err(format!(
                                "Service {} unhealthy after deploy; rolled back to previous version",
                                service_name
                            ))
                        }
                    }
                    Err(e) => {
                        warn!(service = service_name, error = %e, "Service restart failed, rolling back");
                        if backup_path.exists() {
                            std::fs::copy(&backup_path, &target_path).ok();
                            self.restart_service(service_name).await.ok();
                        }
                        Err(format!("Failed to restart service: {}", e))
                    }
                }
            }
            _ => {
                info!(target = target, elapsed_ms = start.elapsed().as_millis() as u64, "Deployed (no service restart)");
                Ok(format!("Deployed to {} (no service restart)", target_path.display()))
            }
        }
    }

    /// Explicit rollback using a checkpoint's binary.
    pub async fn rollback(
        &self,
        target: &str,
        binary_path: &Path,
    ) -> Result<String, String> {
        let target_path = self.resolve_target(target)?;

        info!(
            target = target,
            binary = %binary_path.display(),
            target_path = %target_path.display(),
            "Starting rollback"
        );

        if !binary_path.exists() {
            warn!(binary = %binary_path.display(), "Rollback binary not found");
            return Err(format!("Rollback binary not found: {}", binary_path.display()));
        }

        let binary_size = std::fs::metadata(binary_path)
            .map(|m| m.len())
            .unwrap_or(0);
        debug!(binary_size_bytes = binary_size, "Rollback binary size");

        let out = Command::new("sudo")
            .args(["cp", "-f"])
            .arg(binary_path)
            .arg(&target_path)
            .output()
            .await
            .map_err(|e| format!("Failed to copy rollback binary: {}", e))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("Failed to copy rollback binary: {}", stderr.trim()));
        }

        let _ = Command::new("sudo")
            .args(["chmod", "755"])
            .arg(&target_path)
            .output()
            .await;

        match target {
            "chat-shell" | "chat_shell" | "full" => {
                // For chat-shell targets, restart via pkill.
                let restart_result = self.restart_chat_shell().await;
                match &restart_result {
                    Ok(_) => info!("Chat shell restarted after rollback"),
                    Err(e) => warn!(error = %e, "Chat shell restart failed after rollback"),
                }
                Ok(format!("Rolled back {} and restarted chat shell", target_path.display()))
            }
            "engine" => {
                let service_name = "levsha-engine";
                let restart_result = self.restart_service(service_name).await;
                match &restart_result {
                    Ok(_) => info!(service = service_name, "Service restarted after rollback"),
                    Err(e) => warn!(service = service_name, error = %e, "Service restart failed after rollback"),
                }
                Ok(format!("Rolled back {} and restarted {}", target_path.display(), service_name))
            }
            _ => {
                info!(target = target, "Rolled back (no service restart)");
                Ok(format!("Rolled back {}", target_path.display()))
            }
        }
    }

    /// Resolve a target name to a filesystem path.
    fn resolve_target(&self, target: &str) -> Result<PathBuf, String> {
        let result = match target {
            "chat-shell" | "chat_shell" | "full" => Ok(PathBuf::from("/usr/bin/levsha-chat")),
            "engine" => Ok(PathBuf::from("/usr/bin/levsha-engine")),
            _ => Err(format!("Unknown deploy target: {}", target)),
        };
        match &result {
            Ok(path) => debug!(target = target, resolved = %path.display(), "Resolved deploy target"),
            Err(e) => warn!(target = target, error = %e, "Failed to resolve deploy target"),
        }
        result
    }

    /// Restart the chat shell process by sending SIGTERM.
    pub async fn restart_chat_shell(&self) -> Result<(), String> {
        info!("Restarting chat shell via pkill");
        let output = Command::new("pkill")
            .arg("-x")
            .arg("levsha-chat")
            .output()
            .await
            .map_err(|e| format!("Failed to run pkill: {}", e))?;

        if output.status.success() || output.status.code() == Some(0) {
            info!("pkill levsha-chat succeeded");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(exit_code = ?output.status.code(), stderr = %stderr.trim(), "pkill returned non-zero");
            // Exit code 1 means no process matched — not necessarily an error.
            Ok(())
        }
    }

    /// Restart a systemd service.
    async fn restart_service(&self, service_name: &str) -> Result<(), String> {
        info!(service = service_name, "Restarting service");
        let output = Command::new("systemctl")
            .arg("restart")
            .arg(service_name)
            .output()
            .await
            .map_err(|e| format!("Failed to run systemctl: {}", e))?;

        let exit_code = output.status.code().unwrap_or(-1);
        if output.status.success() {
            debug!(service = service_name, exit_code = exit_code, "Service restart succeeded");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(
                service = service_name,
                exit_code = exit_code,
                stderr = %stderr.trim(),
                "Service restart failed"
            );
            Err(format!("systemctl restart failed: {}", stderr.trim()))
        }
    }

    /// Check if a service is running and healthy.
    async fn check_service_health(&self, service_name: &str) -> bool {
        debug!(service = service_name, "Checking service health");
        let output = Command::new("systemctl")
            .arg("is-active")
            .arg(service_name)
            .output()
            .await;

        match output {
            Ok(o) => {
                let status = String::from_utf8_lossy(&o.stdout);
                let is_healthy = status.trim() == "active";
                debug!(service = service_name, status = %status.trim(), healthy = is_healthy, "Health check result");
                is_healthy
            }
            Err(e) => {
                warn!(service = service_name, error = %e, "Health check failed");
                false
            }
        }
    }
}

#[cfg(test)]
#[path = "deployer_tests.rs"]
mod tests;
