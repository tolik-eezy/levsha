//! Anthropic Claude API client with SSE streaming support.
//!
//! Handles: connection, streaming responses, tool_use parsing, error handling,
//! exponential backoff retries.

use crate::context::ApiMessage;
use crate::skill_loader::ToolDefinition;
use futures::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Authentication failed: invalid or expired API key")]
    AuthFailure,

    #[error("Rate limited, retry after {0:?}")]
    RateLimited(Duration),

    #[error("Server error (HTTP {0})")]
    ServerError(u16),

    #[error("Request timed out")]
    Timeout,

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Malformed response: {0}")]
    MalformedResponse(String),

    #[error("Stream cancelled")]
    Cancelled,
}

impl ApiError {
    /// Whether this error class is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ApiError::RateLimited(_)
                | ApiError::ServerError(_)
                | ApiError::Timeout
                | ApiError::NetworkError(_)
        )
    }

    /// User-friendly error message.
    pub fn user_message(&self) -> String {
        match self {
            ApiError::AuthFailure => {
                "API key is invalid or expired. Please check the configuration.".to_string()
            }
            ApiError::RateLimited(d) => {
                format!("Rate limited. Please wait {} seconds.", d.as_secs())
            }
            ApiError::ServerError(code) => {
                format!("The AI service returned an error (HTTP {}). Please try again.", code)
            }
            ApiError::Timeout => "The request timed out. Please try again.".to_string(),
            ApiError::NetworkError(_) => {
                "Unable to reach the AI service. Check your internet connection.".to_string()
            }
            ApiError::MalformedResponse(detail) => {
                format!("Unexpected response: {}. Please try again.", detail)
            }
            ApiError::Cancelled => "Request cancelled.".to_string(),
        }
    }

    pub fn is_retryable_for_user(&self) -> bool {
        !matches!(self, ApiError::AuthFailure | ApiError::Cancelled)
    }
}

// ---------------------------------------------------------------------------
// API request/response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ApiTool>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl ApiTool {
    pub fn from_definition(def: &ToolDefinition) -> Self {
        Self {
            name: def.name.clone(),
            description: def.description.clone(),
            input_schema: def.input_schema.clone(),
        }
    }
}

/// A parsed tool call from the API response.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

/// The result of streaming an API response.
#[derive(Debug)]
pub enum StreamResult {
    /// Response complete with text only.
    TextComplete(String),
    /// Response ended with tool calls to execute.
    ToolUse {
        text_before: String,
        tool_calls: Vec<ToolCall>,
    },
}

// ---------------------------------------------------------------------------
// SSE event parsing types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SseContentBlockStart {
    index: usize,
    content_block: SseContentBlockInfo,
}

#[derive(Debug, Deserialize)]
struct SseContentBlockInfo {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseContentBlockDelta {
    index: usize,
    delta: SseDelta,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SseDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Debug, Deserialize)]
struct SseMessageDelta {
    delta: SseMessageDeltaInner,
}

#[derive(Debug, Deserialize)]
struct SseMessageDeltaInner {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SseErrorData {
    error: SseErrorDetail,
}

#[derive(Debug, Deserialize)]
struct SseErrorDetail {
    message: String,
}

// ---------------------------------------------------------------------------
// API Client
// ---------------------------------------------------------------------------

pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    timeout: Duration,
    max_retries: u32,
}

