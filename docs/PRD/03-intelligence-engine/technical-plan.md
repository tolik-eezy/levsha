# Intelligence Engine (L2) — Technical Plan

**Module:** 03-intelligence-engine
**Layer:** L2 — Intelligence Engine
**Phase:** 1 (MVP)

---

## 1. Crate Structure

The engine lives in `/engine/` at the repository root. It is a Rust binary crate with internal modules:

```
engine/
  Cargo.toml
  src/
    main.rs              # Entry point, config loading, IPC server
    config.rs            # TOML config parsing
    api/
      mod.rs             # Anthropic API client
      types.rs           # Request/response structs
      stream.rs          # SSE stream parser
      error.rs           # API error types and retry logic
    context/
      mod.rs             # Context manager
      history.rs         # Conversation history (in-memory + SQLite)
      prompt.rs          # System prompt composer
      token.rs           # Token counting / estimation
    tools/
      mod.rs             # Tool registry and dispatch
      executor.rs        # Command execution (std::process::Command)
      guard.rs           # Destructive command classifier
    ipc/
      mod.rs             # Unix socket IPC server
      protocol.rs        # Message types (JSON serialization)
    skills/
      mod.rs             # Skill loader (reads manifests + prompts + tool defs)
```

### Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
reqwest = { version = "0.12", features = ["stream"] }
rusqlite = { version = "0.31", features = ["bundled"] }
regex = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

No Anthropic SDK — we use `reqwest` directly for HTTP calls.

---

## 2. Anthropic API Client

### HTTP Implementation

The API client sends POST requests to `https://api.anthropic.com/v1/messages` with streaming enabled.

```rust
// Simplified API call structure
pub async fn send_message(&self, request: &MessageRequest) -> Result<EventStream> {
    let response = self.client
        .post(&format!("{}/v1/messages", self.config.base_url))
        .header("x-api-key", &self.config.api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(request)
        .send()
        .await?;

    match response.status() {
        StatusCode::OK => Ok(EventStream::new(response.bytes_stream())),
        status => Err(classify_error(status, response).await),
    }
}
```

### Request Structure

```rust
#[derive(Serialize)]
pub struct MessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,          // Composed system prompt
    pub messages: Vec<Message>,  // Conversation history
    pub tools: Vec<Tool>,        // Merged tool definitions
    pub stream: bool,            // Always true in MVP
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: String,            // "user" or "assistant"
    pub content: Vec<ContentBlock>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}
```

### Error Classification and Retry

```rust
pub enum ApiError {
    AuthFailure,           // 401 — no retry
    RateLimited(Duration), // 429 — wait, then retry
    ServerError,           // 500/502/503 — retry with backoff
    Timeout,               // request timeout — retry
    NetworkError,          // connection failure — retry
    MalformedResponse,     // parse failure — retry once
}

impl ApiClient {
    async fn with_retry<F, T>(&self, f: F) -> Result<T>
    where F: Fn() -> Future<Output = Result<T>>
    {
        let delays = [Duration::ZERO, Duration::from_secs(1), Duration::from_secs(2)];
        for (attempt, delay) in delays.iter().enumerate() {
            tokio::time::sleep(*delay).await;
            match f().await {
                Ok(val) => return Ok(val),
                Err(e) if e.is_retryable() && attempt < delays.len() - 1 => continue,
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }
}
```

---

## 3. SSE Stream Parser

The Anthropic API returns Server-Sent Events. The parser is a state machine over the byte stream:

```rust
pub struct SseParser {
    buffer: String,
}

pub enum SseEvent {
    MessageStart { message_id: String },
    ContentBlockStart { index: usize, block_type: String },
    TextDelta { index: usize, text: String },
    InputJsonDelta { index: usize, partial_json: String },
    ContentBlockStop { index: usize },
    MessageDelta { stop_reason: Option<String> },
    MessageStop,
    Error { error_type: String, message: String },
}

impl SseParser {
    /// Feed raw bytes from the HTTP stream. Returns parsed events.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));
        let mut events = Vec::new();
        // Split on double newline, parse each "event:" / "data:" block
        while let Some(block) = self.take_complete_block() {
            if let Some(event) = self.parse_block(&block) {
                events.push(event);
            }
        }
        events
    }
}
```

