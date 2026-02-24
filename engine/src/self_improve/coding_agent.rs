//! Coding agent integration for self-improvement (Track E).
//!
//! Spawns an external coding agent (Claude Code or OpenCode) as a subprocess
//! to handle source code modifications. Parses the agent's JSONL output into
//! structured events for the UI.

use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Which coding agent backend to use.
#[derive(Debug, Clone)]
pub enum CodingAgentBackend {
    ClaudeCode,
    OpenCode,
}

impl CodingAgentBackend {
    /// Parse a backend from a config string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "opencode" | "open-code" => CodingAgentBackend::OpenCode,
            _ => CodingAgentBackend::ClaudeCode,
        }
    }
}

/// Configuration for the coding agent subprocess.
#[derive(Debug, Clone)]
pub struct CodingAgentConfig {
    pub backend: CodingAgentBackend,
    pub binary_path: PathBuf,
    pub source_root: PathBuf,
    pub timeout: Duration,
    pub max_turns: Option<usize>,
    /// Model override (e.g. "haiku", "sonnet"). None = agent default.
    pub model: Option<String>,
    /// API key to pass to the agent subprocess.
    pub api_key: Option<String>,
    /// Environment variable name for the API key (e.g. "ANTHROPIC_API_KEY" or "DEEPSEEK_API_KEY").
    pub api_key_env: String,
}

/// Structured events parsed from the coding agent's JSONL output.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    Thinking { content: String },
    FileRead { path: String, content_preview: Option<String> },
    FileEdit { path: String, diff_snippet: Option<String> },
    BashCommand { command: String, output_preview: Option<String> },
    CodeSearch { pattern: String, match_count: Option<usize> },
    Complete { summary: String, duration: Duration },
    Error { message: String },
    Timeout,
    Unknown { raw: String },
}

/// A running coding agent session.
pub struct CodingAgentSession {
    child: Child,
    _forward_handle: tokio::task::JoinHandle<()>,
    start_time: std::time::Instant,
}

impl CodingAgentSession {
    /// Spawn a new coding agent subprocess.
    ///
    /// The agent is started with the given prompt and its JSONL output is
    /// parsed into `AgentEvent`s sent through `event_tx`.
    pub async fn spawn(
        config: &CodingAgentConfig,
        prompt: &str,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> anyhow::Result<Self> {
        let mut cmd = Command::new(&config.binary_path);

        match config.backend {
            CodingAgentBackend::ClaudeCode => {
                cmd.arg("--print")
                    .arg("--verbose")
                    .arg("--output-format")
                    .arg("stream-json")
                    .arg("--dangerously-skip-permissions");

                if let Some(max_turns) = config.max_turns {
                    // Claude Code has no --max-turns; use --max-budget-usd as a proxy.
                    // Each turn ≈ $0.02 for Sonnet, so max_turns * 0.03 gives headroom.
                    let budget = (max_turns as f64) * 0.03;
                    cmd.arg("--max-budget-usd").arg(format!("{:.2}", budget));
                }

                if let Some(ref model) = config.model {
                    cmd.arg("--model").arg(model);
                }

                // Restrict tools to only what's needed for code changes.
                cmd.arg("--allowedTools")
                    .arg("Read,Edit,Write,Bash,Grep,Glob");

                // Prompt is a positional argument in Claude Code.
                cmd.arg(prompt);
            }
            CodingAgentBackend::OpenCode => {
                // OpenCode CLI: `opencode run --format json [-m provider/model] <prompt>`
                cmd.arg("run")
                    .arg("--format")
                    .arg("json");

                if let Some(ref model) = config.model {
                    cmd.arg("-m").arg(model);
                }

                cmd.arg(prompt);
            }
        }

        // Pass API key to the agent subprocess using the configured env var name.
        if let Some(ref key) = config.api_key {
            cmd.env(&config.api_key_env, key);
        }

        cmd.current_dir(&config.source_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        info!(
            backend = ?config.backend,
            binary = %config.binary_path.display(),
            source_root = %config.source_root.display(),
            "Spawning coding agent"
        );

        let mut child = cmd.spawn().map_err(|e| {
            anyhow::anyhow!(
                "Failed to spawn coding agent ({}): {}",
                config.binary_path.display(),
                e
            )
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            anyhow::anyhow!("Failed to capture coding agent stdout")
        })?;

        let timeout = config.timeout;
        let start_time = std::time::Instant::now();
        let event_tx_clone = event_tx.clone();

        // Spawn a task to read JSONL lines from stdout and parse them.
        let forward_handle = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            loop {
                let line_result = tokio::time::timeout(timeout, lines.next_line()).await;

                match line_result {
                    Ok(Ok(Some(line))) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        let event = parse_jsonl_event(&line);
                        if event_tx_clone.send(event).await.is_err() {
                            debug!("Event receiver dropped, stopping agent output parsing");
                            break;
                        }
                    }
                    Ok(Ok(None)) => {
                        // EOF — agent process exited.
                        debug!("Coding agent stdout EOF");
                        break;
                    }
                    Ok(Err(e)) => {
                        warn!("Error reading coding agent output: {}", e);
                        let _ = event_tx_clone
                            .send(AgentEvent::Error {
                                message: format!("IO error: {}", e),
                            })
                            .await;
                        break;
                    }
                    Err(_) => {
                        // Timeout reading next line.
                        warn!("Coding agent timed out after {:?}", timeout);
                        let _ = event_tx_clone.send(AgentEvent::Timeout).await;
                        break;
                    }
                }
            }
        });

