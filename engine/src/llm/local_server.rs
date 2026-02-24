//! Local LLM server lifecycle management (Track D).
//!
//! Manages the llama-server process: starting, stopping, health checking,
//! and restarting. Used when local model inference is enabled.

use std::time::Duration;
use tokio::process::{Child, Command};
use tracing::{debug, error, info, warn};

/// Manages the lifecycle of a local llama-server process.
pub struct LocalServer {
    server_binary: String,
    model_path: String,
    port: u16,
    context_size: u32,
    gpu_layers: u32,
    child: Option<Child>,
}

impl LocalServer {
    /// Create a new local server manager.
    pub fn new(
        server_binary: &str,
        model_path: &str,
        port: u16,
        context_size: u32,
        gpu_layers: u32,
    ) -> Self {
        Self {
            server_binary: server_binary.to_string(),
            model_path: model_path.to_string(),
            port,
            context_size,
            gpu_layers,
            child: None,
        }
    }

    /// Start the llama-server process.
    pub async fn start(&mut self) -> Result<(), String> {
        if self.child.is_some() {
            return Err("Server is already running".to_string());
        }

        info!(
            "Starting llama-server on port {} with model {}",
            self.port, self.model_path
        );

        let mut cmd = Command::new(&self.server_binary);
        cmd.arg("--port")
            .arg(self.port.to_string())
            .arg("--model")
            .arg(&self.model_path)
            .arg("-c")
            .arg(self.context_size.to_string());

        if self.gpu_layers > 0 {
            cmd.arg("-ngl").arg(self.gpu_layers.to_string());
        }

        cmd.stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        match cmd.spawn() {
            Ok(child) => {
                self.child = Some(child);
                info!("llama-server process spawned, waiting for health check...");

                // Wait for the server to become healthy (up to 30 seconds).
                let healthy = self.wait_for_health(Duration::from_secs(30)).await;
                if healthy {
                    info!("llama-server is healthy");
                    Ok(())
                } else {
                    warn!("llama-server did not become healthy in time, stopping");
                    self.stop().await;
                    Err("Server did not become healthy within 30 seconds".to_string())
                }
            }
            Err(e) => {
                error!("Failed to spawn llama-server: {}", e);
                Err(format!("Failed to start llama-server: {}", e))
            }
        }
    }

    /// Stop the llama-server process gracefully.
    pub async fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            info!("Stopping llama-server");

            // Send SIGTERM via kill command on Unix.
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                // Use kill command to send SIGTERM.
                let _ = std::process::Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .status();
            }

            // Wait up to 5 seconds for graceful shutdown.
            match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
                Ok(_) => {
                    info!("llama-server stopped gracefully");
                }
                Err(_) => {
                    warn!("llama-server did not stop gracefully, sending SIGKILL");
                    child.kill().await.ok();
                    child.wait().await.ok();
                }
            }
        }
    }

    /// Check if the server is running and healthy.
    pub async fn is_running(&self) -> bool {
        self.health_check().await
    }

    /// Perform a health check against the server.
    pub async fn health_check(&self) -> bool {
        let url = format!("http://localhost:{}/health", self.port);
        let client = reqwest::Client::new();
        match client
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Restart the server.
    pub async fn restart(&mut self) -> Result<(), String> {
        self.stop().await;
        self.start().await
    }

    /// Wait for the server to become healthy, polling every 500ms.
    async fn wait_for_health(&self, timeout: Duration) -> bool {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() >= timeout {
                return false;
            }
            if self.health_check().await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Get the base URL for the local server.
    pub fn base_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            // Best-effort cleanup on drop.
            #[cfg(unix)]
            if let Some(pid) = child.id() {
                let _ = std::process::Command::new("kill")
                    .arg("-TERM")
                    .arg(pid.to_string())
                    .status();
            }
            #[cfg(not(unix))]
            {
                let _ = child.start_kill();
            }
            debug!("LocalServer dropped, sent termination signal");
        }
    }
}
