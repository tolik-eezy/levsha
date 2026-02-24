//! LLM backend abstraction layer (Track D).
//!
//! Provides a unified trait for different LLM backends (Anthropic cloud,
//! OpenAI-compatible local servers) and a router that selects the appropriate
//! backend based on message characteristics and user preferences.

pub mod anthropic;
pub mod local_server;
pub mod model_manager;
pub mod openai_compat;
pub mod router;

use crate::api_client::{ApiError, MessageRequest, StreamResult};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Trait for LLM backends that can send streaming requests.
#[async_trait]
pub trait LlmBackend: Send + Sync {
    /// Send a streaming request to this backend.
    async fn send_streaming(
        &self,
        request: MessageRequest,
        chunk_tx: &mpsc::Sender<String>,
        cancel: CancellationToken,
    ) -> Result<StreamResult, ApiError>;

    /// Display name of this backend.
    fn name(&self) -> &str;

    /// Whether this backend is currently available.
    fn is_available(&self) -> bool;

    /// Update the API key at runtime (e.g. during initial key setup).
    /// Default implementation is a no-op for backends that don't support it.
    async fn set_api_key(&self, _key: &str) {}

    /// Update the model at runtime (e.g. when user switches models).
    /// Default implementation is a no-op for backends that don't support it.
    async fn set_model(&self, _model: &str) {}
}