impl ApiClient {
    pub fn new(
        base_url: &str,
        api_key: &str,
        model: &str,
        timeout_seconds: u64,
        max_retries: u32,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_seconds + 60)) // HTTP timeout slightly above our own
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
            timeout: Duration::from_secs(timeout_seconds),
            max_retries,
        }
    }

    /// Send a message to the API and stream the response.
    ///
    /// Streams text chunks to `chunk_sender`. Returns the final StreamResult
    /// which indicates whether the response was text-only or includes tool calls.
    ///
    /// Supports cancellation via the `cancel` token.
    pub async fn send_streaming(
        &self,
        request: MessageRequest,
        chunk_sender: &mpsc::Sender<String>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> Result<StreamResult, ApiError> {
        let delays = self.retry_delays();

        for (attempt, delay) in delays.iter().enumerate() {
            if !delay.is_zero() {
                debug!("Retry attempt {} after {:?}", attempt + 1, delay);
                tokio::time::sleep(*delay).await;
            }

            if cancel.is_cancelled() {
                return Err(ApiError::Cancelled);
            }

            match self
                .send_streaming_once(&request, chunk_sender, &cancel)
                .await
            {
                Ok(result) => return Ok(result),
                Err(ApiError::RateLimited(wait)) if attempt < delays.len() - 1 => {
                    warn!("Rate limited (attempt {}), waiting {:?}", attempt + 1, wait);
                    tokio::time::sleep(wait).await;
                    continue;
                }
                Err(e) if e.is_retryable() && attempt < delays.len() - 1 => {
                    warn!("API request failed (attempt {}): {}", attempt + 1, e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    /// Single attempt to send and stream.
    async fn send_streaming_once(
        &self,
        request: &MessageRequest,
        chunk_sender: &mpsc::Sender<String>,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<StreamResult, ApiError> {
        let url = format!("{}/v1/messages", self.base_url);

        let http_response = tokio::select! {
            result = self.client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(request)
                .send() => {
                match result {
                    Ok(resp) => resp,
                    Err(e) => {
                        if e.is_timeout() {
                            return Err(ApiError::Timeout);
                        }
                        return Err(ApiError::NetworkError(e.to_string()));
                    }
                }
            }
            _ = cancel.cancelled() => {
                return Err(ApiError::Cancelled);
            }
        };

        // Handle non-200 status codes.
        let status = http_response.status();
        if status != StatusCode::OK {
            return Err(classify_error_status(status, http_response).await);
        }

        // Parse the SSE stream.
        self.parse_sse_stream(http_response, chunk_sender, cancel)
            .await
    }

    /// Parse the SSE event stream from a successful response.
    async fn parse_sse_stream(
        &self,
        response: reqwest::Response,
        chunk_sender: &mpsc::Sender<String>,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<StreamResult, ApiError> {
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut full_text = String::new();

        // Track content blocks for tool_use.
        let mut current_blocks: Vec<BlockState> = Vec::new();

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
                None => {
                    // Stream ended without message_stop — treat as complete.
                    break;
                }
            };

            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE events from the buffer.
            while let Some(event) = take_sse_event(&mut buffer) {
                let (event_type, data) = match parse_sse_fields(&event) {
                    Some(fields) => fields,
                    None => continue,
                };

                match event_type.as_str() {
                    "message_start" => {
                        // We can parse message_id here if needed.
                        debug!("SSE: message_start");
                    }
                    "content_block_start" => {
                        if let Ok(block_start) =
                            serde_json::from_str::<SseContentBlockStart>(&data)
                        {
                            let state = BlockState {
                                index: block_start.index,
                                block_type: block_start.content_block.block_type.clone(),
                                tool_id: block_start.content_block.id.clone(),
                                tool_name: block_start.content_block.name.clone(),
                                accumulated_json: String::new(),
                            };
                            // Ensure vec is large enough.
                            while current_blocks.len() <= block_start.index {
                                current_blocks.push(BlockState::default());
                            }
                            current_blocks[block_start.index] = state;
                        }
                    }
                    "content_block_delta" => {
                        if let Ok(delta) =
                            serde_json::from_str::<SseContentBlockDelta>(&data)
                        {
                            match delta.delta {
                                SseDelta::TextDelta { text } => {
                                    full_text.push_str(&text);
                                    chunk_sender.send(text).await.ok();
                                }
                                SseDelta::InputJsonDelta { partial_json } => {
                                    if let Some(block) =
                                        current_blocks.get_mut(delta.index)
                                    {
                                        block.accumulated_json.push_str(&partial_json);
                                    }
                                }
                            }
                        }
                    }
                    "content_block_stop" => {
                        debug!("SSE: content_block_stop");
                    }
                    "message_delta" => {
                        if let Ok(msg_delta) =
                            serde_json::from_str::<SseMessageDelta>(&data)
                        {
                            if let Some(reason) = &msg_delta.delta.stop_reason {
                                debug!("SSE: stop_reason = {}", reason);
                                if reason == "tool_use" {
                                    // Collect tool calls.
                                    let tool_calls: Vec<ToolCall> = current_blocks
                                        .iter()
                                        .filter(|b| b.block_type == "tool_use")
                                        .filter_map(|b| {
                                            let input: serde_json::Value =
                                                serde_json::from_str(&b.accumulated_json)
                                                    .unwrap_or(serde_json::Value::Object(
                                                        serde_json::Map::new(),
                                                    ));
                                            Some(ToolCall {
                                                id: b.tool_id.clone()?,
                                                name: b.tool_name.clone()?,
                                                input,
                                            })
                                        })
                                        .collect();

                                    return Ok(StreamResult::ToolUse {
                                        text_before: full_text,
                                        tool_calls,
                                    });
                                }
                            }
                        }
                    }
                    "message_stop" => {
                        debug!("SSE: message_stop");
                        return Ok(StreamResult::TextComplete(full_text));
                    }
                    "ping" => {}
                    "error" => {
                        if let Ok(err_data) = serde_json::from_str::<SseErrorData>(&data)
                        {
                            error!("SSE error: {}", err_data.error.message);
                            return Err(ApiError::MalformedResponse(
                                err_data.error.message,
                            ));
                        }
                    }
                    other => {
                        debug!("SSE: unknown event type '{}'", other);
                    }
                }
            }
        }

        Ok(StreamResult::TextComplete(full_text))
    }

    /// Validate an API key by sending a minimal request to the Haiku model.
    /// Returns Ok(()) if the key is valid, or an ApiError otherwise.
    pub async fn validate_key(
        base_url: &str,
        key: &str,
        timeout: Duration,
    ) -> Result<(), ApiError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| ApiError::NetworkError(e.to_string()))?;

        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let response = client
            .post(&url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ApiError::Timeout
                } else {
                    ApiError::NetworkError(e.to_string())
                }
            })?;

        let status = response.status();
        if status == StatusCode::OK {
            Ok(())
        } else {
            Err(classify_error_status(status, response).await)
        }
    }

    /// Generate a short session name from a user message using a lightweight API call.
    /// Returns a 2-4 word title, or None on failure.
    pub async fn generate_session_name(
        base_url: &str,
        key: &str,
        user_message: &str,
        timeout: Duration,
    ) -> Option<String> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .ok()?;

        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        // Use the fastest model with minimal tokens.
        let prompt = format!(
            "Give a 2-4 word title for this chat message. \
             Reply with ONLY the title, no quotes or punctuation.\n\n{}",
            &user_message[..user_message.len().min(200)]
        );

        let body = serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 20,
            "messages": [{"role": "user", "content": prompt}]
        });

        let response = client
            .post(&url)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            return None;
        }

        let json: serde_json::Value = response.json().await.ok()?;
        let name = json
            .get("content")?
            .as_array()?
            .first()?
            .get("text")?
            .as_str()?
            .trim()
            .to_string();

        if name.is_empty() || name.len() > 50 {
            None
        } else {
            Some(name)
        }
    }

    /// Update the API key used by this client.
    pub fn set_api_key(&mut self, key: &str) {
        self.api_key = key.to_string();
    }

    /// Update the model used by this client.
    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    fn retry_delays(&self) -> Vec<Duration> {
        let mut delays = vec![Duration::ZERO];
        for i in 0..self.max_retries.saturating_sub(1) {
            delays.push(Duration::from_secs(1 << i)); // 1s, 2s, 4s, ...
        }
        delays
    }
}

