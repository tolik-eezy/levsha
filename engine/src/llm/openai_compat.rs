//! OpenAI-compatible API client for local LLM servers (Track D).
//!
//! Implements the `LlmBackend` trait for servers that expose an
//! OpenAI-compatible `/v1/chat/completions` endpoint (e.g., llama.cpp server).
//! Handles SSE streaming with OpenAI-format events.

use crate::api_client::{ApiError, MessageRequest, StreamResult, ToolCall};
use crate::llm::LlmBackend;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

/// OpenAI-compatible LLM backend.
///
/// Works with local servers (llama.cpp) and cloud providers that use the
/// OpenAI-compatible API format (e.g., DeepSeek).
pub struct OpenAiCompatBackend {
    client: reqwest::Client,
    base_url: String,
    display_name: String,
    /// Optional API key for authenticated cloud providers (e.g., DeepSeek).
    api_key: Option<String>,
}

impl OpenAiCompatBackend {
    /// Create a new OpenAI-compatible backend for local (unauthenticated) servers.
    pub fn new(base_url: &str, display_name: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // Local models can be slow
            .build()
            .expect("Failed to build HTTP client for local LLM");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            display_name: display_name.to_string(),
            api_key: None,
        }
    }

    /// Create a new OpenAI-compatible backend with API key authentication.
    ///
    /// Used for cloud providers like DeepSeek that use the OpenAI API format
    /// but require Bearer token auth.
    pub fn new_with_auth(base_url: &str, api_key: &str, display_name: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            display_name: display_name.to_string(),
            api_key: if api_key.is_empty() {
                None
            } else {
                Some(api_key.to_string())
            },
        }
    }

    /// Check if the local server is running by hitting the health endpoint.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).timeout(Duration::from_secs(2)).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Convert an Anthropic MessageRequest to OpenAI chat completion format.
    fn convert_request(&self, request: &MessageRequest) -> OpenAiChatRequest {
        let mut messages = Vec::new();

        // System message.
        if !request.system.is_empty() {
            messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: Some(request.system.clone()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Convert API messages.
        for msg in &request.messages {
            for block in &msg.content {
                match block {
                    crate::context::ContentBlock::Text { text } => {
                        messages.push(OpenAiMessage {
                            role: msg.role.clone(),
                            content: Some(text.clone()),
                            tool_calls: None,
                            tool_call_id: None,
                        });
                    }
                    crate::context::ContentBlock::ToolUse { id, name, input } => {
                        messages.push(OpenAiMessage {
                            role: "assistant".to_string(),
                            content: None,
                            tool_calls: Some(vec![OpenAiToolCall {
                                id: id.clone(),
                                call_type: "function".to_string(),
                                function: OpenAiFunction {
                                    name: name.clone(),
                                    arguments: input.to_string(),
                                },
                            }]),
                            tool_call_id: None,
                        });
                    }
                    crate::context::ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => {
                        messages.push(OpenAiMessage {
                            role: "tool".to_string(),
                            content: Some(content.clone()),
                            tool_calls: None,
                            tool_call_id: Some(tool_use_id.clone()),
                        });
                    }
                }
            }
        }

        // Convert tool definitions to OpenAI function format.
        let tools: Option<Vec<OpenAiToolDef>> = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| OpenAiToolDef {
                        tool_type: "function".to_string(),
                        function: OpenAiFunctionDef {
                            name: t.name.clone(),
                            description: t.description.clone(),
                            parameters: t.input_schema.clone(),
                        },
                    })
                    .collect(),
            )
        };

        OpenAiChatRequest {
            model: request.model.clone(),
            messages,
            max_tokens: Some(request.max_tokens),
            stream: true,
            tools,
        }
    }

    /// Parse the OpenAI SSE stream.
    async fn parse_stream(
        &self,
        response: reqwest::Response,
        chunk_tx: &mpsc::Sender<String>,
        cancel: &CancellationToken,
    ) -> Result<StreamResult, ApiError> {
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();
        let mut tool_calls: Vec<PartialToolCall> = Vec::new();

        loop {
            let chunk = tokio::select! {
                chunk = stream.next() => chunk,
                _ = cancel.cancelled() => {
                    return Err(ApiError::Cancelled);
                }
            };

            let chunk = match chunk {
                Some(Ok(bytes)) => bytes,
                Some(Err(e)) => {
                    return Err(ApiError::NetworkError(format!(
                        "Stream read error: {}",
                        e
                    )));
                }
                None => break,
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process SSE lines.
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if data.trim() == "[DONE]" {
                        debug!("OpenAI stream: [DONE]");
                        break;
                    }

                    match serde_json::from_str::<OpenAiStreamChunk>(data) {
                        Ok(chunk) => {
                            for choice in &chunk.choices {
                                // Handle text content.
                                if let Some(content) = &choice.delta.content {
                                    full_text.push_str(content);
                                    chunk_tx.send(content.clone()).await.ok();
                                }
                                // Handle tool calls (accumulate).
                                if let Some(tc_list) = &choice.delta.tool_calls {
                                    for tc in tc_list {
                                        let idx = tc.index.unwrap_or(0) as usize;
                                        while tool_calls.len() <= idx {
                                            tool_calls.push(PartialToolCall::default());
                                        }
                                        if let Some(id) = &tc.id {
                                            tool_calls[idx].id = id.clone();
                                        }
                                        if let Some(f) = &tc.function {
                                            if let Some(name) = &f.name {
                                                tool_calls[idx].name = name.clone();
                                            }
                                            if let Some(args) = &f.arguments {
                                                tool_calls[idx].arguments.push_str(args);
                                            }
                                        }
                                    }
                                }
                                // Check finish reason.
                                if let Some(reason) = &choice.finish_reason {
                                    debug!("OpenAI stream: finish_reason={}", reason);
                                    if reason == "tool_calls" && !tool_calls.is_empty() {
                                        let calls: Vec<ToolCall> = tool_calls
                                            .iter()
                                            .filter(|tc| !tc.name.is_empty())
                                            .map(|tc| ToolCall {
                                                id: if tc.id.is_empty() {
                                                    uuid::Uuid::new_v4().to_string()
                                                } else {
                                                    tc.id.clone()
                                                },
                                                name: tc.name.clone(),
                                                input: serde_json::from_str(&tc.arguments)
                                                    .unwrap_or(serde_json::Value::Object(
                                                        serde_json::Map::new(),
                                                    )),
                                            })
                                            .collect();
                                        return Ok(StreamResult::ToolUse {
                                            text_before: full_text,
                                            tool_calls: calls,
                                        });
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse OpenAI stream chunk: {}", e);
                        }
                    }
                }
            }
        }

        Ok(StreamResult::TextComplete(full_text))
    }
}

#[async_trait]
impl LlmBackend for OpenAiCompatBackend {
    async fn send_streaming(
        &self,
        request: MessageRequest,
        chunk_tx: &mpsc::Sender<String>,
        cancel: CancellationToken,
    ) -> Result<StreamResult, ApiError> {
        let openai_request = self.convert_request(&request);
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut req_builder = self
            .client
            .post(&url)
            .header("content-type", "application/json");

        // Add auth header for cloud providers (e.g., DeepSeek).
        if let Some(ref key) = self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", key));
        }

        let http_response = tokio::select! {
            result = req_builder.json(&openai_request).send() => {
                match result {
                    Ok(resp) => resp,
                    Err(e) => {
                        if e.is_timeout() {
                            return Err(ApiError::Timeout);
                        }
                        let label = if self.api_key.is_some() { &self.display_name } else { "Local LLM" };
                        return Err(ApiError::NetworkError(format!(
                            "{} connection failed: {}",
                            label, e
                        )));
                    }
                }
            }
            _ = cancel.cancelled() => {
                return Err(ApiError::Cancelled);
            }
        };

        let status = http_response.status();
        if !status.is_success() {
            let body = http_response.text().await.unwrap_or_default();
            let label = if self.api_key.is_some() { &self.display_name } else { "Local LLM" };
            error!("{} error (HTTP {}): {}", label, status.as_u16(), body);
            if status.as_u16() == 401 || status.as_u16() == 403 {
                return Err(ApiError::AuthFailure);
            }
            return Err(ApiError::ServerError(status.as_u16()));
        }

        self.parse_stream(http_response, chunk_tx, &cancel).await
    }

    fn name(&self) -> &str {
        &self.display_name
    }

    fn is_available(&self) -> bool {
        // For authenticated cloud providers, require an API key.
        // For local servers (no api_key configured), always report available
        // (actual availability is checked via health_check()).
        match &self.api_key {
            Some(key) => !key.is_empty(),
            None => true,
        }
    }
}

// ---------------------------------------------------------------------------
// OpenAI API types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAiToolDef>>,
}

/// OpenAI tool definition (function calling).
#[derive(Debug, Serialize)]
struct OpenAiToolDef {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAiFunctionDef,
}

/// OpenAI function definition within a tool.
#[derive(Debug, Serialize)]
struct OpenAiFunctionDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiStreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamToolCall {
    index: Option<u32>,
    id: Option<String>,
    function: Option<OpenAiStreamFunction>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}
