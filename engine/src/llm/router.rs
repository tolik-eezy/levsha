//! LLM backend router (Track D).
//!
//! Routes requests to the appropriate backend (cloud or local) based on
//! message characteristics, tool requirements, and user preferences.

use crate::api_client::{ApiError, MessageRequest, StreamResult};
use crate::llm::LlmBackend;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

/// Routing mode for backend selection.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingMode {
    /// Automatically select based on request characteristics.
    Auto,
    /// Always use cloud backend.
    CloudOnly,
    /// Always use local backend.
    LocalOnly,
}

impl RoutingMode {
    /// Parse a routing mode from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "auto" => RoutingMode::Auto,
            "local" | "local_only" | "localonly" => RoutingMode::LocalOnly,
            _ => RoutingMode::CloudOnly,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            RoutingMode::Auto => "auto",
            RoutingMode::CloudOnly => "cloud",
            RoutingMode::LocalOnly => "local",
        }
    }
}

/// Why a particular backend was chosen.
#[derive(Debug, Clone)]
pub enum RoutingDecision {
    /// Request routed to cloud with reason.
    Cloud(String),
    /// Request routed to local with reason.
    Local(String),
}

/// Routes LLM requests between cloud and local backends.
pub struct Router {
    cloud: Arc<dyn LlmBackend>,
    local: Option<Arc<dyn LlmBackend>>,
    mode: RoutingMode,
    last_decision: Option<RoutingDecision>,
}

impl Router {
    /// Create a new router with the given backends.
    pub fn new(
        cloud: Arc<dyn LlmBackend>,
        local: Option<Arc<dyn LlmBackend>>,
        mode: RoutingMode,
    ) -> Self {
        Self {
            cloud,
            local,
            mode,
            last_decision: None,
        }
    }

    /// Get a reference to the cloud backend (trait object).
    pub fn cloud_backend(&self) -> &Arc<dyn LlmBackend> {
        &self.cloud
    }

    /// Decide which backend to route to.
    fn route(&self, _user_message: &str, tool_count: usize) -> RoutingDecision {
        match &self.mode {
            RoutingMode::CloudOnly => {
                RoutingDecision::Cloud("Cloud-only mode".to_string())
            }
            RoutingMode::LocalOnly => {
                if self.local.is_some() {
                    RoutingDecision::Local("Local-only mode".to_string())
                } else {
                    RoutingDecision::Cloud("Local-only mode but no local backend available; falling back to cloud".to_string())
                }
            }
            RoutingMode::Auto => {
                // Rule-based routing:
                // - Use cloud when tools are needed (local models handle tools poorly)
                // - Use local for simple queries without tools
                if tool_count > 0 {
                    return RoutingDecision::Cloud("Request has tools; routing to cloud".to_string());
                }

                if self.local.is_none() {
                    return RoutingDecision::Cloud("No local backend; using cloud".to_string());
                }

                if !self.cloud.is_available() {
                    return RoutingDecision::Local("Cloud unavailable; using local".to_string());
                }

                // Default to local for simple queries when available.
                RoutingDecision::Local("Simple query; routing to local".to_string())
            }
        }
    }

    /// Send a streaming request, routing to the appropriate backend.
    pub async fn send_streaming(
        &mut self,
        request: MessageRequest,
        chunk_tx: &mpsc::Sender<String>,
        cancel: CancellationToken,
    ) -> Result<StreamResult, ApiError> {
        let tool_count = request.tools.len();
        // Use an empty string as message hint since we don't have the raw user message here.
        let decision = self.route("", tool_count);
        self.last_decision = Some(decision.clone());

        match &decision {
            RoutingDecision::Cloud(reason) => {
                debug!("Routing to cloud: {}", reason);
                match self.cloud.send_streaming(request.clone(), chunk_tx, cancel.clone()).await {
                    Ok(result) => Ok(result),
                    Err(ApiError::AuthFailure) if self.local.is_some() => {
                        warn!("Cloud auth failed, falling back to local backend");
                        self.local.as_ref().unwrap()
                            .send_streaming(request, chunk_tx, cancel).await
                    }
                    Err(e) => Err(e),
                }
            }
            RoutingDecision::Local(reason) => {
                debug!("Routing to local: {}", reason);
                match &self.local {
                    Some(local) => {
                        match local.send_streaming(request.clone(), chunk_tx, cancel.clone()).await {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                warn!("Local backend failed ({}), falling back to cloud", e);
                                // Fallback to cloud on local failure.
                                self.cloud.send_streaming(request, chunk_tx, cancel).await
                            }
                        }
                    }
                    None => {
                        warn!("No local backend configured, using cloud");
                        self.cloud.send_streaming(request, chunk_tx, cancel).await
                    }
                }
            }
        }
    }

    /// Set the routing mode.
    pub fn set_mode(&mut self, mode: RoutingMode) {
        info!("Routing mode changed to {:?}", mode);
        self.mode = mode;
    }

    /// Get the current routing mode.
    pub fn current_mode(&self) -> &RoutingMode {
        &self.mode
    }

    /// Get the last routing decision.
    pub fn last_decision(&self) -> Option<&RoutingDecision> {
        self.last_decision.as_ref()
    }

    /// Check if the local backend is available.
    pub async fn check_local_available(&self) -> bool {
        match &self.local {
            Some(local) => local.is_available(),
            None => false,
        }
    }
}