// ---------------------------------------------------------------------------
// Block tracking state
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
struct BlockState {
    index: usize,
    block_type: String,
    tool_id: Option<String>,
    tool_name: Option<String>,
    accumulated_json: String,
}

// ---------------------------------------------------------------------------
// SSE parsing helpers
// ---------------------------------------------------------------------------

/// Take the next complete SSE event from the buffer (delimited by double newline).
fn take_sse_event(buffer: &mut String) -> Option<String> {
    if let Some(pos) = buffer.find("\n\n") {
        let event = buffer[..pos].to_string();
        *buffer = buffer[pos + 2..].to_string();
        Some(event)
    } else {
        None
    }
}

/// Parse SSE event fields into (event_type, data).
fn parse_sse_fields(event: &str) -> Option<(String, String)> {
    let mut event_type = String::new();
    let mut data_lines: Vec<String> = Vec::new();

    for line in event.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            event_type = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data: ") {
            data_lines.push(value.to_string());
        } else if line.starts_with("event:") {
            event_type = line[6..].trim().to_string();
        } else if line.starts_with("data:") {
            data_lines.push(line[5..].trim().to_string());
        }
    }

    if event_type.is_empty() && data_lines.is_empty() {
        return None;
    }

    let data = data_lines.join("\n");
    Some((event_type, data))
}