The parser handles:
- Partial chunks (SSE data split across TCP segments).
- Multiple events in a single chunk.
- `event:` and `data:` field extraction per the SSE specification.
- JSON deserialization of the `data:` payload based on the `type` field.

---

## 4. Tool Calling Implementation

### Tool Registry

At startup, the engine loads tool definitions from all skill manifests:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub handler: ToolHandler,
}

pub enum ToolHandler {
    ShellCommand { template: Option<String> },  // Most tools
    Internal(fn(serde_json::Value) -> Result<String>),
}
```

### Execution Flow

```rust
pub async fn execute_tool(&self, call: &ToolCall) -> ToolResult {
    let definition = self.registry.get(&call.name)
        .ok_or_else(|| format!("Unknown tool: {}", call.name))?;

    let command = self.resolve_command(definition, &call.input);

    // Destructive guard check
    if self.guard.is_destructive(&command) {
        let approved = self.request_confirmation(&command).await;
        if !approved {
            return ToolResult {
                tool_use_id: call.id.clone(),
                content: "User cancelled this command.".into(),
                is_error: false,
            };
        }
    }

    // Execute
    let output = self.executor.run(&command, self.config.command_timeout).await;
    ToolResult {
        tool_use_id: call.id.clone(),
        content: format_output(&output),
        is_error: !output.status.success(),
    }
}
```

### Command Execution

```rust
pub struct CommandExecutor;

impl CommandExecutor {
    pub async fn run(&self, command: &str, timeout: Duration) -> CommandOutput {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn")
                .wait_with_output()
        ).await;

        match result {
            Ok(Ok(output)) => CommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).into(),
                stderr: String::from_utf8_lossy(&output.stderr).into(),
                status: output.status,
                timed_out: false,
            },
            Ok(Err(e)) => CommandOutput::error(format!("Execution failed: {}", e)),
            Err(_) => CommandOutput::error("Command timed out"),
        }
    }
}
```

---

## 5. Destructive Command Patterns

The guard uses compiled regex patterns loaded at startup:

```rust
pub struct DestructiveGuard {
    patterns: Vec<Regex>,
}

const DESTRUCTIVE_PATTERNS: &[&str] = &[
    // Recursive / forced file deletion
    r"rm\s+(-[a-zA-Z]*r|-[a-zA-Z]*f[a-zA-Z]*\s+/|--recursive)",
    // Filesystem formatting
    r"mkfs\.",
    r"wipefs\s",
    // Raw disk writes
    r"dd\s+.*if=",
    // Partition table modification
    r"fdisk\s",
    r"parted\s",
    r"gdisk\s",
    // System shutdown / reboot
    r"(reboot|shutdown|poweroff|halt)\b",
    r"init\s+[06]",
    r"systemctl\s+(stop|disable|mask)\s+(sshd|systemd-|levsha-|NetworkManager)",
    // Bulk package removal
    r"dnf\s+(-y\s+)?remove\s",
    // Dangerous redirects
    r">\s*/dev/sd[a-z]",
    r">\s*/dev/nvme",
];

impl DestructiveGuard {
    pub fn is_destructive(&self, command: &str) -> bool {
        self.patterns.iter().any(|p| p.is_match(command))
    }
}
```

The pattern list is intentionally conservative (false positives are preferred over false negatives). Users can always confirm and proceed. Patterns are maintained in the engine config or a separate file for easy updates.

---

## 6. System Prompt Template

Located at `/usr/share/levsha/prompts/system.md`:

```markdown
You are Levsha OS, a minimal Linux operating system where the chat is the
entire user interface. You are running on Fedora Linux.

System: {{os_version}} | Host: {{hostname}} | Time: {{datetime}}

## Behavior

- You ARE the operating system. Speak as the system, not an assistant.
- Execute tasks using the provided tools. Do not guess at output.
- Be concise. Use tables and code blocks for structured output.
- When a command fails, report the error clearly and suggest fixes.
- Never refuse to execute a command — but destructive commands will
  require user confirmation automatically.

