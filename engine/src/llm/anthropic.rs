//! Anthropic Claude API backend adapter (Track D).
//!
//! Wraps the existing `ApiClient` to implement the `LlmBackend` trait,
//! providing a thin adapter layer for the cloud API.

use crate::api_client::{ApiClient, ApiError, MessageRequest, StreamResult};
use crate::llm::LlmBackend;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Anthropic Claude cloud API backend.
pub struct AnthropicBackend {
    /// The underlying API client, behind a tokio Mutex for Send safety.
    client: Mutex<ApiClient>,
    /// Display name for this backend.
    display_name: String,
    /// Whether an API key is configured.
    has_key: AtomicBool,
}

impl AnthropicBackend {
    /// Create a new Anthropic backend from an existing API client.
    pub fn new(client: ApiClient, display_name: String, has_key: bool) -> Self {
        Self {
            client: Mutex::new(client),
            display_name,
            has_key: AtomicBool::new(has_key),
        }
    }

}

#[async_trait]
impl LlmBackend for AnthropicBackend {
    async fn send_streaming(
        &self,
        request: MessageRequest,
        chunk_tx: &mpsc::Sender<String>,
        cancel: CancellationToken,
    ) -> Result<StreamResult, ApiError> {
        let client = self.client.lock().await;
        client.send_streaming(request, chunk_tx, cancel).await
    }

    fn name(&self) -> &str {
        &self.display_name
    }

    fn is_available(&self) -> bool {
        self.has_key.load(Ordering::Relaxed)
    }

    async fn set_api_key(&self, key: &str) {
        let mut client = self.client.lock().await;
        client.set_api_key(key);
        self.has_key.store(!key.is_empty(), Ordering::Relaxed);
    }

    async fn set_model(&self, model: &str) {
        let mut client = self.client.lock().await;
        client.set_model(model);
    }
}