/// Classify a non-200 HTTP response into an ApiError.
/// Reads the response body to extract the API error message.
async fn classify_error_status(status: StatusCode, response: reqwest::Response) -> ApiError {
    // Extract retry-after header before consuming the response.
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);

    // Try to read the response body for error details.
    let body = response.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v["error"]["message"].as_str().map(String::from))
        .unwrap_or_else(|| body.chars().take(200).collect());

    error!("API error (HTTP {}): {}", status.as_u16(), detail);

    match status.as_u16() {
        401 => ApiError::AuthFailure,
        429 => ApiError::RateLimited(Duration::from_secs(retry_after)),
        408 => ApiError::Timeout,
        500 | 502 | 503 | 529 => ApiError::ServerError(status.as_u16()),
        _ => ApiError::MalformedResponse(format!("HTTP {}: {}", status.as_u16(), detail)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // take_sse_event
    // -----------------------------------------------------------------------

    #[test]
    fn take_sse_event_complete() {
        let mut buf = "event: message_start\ndata: {}\n\nremaining".to_string();
        let event = take_sse_event(&mut buf);
        assert!(event.is_some());
        assert_eq!(event.unwrap(), "event: message_start\ndata: {}");
        assert_eq!(buf, "remaining");
    }

    #[test]
    fn take_sse_event_no_double_newline() {
        let mut buf = "event: message_start\ndata: {}".to_string();
        let event = take_sse_event(&mut buf);
        assert!(event.is_none());
        // Buffer should be unchanged
        assert_eq!(buf, "event: message_start\ndata: {}");
    }

    #[test]
    fn take_sse_event_empty_buffer() {
        let mut buf = String::new();
        assert!(take_sse_event(&mut buf).is_none());
    }

    #[test]
    fn take_sse_event_multiple_events() {
        let mut buf = "event: ping\ndata: {}\n\nevent: message_stop\ndata: {}\n\n".to_string();
        let first = take_sse_event(&mut buf);
        assert_eq!(first.unwrap(), "event: ping\ndata: {}");
        let second = take_sse_event(&mut buf);
        assert_eq!(second.unwrap(), "event: message_stop\ndata: {}");
        assert!(take_sse_event(&mut buf).is_none());
    }

    #[test]
    fn take_sse_event_only_double_newline() {
        let mut buf = "\n\nafter".to_string();
        let event = take_sse_event(&mut buf);
        assert_eq!(event.unwrap(), "");
        assert_eq!(buf, "after");
    }

    // -----------------------------------------------------------------------
    // parse_sse_fields
    // -----------------------------------------------------------------------

    #[test]
    fn parse_sse_fields_event_and_data() {
        let event = "event: content_block_delta\ndata: {\"index\":0}";
        let result = parse_sse_fields(event);
        assert!(result.is_some());
        let (event_type, data) = result.unwrap();
        assert_eq!(event_type, "content_block_delta");
        assert_eq!(data, "{\"index\":0}");
    }

    #[test]
    fn parse_sse_fields_no_space_after_colon() {
        let event = "event:ping\ndata:{\"type\":\"ping\"}";
        let result = parse_sse_fields(event);
        assert!(result.is_some());
        let (event_type, data) = result.unwrap();
        assert_eq!(event_type, "ping");
        assert_eq!(data, "{\"type\":\"ping\"}");
    }

    #[test]
    fn parse_sse_fields_empty_input() {
        assert!(parse_sse_fields("").is_none());
    }

    #[test]
    fn parse_sse_fields_only_event_no_data() {
        let result = parse_sse_fields("event: ping");
        assert!(result.is_some());
        let (event_type, data) = result.unwrap();
        assert_eq!(event_type, "ping");
        assert_eq!(data, "");
    }

    #[test]
    fn parse_sse_fields_multiline_data() {
        let event = "event: test\ndata: line1\ndata: line2";
        let result = parse_sse_fields(event);
        let (_, data) = result.unwrap();
        assert_eq!(data, "line1\nline2");
    }

    #[test]
    fn parse_sse_fields_irrelevant_lines_ignored() {
        let event = "id: 123\nevent: ping\ndata: {}";
        let result = parse_sse_fields(event);
        let (event_type, data) = result.unwrap();
        assert_eq!(event_type, "ping");
        assert_eq!(data, "{}");
    }

    // -----------------------------------------------------------------------
    // ApiError
    // -----------------------------------------------------------------------

    #[test]
    fn api_error_retryable() {
        assert!(ApiError::RateLimited(Duration::from_secs(5)).is_retryable());
        assert!(ApiError::ServerError(500).is_retryable());
        assert!(ApiError::Timeout.is_retryable());
        assert!(ApiError::NetworkError("conn reset".into()).is_retryable());
        assert!(!ApiError::AuthFailure.is_retryable());
        assert!(!ApiError::Cancelled.is_retryable());
    }

    #[test]
    fn api_error_user_message_not_empty() {
        let errors = vec![
            ApiError::AuthFailure,
            ApiError::RateLimited(Duration::from_secs(5)),
            ApiError::ServerError(500),
            ApiError::Timeout,
            ApiError::NetworkError("test".into()),
            ApiError::MalformedResponse("bad".into()),
            ApiError::Cancelled,
        ];
        for e in errors {
            assert!(!e.user_message().is_empty());
        }
    }

    #[test]
    fn api_error_retryable_for_user() {
        assert!(!ApiError::AuthFailure.is_retryable_for_user());
        assert!(!ApiError::Cancelled.is_retryable_for_user());
        assert!(ApiError::ServerError(500).is_retryable_for_user());
        assert!(ApiError::Timeout.is_retryable_for_user());
    }
}