## Available Skills

{{skills_summary}}
```

Template variables are substituted at each API request for `{{datetime}}`, and at startup for the rest.

---

## 7. Context Window Token Counting

```rust
pub struct TokenCounter {
    chars_per_token: usize,  // default: 4
}

impl TokenCounter {
    pub fn estimate(&self, text: &str) -> usize {
        text.len().div_ceil(self.chars_per_token)
    }

    pub fn estimate_message(&self, msg: &Message) -> usize {
        let content_tokens: usize = msg.content.iter().map(|block| {
            match block {
                ContentBlock::Text { text } => self.estimate(text),
                ContentBlock::ToolUse { input, .. } => self.estimate(&input.to_string()),
                ContentBlock::ToolResult { content, .. } => self.estimate(content),
            }
        }).sum();
        content_tokens + 4 // overhead per message (role, formatting)
    }
}
```

### Truncation Implementation

```rust
pub fn truncate_history(&mut self, budget: usize) -> bool {
    let mut total = self.system_tokens + self.skill_tokens + self.response_reserve;

    for msg in self.history.iter().rev() {
        total += self.counter.estimate_message(msg);
    }

    let mut truncated = false;
    while total > self.max_tokens && self.history.len() > 1 {
        let removed = self.history.remove(0);
        total -= self.counter.estimate_message(&removed);
        truncated = true;
    }
    truncated
}
```

---

## 8. Configuration File Format

`/etc/levsha/config.toml`:

```rust
#[derive(Deserialize)]
pub struct EngineConfig {
    pub api: ApiConfig,
    pub context: ContextConfig,
    pub execution: ExecutionConfig,
    pub skills: SkillsConfig,
}