        Ok(Self {
            child,
            _forward_handle: forward_handle,
            start_time,
        })
    }

    /// Terminate the agent process gracefully: SIGTERM, wait 5s, then SIGKILL.
    pub async fn kill(&mut self) -> anyhow::Result<()> {
        info!("Sending SIGTERM to coding agent");

        // Send SIGTERM on Unix.
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            if let Some(pid) = self.child.id() {
                let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
            }
        }

        #[cfg(not(unix))]
        {
            self.child.kill().await.ok();
        }

        // Wait up to 5 seconds for graceful exit.
        match tokio::time::timeout(Duration::from_secs(5), self.child.wait()).await {
            Ok(Ok(status)) => {
                info!(?status, "Coding agent exited after SIGTERM");
                Ok(())
            }
            _ => {
                warn!("Coding agent did not exit after SIGTERM, sending SIGKILL");
                self.child.kill().await.map_err(|e| {
                    anyhow::anyhow!("Failed to SIGKILL coding agent: {}", e)
                })?;
                Ok(())
            }
        }
    }

    /// Wait for the agent process to exit.
    pub async fn wait(&mut self) -> anyhow::Result<ExitStatus> {
        let status = self.child.wait().await.map_err(|e| {
            anyhow::anyhow!("Failed to wait for coding agent: {}", e)
        })?;

        let elapsed = self.start_time.elapsed();
        info!(?status, elapsed_secs = elapsed.as_secs(), "Coding agent exited");

        Ok(status)
    }
}

// ---------------------------------------------------------------------------
// JSONL event parsing
// ---------------------------------------------------------------------------

/// Intermediate structs for deserializing coding agent JSONL output.
/// Supports both Claude Code (stream-json) and OpenCode (--format json) formats.
#[derive(Debug, Deserialize)]
struct JsonlMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    subtype: Option<String>,
    // For "assistant" messages (Claude Code).
    message: Option<AssistantMessage>,
    // Error field on assistant messages (e.g. "billing_error").
    error: Option<String>,
    // For "result" messages (Claude Code).
    result: Option<String>,
    // Whether the result is an error.
    is_error: Option<bool>,
    // Duration in milliseconds (Claude Code uses duration_ms).
    duration_ms: Option<f64>,
    // Duration in seconds (fallback).
    duration_seconds: Option<f64>,
    // For "user" tool_result messages (Claude Code).
    tool_use_result: Option<ToolUseResult>,
    // OpenCode: nested part object containing event details.
    part: Option<OpenCodePart>,
}

/// OpenCode event part — the payload inside each JSON event.
#[derive(Debug, Deserialize)]
struct OpenCodePart {
    #[serde(rename = "type")]
    part_type: Option<String>,
    // "text" events.
    text: Option<String>,
    // "step-finish" events.
    reason: Option<String>,
    cost: Option<f64>,
    tokens: Option<OpenCodeTokens>,
    // "tool" events (type: "tool_use") — tool name and state.
    tool: Option<String>,
    state: Option<OpenCodeToolState>,
}