#[derive(Deserialize)]
pub struct ApiConfig {
    pub key: String,
    pub model: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

#[derive(Deserialize)]
pub struct ContextConfig {
    pub max_tokens: usize,
    pub response_reserve: usize,
    pub chars_per_token: usize,
}

#[derive(Deserialize)]
pub struct ExecutionConfig {
    pub command_timeout_seconds: u64,
    pub max_tool_iterations: u32,
}

#[derive(Deserialize)]
pub struct SkillsConfig {
    pub path: String,
}
```

---

## 9. Integration Points

### Chat Shell Integration

The engine exposes a Unix domain socket at `/run/levsha/engine.sock`. The Chat Shell connects on startup. Protocol is newline-delimited JSON.

**Startup sequence:**

1. systemd starts `levsha-engine.service` (Type=notify).
2. Engine loads config, connects to SQLite, loads skills, compiles destructive patterns.
3. Engine opens Unix socket at `/run/levsha/engine.sock`.
4. Engine sends sd_notify READY=1.
5. systemd starts `levsha-chat.service` (After=levsha-engine.service).
6. Chat Shell connects to the engine socket.
7. Engine sends welcome message or restores last session.

### Skill System Integration

Skills are directories under `/usr/share/levsha/skills/`:

```
/usr/share/levsha/skills/
  pkg-manager/
    skill.yaml          # Manifest
    prompt.md           # Prompt fragment
    tools/
      pkg_install.json  # Tool definition
      pkg_remove.json
      pkg_search.json
      pkg_update.json
      pkg_list.json
  sysinfo/
    skill.yaml
    prompt.md
    tools/
      sys_disk.json
      sys_memory.json
      sys_cpu.json
      sys_uptime.json
      sys_network.json
      sys_processes.json
```

The engine loads all skills at startup: parses YAML manifests, reads prompt files, and deserializes tool JSON schemas into the tool registry.

---

## 10. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `api::stream` | SSE parsing with partial chunks, multi-event chunks, error events |
| `api::error` | Error classification from HTTP status codes |
| `context::token` | Token estimation accuracy against known strings |
| `context::history` | Truncation correctness, boundary conditions |
| `tools::guard` | Destructive pattern matching (true positives, true negatives) |
| `ipc::protocol` | JSON serialization/deserialization of all message types |
| `skills::mod` | Skill manifest parsing, tool definition loading |

### Integration Tests with Mock API

A mock HTTP server (`wiremock` or custom) simulates the Anthropic API:

```rust
#[tokio::test]
async fn test_simple_text_response() {
    let mock = MockAnthropicServer::start().await;
    mock.respond_with_text("Hello from Levsha OS").await;

    let engine = Engine::new(config_with_mock(&mock)).await;
    let response = engine.handle_message("hi").await;

    assert_eq!(response.text(), "Hello from Levsha OS");
}

#[tokio::test]
async fn test_tool_call_and_execution() {
    let mock = MockAnthropicServer::start().await;
    mock.respond_with_tool_call("run_command", json!({"command": "uptime"})).await;
    mock.then_respond_with_text("System has been up for 2 hours").await;

    let engine = Engine::new(config_with_mock(&mock)).await;
    let response = engine.handle_message("how long has the system been running?").await;

    assert!(response.text().contains("2 hours"));
}
```

### Tool Execution Tests

Tool execution tests use a controlled environment:

- Run commands against a mock filesystem (tmpdir).
- Verify destructive guard blocks `rm -rf /`, `mkfs.ext4`, `dd if=/dev/zero`.
- Verify safe commands pass through: `ls`, `df -h`, `free -m`.
- Verify timeout handling kills long-running processes.

### End-to-End Test

A full integration test boots the engine with a mock API, connects a mock Chat Shell client via Unix socket, and verifies the complete flow: user message -> API call -> streaming response -> tool call -> confirmation -> execution -> final response.

```rust
#[tokio::test]
async fn test_full_conversation_flow() {
    let mock_api = MockAnthropicServer::start().await;
    let engine = spawn_engine(config_with_mock(&mock_api)).await;
    let client = connect_ipc("/tmp/test-engine.sock").await;

    // Send user message
    client.send(json!({"type": "user_message", "content": "what's my disk usage?"})).await;

    // Expect streaming chunks
    let chunks = client.collect_until_stream_end().await;
    assert!(!chunks.is_empty());

    // Verify tool was called
    let api_requests = mock_api.recorded_requests().await;
    assert!(api_requests.last().unwrap().tools.iter().any(|t| t.name == "sys_disk"));
}
```

### Cross-Module Integration Tests

These tests verify the Engine is correctly wired to adjacent modules. See `00-system-architecture/integration-checks.md` for full details.

| Check | Seam | What it verifies |
|-------|------|------------------|
| IC-01 | L2 <- L3 | Engine accepts Chat Shell socket connection and logs "client connected" |
| IC-02 | L2 <- L3 | Engine receives and parses `user_message` JSON from Chat Shell |
| IC-03 | L2 -> L3 | Engine forwards SSE tokens as IPC `token` messages in correct order |
| IC-05 | L2 -> L3 | Engine sends `error` IPC messages that Chat Shell can parse and render |
| IC-10 | L2 -> API | Engine constructs valid Anthropic request with system prompt + skills + history |
| IC-11 | API -> L2 -> L3 | SSE stream from API converts to IPC tokens within 16ms per token |
| IC-12 | API -> L2 -> L1 | Tool call from API routed to correct skill, command executed, result sent back |
| IC-13 | L2 <-> API | Multi-turn tool calling: multiple tool_use blocks handled sequentially |
| IC-20 | L2 -> L1 | Engine spawns commands on base system via `tokio::process::Command` |
| IC-21 | L3 -> L2 -> API -> L2 -> L1 -> L2 -> API -> L2 -> L3 | Full package install chain from user input to installed binary |
| IC-22 | L2 -> L3 -> L2 | Destructive guard sends confirmation, receives approval/denial, acts accordingly |
| IC-30 | Skills -> L2 -> API | All skill prompts in system message, all tool definitions in API request |
| IC-40 | L2 -> SQLite | Engine writes messages during conversation, readable by Chat Shell |
| IC-43 | L2 <- SQLite | Engine loads recent history and constructs valid context window |
| IC-53 | L2 + systemd | Engine crash triggers restart, re-initializes socket/DB/skills, Chat Shell reconnects |