/// OpenCode tool state — nested inside tool_use events.
#[derive(Debug, Deserialize)]
struct OpenCodeToolState {
    status: Option<String>,
    input: Option<serde_json::Value>,
    output: Option<String>,
}

/// Token usage from OpenCode step-finish events.
#[derive(Debug, Deserialize)]
struct OpenCodeTokens {
    total: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Option<Vec<ContentBlock>>,
}

/// Tool execution result from "user" type messages.
#[derive(Debug, Deserialize)]
struct ToolUseResult {
    stdout: Option<String>,
    stderr: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: Option<String>,
    // For thinking blocks.
    thinking: Option<String>,
    // For text blocks.
    text: Option<String>,
    // For tool_result blocks (content is a string).
    content: Option<serde_json::Value>,
    // For tool_use blocks.
    name: Option<String>,
    input: Option<serde_json::Value>,
}

/// Parse a single JSONL line from the coding agent into an `AgentEvent`.
pub fn parse_jsonl_event(line: &str) -> AgentEvent {
    let msg: JsonlMessage = match serde_json::from_str(line) {
        Ok(m) => m,
        Err(_) => {
            debug!(line, "Unparseable JSONL line from coding agent");
            return AgentEvent::Unknown {
                raw: truncate(line, 500),
            };
        }
    };

    let msg_type = msg.msg_type.as_deref().unwrap_or("");

    match msg_type {
        // ── Claude Code event types ──
        "system" => {
            // Init message — skip silently.
            debug!("Coding agent system init message");
            AgentEvent::Thinking {
                content: "Agent initializing...".to_string(),
            }
        }
        "assistant" => parse_assistant_event(&msg),
        "user" => parse_tool_result_event(&msg),
        "result" => parse_result_event(&msg),

        // ── OpenCode event types ──
        "step_start" => {
            AgentEvent::Thinking {
                content: "Agent starting step...".to_string(),
            }
        }
        "text" => {
            let text = msg
                .part
                .as_ref()
                .and_then(|p| p.text.as_deref())
                .unwrap_or("");
            if text.is_empty() {
                AgentEvent::Thinking {
                    content: "(empty)".to_string(),
                }
            } else {
                AgentEvent::Thinking {
                    content: truncate(text, 1000),
                }
            }
        }
        "tool_use" => parse_opencode_tool_call(&msg),
        "step_finish" => {
            let part = msg.part.as_ref();
            let reason = part.and_then(|p| p.reason.as_deref()).unwrap_or("done");
            let cost = part.and_then(|p| p.cost).unwrap_or(0.0);
            AgentEvent::Complete {
                summary: format!("Step finished (reason: {}, cost: ${:.4})", reason, cost),
                duration: Duration::default(),
            }
        }

        _ => AgentEvent::Unknown {
            raw: truncate(line, 500),
        },
    }
}

/// Parse an "assistant" type message into an AgentEvent.
fn parse_assistant_event(msg: &JsonlMessage) -> AgentEvent {
    // Check for top-level error (e.g. billing_error).
    if let Some(ref error) = msg.error {
        if let Some(ref message) = msg.message {
            if let Some(ref blocks) = message.content {
                for block in blocks {
                    if let Some(ref text) = block.text {
                        return AgentEvent::Error {
                            message: format!("{}: {}", error, text),
                        };
                    }
                }
            }
        }
        return AgentEvent::Error {
            message: error.clone(),
        };
    }

    let content_blocks = match msg.message.as_ref().and_then(|m| m.content.as_ref()) {
        Some(blocks) => blocks,
        None => {
            return AgentEvent::Unknown {
                raw: "assistant message with no content".to_string(),
            }
        }
    };

    // Check if this is a thinking block.
    if msg.subtype.as_deref() == Some("thinking") {
        for block in content_blocks {
            if block.block_type.as_deref() == Some("thinking") {
                if let Some(thinking) = &block.thinking {
                    return AgentEvent::Thinking {
                        content: truncate(thinking, 1000),
                    };
                }
            }
        }
    }

    // Prioritize tool_use blocks over text blocks.
    for block in content_blocks {
        if block.block_type.as_deref() != Some("tool_use") {
            continue;
        }
        let tool_name = block.name.as_deref().unwrap_or("");
        let input = block.input.as_ref();

        match tool_name {
                "Read" => {
                    let path = input
                        .and_then(|v| v.get("file_path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("<unknown>")
                        .to_string();
                    return AgentEvent::FileRead {
                        path,
                        content_preview: None,
                    };
                }
                "Edit" | "Write" => {
                    let path = input
                        .and_then(|v| v.get("file_path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("<unknown>")
                        .to_string();
                    let diff_snippet = input
                        .and_then(|v| v.get("new_string"))
                        .and_then(|v| v.as_str())
                        .map(|s| truncate(s, 200));
                    return AgentEvent::FileEdit { path, diff_snippet };
                }
                "Bash" => {
                    let command = input
                        .and_then(|v| v.get("command"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("<unknown>")
                        .to_string();
                    return AgentEvent::BashCommand {
                        command: truncate(&command, 200),
                        output_preview: None,
                    };
                }
                "Grep" | "Glob" => {
                    let pattern = input
                        .and_then(|v| v.get("pattern"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("<unknown>")
                        .to_string();
                    return AgentEvent::CodeSearch {
                        pattern,
                        match_count: None,
                    };
                }
                "Task" => {
                    let prompt = input
                        .and_then(|v| v.get("prompt"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("subtask");
                    return AgentEvent::Thinking {
                        content: format!("Spawning subtask: {}", truncate(prompt, 200)),
                    };
                }
                "NotebookEdit" => {
                    let path = input
                        .and_then(|v| v.get("notebook_path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("<unknown>")
                        .to_string();
                    return AgentEvent::FileEdit { path, diff_snippet: None };
                }
                _ => {
                    // Show any unknown tool as a thinking event rather than raw "unknown".
                    return AgentEvent::Thinking {
                        content: format!("Using tool: {}", tool_name),
                    };
                }
            }
    }

    // No tool_use found — look for text blocks.
    for block in content_blocks {
        if block.block_type.as_deref() == Some("text") {
            if let Some(text) = &block.text {
                if !text.trim().is_empty() {
                    return AgentEvent::Thinking {
                        content: truncate(text, 1000),
                    };
                }
            }
        }
    }

    // Truly unrecognized — show block types for debugging.
    let block_types: Vec<String> = content_blocks
        .iter()
        .map(|b| b.block_type.as_deref().unwrap_or("?").to_string())
        .collect();
    AgentEvent::Unknown {
        raw: format!("assistant: [{}]", block_types.join(", ")),
    }
}

/// Parse a "user" type message (tool execution result) into an AgentEvent.
/// Shows a brief summary of tool output in the activity stream.
fn parse_tool_result_event(msg: &JsonlMessage) -> AgentEvent {
    // First try the top-level tool_use_result (has stdout/stderr).
    if let Some(ref result) = msg.tool_use_result {
        let stdout = result.stdout.as_deref().unwrap_or("");
        let stderr = result.stderr.as_deref().unwrap_or("");
        let output = if !stdout.is_empty() { stdout } else { stderr };
        if !output.is_empty() {
            let first_line = output.lines().next().unwrap_or("");
            let line_count = output.lines().count();
            let preview = if line_count > 1 {
                format!("{} (+{} more lines)", truncate(first_line, 100), line_count - 1)
            } else {
                truncate(first_line, 200)
            };
            return AgentEvent::Thinking {
                content: format!("  → {}", preview),
            };
        }
    }

    // Try message.content for tool_result blocks.
    if let Some(ref message) = msg.message {
        if let Some(ref blocks) = message.content {
            for block in blocks {
                if block.block_type.as_deref() == Some("tool_result") {
                    if let Some(ref content) = block.content {
                        let text = match content {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        if !text.is_empty() {
                            let first_line = text.lines().next().unwrap_or("");
                            let line_count = text.lines().count();
                            let preview = if line_count > 1 {
                                format!("{} (+{} more lines)", truncate(first_line, 100), line_count - 1)
                            } else {
                                truncate(first_line, 200)
                            };
                            return AgentEvent::Thinking {
                                content: format!("  → {}", preview),
                            };
                        }
                    }
                }
            }
        }
    }

    // No useful content — emit a minimal marker.
    AgentEvent::Thinking {
        content: "  → (done)".to_string(),
    }
}

/// Parse a "result" type message into an AgentEvent.
fn parse_result_event(msg: &JsonlMessage) -> AgentEvent {
    let result_text = msg.result.as_deref().unwrap_or("");

    // Prefer duration_ms (Claude Code's actual field), fall back to duration_seconds.
    let duration = msg
        .duration_ms
        .map(|ms| Duration::from_millis(ms as u64))
        .or_else(|| msg.duration_seconds.map(|s| Duration::from_secs_f64(s)))
        .unwrap_or_default();

    // Only treat as error if the is_error field is explicitly true.
    // Do NOT use heuristic text matching — the result summary often
    // contains words like "error" in harmless contexts (e.g. "no errors").
    if msg.is_error.unwrap_or(false) {
        AgentEvent::Error {
            message: truncate(result_text, 500),
        }
    } else {
        AgentEvent::Complete {
            summary: truncate(result_text, 500),
            duration,
        }
    }
}

/// Parse an OpenCode "tool_use" event into an AgentEvent.
/// OpenCode bundles tool input and output in the same event via `part.state`.
/// When `state.status == "completed"`, the tool output is in `state.output`.
fn parse_opencode_tool_call(msg: &JsonlMessage) -> AgentEvent {
    let part = match &msg.part {
        Some(p) => p,
        None => {
            return AgentEvent::Unknown {
                raw: "tool_use with no part".to_string(),
            }
        }
    };

    let tool_name = part.tool.as_deref().unwrap_or("");
    let args = part.state.as_ref().and_then(|s| s.input.as_ref());
    let is_completed = part
        .state
        .as_ref()
        .and_then(|s| s.status.as_deref())
        .map(|s| s == "completed")
        .unwrap_or(false);

    // If the tool has completed and has output, show the output as a result.
    if is_completed {
        if let Some(output) = part.state.as_ref().and_then(|s| s.output.as_deref()) {
            if !output.is_empty() {
                let first_line = output.lines().next().unwrap_or("");
                let line_count = output.lines().count();
                let preview = if line_count > 1 {
                    format!(
                        "{} (+{} more lines)",
                        truncate(first_line, 100),
                        line_count - 1
                    )
                } else {
                    truncate(first_line, 200)
                };
                return AgentEvent::Thinking {
                    content: format!("  → {}", preview),
                };
            }
        }
        // Completed with no output — show a minimal marker.
        return AgentEvent::Thinking {
            content: format!("  → {} (done)", tool_name),
        };
    }

    // Tool is still running or just started — show what it's doing.
    match tool_name {
        "read" | "Read" => {
            let path = args
                .and_then(|v| v.get("path").or(v.get("file_path")))
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>")
                .to_string();
            AgentEvent::FileRead {
                path,
                content_preview: None,
            }
        }
        "edit" | "Edit" | "write" | "Write" | "patch" => {
            let path = args
                .and_then(|v| v.get("path").or(v.get("file_path")))
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>")
                .to_string();
            AgentEvent::FileEdit {
                path,
                diff_snippet: None,
            }
        }
        "bash" | "Bash" | "shell" => {
            let command = args
                .and_then(|v| v.get("command").or(v.get("cmd")))
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>")
                .to_string();
            AgentEvent::BashCommand {
                command: truncate(&command, 200),
                output_preview: None,
            }
        }
        "grep" | "Grep" | "glob" | "Glob" | "search" => {
            let pattern = args
                .and_then(|v| v.get("pattern").or(v.get("query")))
                .and_then(|v| v.as_str())
                .unwrap_or("<unknown>")
                .to_string();
            AgentEvent::CodeSearch {
                pattern,
                match_count: None,
            }
        }
        _ => AgentEvent::Thinking {
            content: format!("Using tool: {}", tool_name),
        },
    }
}

/// Truncate a string to a maximum length, adding "..." if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_thinking_event() {
        let line = r#"{"type":"assistant","subtype":"thinking","message":{"content":[{"type":"thinking","thinking":"Let me analyze this code."}]}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Thinking { content } => {
                assert!(content.contains("analyze"));
            }
            other => panic!("Expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_file_read_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"src/main.rs"}}]}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::FileRead { path, .. } => {
                assert_eq!(path, "src/main.rs");
            }
            other => panic!("Expected FileRead, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_file_edit_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/lib.rs","new_string":"fn hello() {}"}}]}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::FileEdit { path, diff_snippet } => {
                assert_eq!(path, "src/lib.rs");
                assert!(diff_snippet.unwrap().contains("hello"));
            }
            other => panic!("Expected FileEdit, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_bash_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo build"}}]}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::BashCommand { command, .. } => {
                assert_eq!(command, "cargo build");
            }
            other => panic!("Expected BashCommand, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_code_search_event() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Grep","input":{"pattern":"fn main"}}]}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::CodeSearch { pattern, .. } => {
                assert_eq!(pattern, "fn main");
            }
            other => panic!("Expected CodeSearch, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_result_success() {
        let line = r#"{"type":"result","subtype":"success","result":"Successfully refactored the module.","duration_ms":45200}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Complete { summary, duration } => {
                assert!(summary.contains("Successfully"));
                assert!(duration.as_secs() >= 45);
            }
            other => panic!("Expected Complete, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_result_success_duration_seconds_fallback() {
        let line = r#"{"type":"result","result":"Done.","duration_seconds":10.5}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Complete { duration, .. } => {
                assert!(duration.as_secs() >= 10);
            }
            other => panic!("Expected Complete, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_result_error() {
        let line = r#"{"type":"result","result":"Credit balance is too low","is_error":true,"duration_ms":797}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Error { message } => {
                assert!(message.contains("Credit balance"));
            }
            other => panic!("Expected Error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_result_without_is_error_flag_is_complete() {
        // Without is_error=true, even text containing "error" should be Complete.
        let line = r#"{"type":"result","result":"No errors found in the build","duration_ms":10000}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Complete { summary, .. } => {
                assert!(summary.contains("No errors"));
            }
            other => panic!("Expected Complete, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_system_type() {
        // Claude Code system messages have no "message" field, just type + metadata.
        let line = r#"{"type":"system","subtype":"init"}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Thinking { content } => {
                assert!(content.contains("initializing"));
            }
            other => panic!("Expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        let line = "this is not json";
        match parse_jsonl_event(line) {
            AgentEvent::Unknown { .. } => {}
            other => panic!("Expected Unknown, got {:?}", other),
        }
    }

    #[test]
    fn test_backend_from_str() {
        assert!(matches!(
            CodingAgentBackend::from_str("claude-code"),
            CodingAgentBackend::ClaudeCode
        ));
        assert!(matches!(
            CodingAgentBackend::from_str("opencode"),
            CodingAgentBackend::OpenCode
        ));
        assert!(matches!(
            CodingAgentBackend::from_str("anything-else"),
            CodingAgentBackend::ClaudeCode
        ));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    // ── OpenCode event parsing tests ──

    #[test]
    fn test_parse_opencode_step_start() {
        let line = r#"{"type":"step_start","part":{"type":"step_start"}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Thinking { content } => {
                assert!(content.contains("starting step"));
            }
            other => panic!("Expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_opencode_text() {
        let line = r#"{"type":"text","part":{"type":"text","text":"I'll read the file now."}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Thinking { content } => {
                assert!(content.contains("read the file"));
            }
            other => panic!("Expected Thinking, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_opencode_tool_use_pending() {
        let line = r#"{"type":"tool_use","part":{"type":"tool_use","tool":"bash","state":{"status":"pending","input":{"command":"cargo build"}}}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::BashCommand { command, .. } => {
                assert_eq!(command, "cargo build");
            }
            other => panic!("Expected BashCommand, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_opencode_tool_use_completed() {
        let line = r#"{"type":"tool_use","part":{"type":"tool_use","tool":"bash","state":{"status":"completed","input":{"command":"cargo build"},"output":"Compiling levsha v0.1.0\nFinished"}}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Thinking { content } => {
                assert!(content.contains("Compiling"));
                assert!(content.contains("+1 more lines"));
            }
            other => panic!("Expected Thinking (result), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_opencode_tool_use_read() {
        let line = r#"{"type":"tool_use","part":{"type":"tool_use","tool":"read","state":{"status":"pending","input":{"path":"src/main.rs"}}}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::FileRead { path, .. } => {
                assert_eq!(path, "src/main.rs");
            }
            other => panic!("Expected FileRead, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_opencode_step_finish() {
        let line = r#"{"type":"step_finish","part":{"type":"step_finish","reason":"done","cost":0.0042,"tokens":{"total":1234}}}"#;
        match parse_jsonl_event(line) {
            AgentEvent::Complete { summary, .. } => {
                assert!(summary.contains("done"));
                assert!(summary.contains("0.0042"));
            }
            other => panic!("Expected Complete, got {:?}", other),
        }
    }
}
