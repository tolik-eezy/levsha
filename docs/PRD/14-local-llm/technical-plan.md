# 14 — Local LLM Support: Technical Plan

**Module:** Local LLM + Intelligent Backend Routing
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

```
engine/
  src/
    api/
      mod.rs              # Modified: backend abstraction
      anthropic.rs        # Renamed from existing API client (cloud)
      openai_compat.rs    # New: OpenAI-compatible client for llama-server
      router.rs           # New: intelligent backend routing
    local/
      mod.rs              # Local model management
      server.rs           # llama-server lifecycle (start/stop/health)
      models.rs           # Model download, list, delete
    config.rs             # Modified: local_model and routing config sections
```

### Dependencies

```toml
# Added to engine/Cargo.toml
# No new crates needed for Phase 2 (TCP) — reqwest already handles OpenAI-compatible endpoints
# Future (Unix socket transport): add hyperlocal for HTTP-over-UDS support
# hyperlocal = "0.9"
```

---

## 2. Backend Abstraction

```rust
/// Unified interface for both cloud and local backends
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn send_message(&self, request: &MessageRequest) -> Result<EventStream>;
    fn name(&self) -> &str;
    fn is_available(&self) -> bool;
}

pub struct AnthropicBackend {
    client: reqwest::Client,
    config: ApiConfig,
}

pub struct LocalBackend {
    client: reqwest::Client,
    base_url: String,       // http://localhost:8080 or unix socket path
    model_name: String,
}

/// Transport configuration for llama-server connection.
/// Phase 2 uses TCP (localhost:8080).
/// Future: Unix domain socket eliminates loopback overhead
/// while keeping the same OpenAI-compatible HTTP protocol.
pub enum LocalTransport {
    Tcp { port: u16 },                        // http://localhost:{port}
    UnixSocket { path: PathBuf },             // /run/levsha/llama.sock
}

#[async_trait]
impl LlmBackend for AnthropicBackend {
    async fn send_message(&self, request: &MessageRequest) -> Result<EventStream> {
        // Existing Anthropic API implementation
        let response = self.client
            .post(format!("{}/v1/messages", self.config.base_url))
            .header("x-api-key", &self.config.key)
            .header("anthropic-version", "2023-06-01")
            .json(&request.to_anthropic_format())
            .send()
            .await?;
        Ok(EventStream::new(response.bytes_stream()))
    }

    fn name(&self) -> &str { &self.config.model }
    fn is_available(&self) -> bool { !self.config.key.is_empty() }
}

#[async_trait]
impl LlmBackend for LocalBackend {
    async fn send_message(&self, request: &MessageRequest) -> Result<EventStream> {
        // OpenAI-compatible API for llama-server
        let response = self.client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .json(&request.to_openai_format(&self.model_name))
            .send()
            .await?;
        Ok(EventStream::new_openai(response.bytes_stream()))
    }

    fn name(&self) -> &str { &self.model_name }

    fn is_available(&self) -> bool {
        // Quick health check
        reqwest::blocking::get(format!("{}/health", self.base_url))
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
```

---

## 3. OpenAI-Compatible SSE Parser

llama-server uses OpenAI's streaming format, which differs from Anthropic's.

```rust
pub struct OpenAiSseParser {
    buffer: String,
}

#[derive(Deserialize)]
struct OpenAiChunk {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

impl OpenAiSseParser {
    /// Convert OpenAI SSE events to our internal SseEvent format
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));
        let mut events = Vec::new();

        while let Some(block) = self.take_complete_block() {
            if block.trim() == "[DONE]" {
                events.push(SseEvent::MessageStop);
                continue;
            }
            if let Ok(chunk) = serde_json::from_str::<OpenAiChunk>(&block) {
                for choice in &chunk.choices {
                    if let Some(content) = &choice.delta.content {
                        events.push(SseEvent::TextDelta {
                            index: 0,
                            text: content.clone(),
                        });
                    }
                    if choice.finish_reason.is_some() {
                        events.push(SseEvent::MessageStop);
                    }
                }
            }
        }
        events
    }
}
```

---

## 4. Backend Router

```rust
pub struct BackendRouter {
    cloud: Arc<AnthropicBackend>,
    local: Arc<LocalBackend>,
    mode: RoutingMode,
}

#[derive(Clone, Copy, PartialEq)]
pub enum RoutingMode {
    Auto,
    CloudOnly,
    LocalOnly,
}

pub enum RoutingDecision {
    Cloud(String),     // reason
    Local(String),     // reason
}

impl BackendRouter {
    pub fn route(&self, context: &RoutingContext) -> RoutingDecision {
        match self.mode {
            RoutingMode::CloudOnly => RoutingDecision::Cloud("user preference".into()),
            RoutingMode::LocalOnly => RoutingDecision::Local("user preference".into()),
            RoutingMode::Auto => self.auto_route(context),
        }
    }

    fn auto_route(&self, ctx: &RoutingContext) -> RoutingDecision {
        // Rule 1: Self-improvement always uses cloud
        if ctx.is_self_improvement {
            return RoutingDecision::Cloud("self-improvement requires cloud".into());
        }

        // Rule 2: If cloud is unavailable, use local
        if !self.cloud.is_available() {
            return RoutingDecision::Local("cloud unavailable (offline)".into());
        }

        // Rule 3: If local is unavailable, use cloud
        if !self.local.is_available() {
            return RoutingDecision::Cloud("local model not available".into());
        }

        // Rule 4: Short simple messages → local
        if ctx.estimated_tokens < 50 && !ctx.has_complex_tools {
            return RoutingDecision::Local("simple query".into());
        }

        // Rule 5: Multi-step tool use → cloud
        if ctx.expected_tool_chains > 2 {
            return RoutingDecision::Cloud("complex multi-step task".into());
        }

        // Rule 6: Known simple tool patterns → local
        if ctx.tools_requested.iter().all(|t| is_simple_tool(t)) {
            return RoutingDecision::Local("simple tools only".into());
        }

        // Default: cloud for complex queries
        RoutingDecision::Cloud("complex query".into())
    }

    pub fn set_mode(&mut self, mode: RoutingMode) {
        self.mode = mode;
    }

    pub async fn send(
        &self,
        request: &MessageRequest,
        context: &RoutingContext,
    ) -> Result<(EventStream, RoutingDecision)> {
        let decision = self.route(context);
        let backend: &dyn LlmBackend = match &decision {
            RoutingDecision::Cloud(_) => self.cloud.as_ref(),
            RoutingDecision::Local(_) => self.local.as_ref(),
        };

        match backend.send_message(request).await {
            Ok(stream) => Ok((stream, decision)),
            Err(e) => {
                // Fallback: if cloud fails, try local (and vice versa)
                let fallback: &dyn LlmBackend = match &decision {
                    RoutingDecision::Cloud(_) if self.local.is_available() => {
                        self.local.as_ref()
                    }
                    RoutingDecision::Local(_) if self.cloud.is_available() => {
                        self.cloud.as_ref()
                    }
                    _ => return Err(e),
                };
                let stream = fallback.send_message(request).await?;
                let fallback_decision = match &decision {
                    RoutingDecision::Cloud(_) => RoutingDecision::Local("cloud failed, fallback".into()),
                    RoutingDecision::Local(_) => RoutingDecision::Cloud("local failed, fallback".into()),
                };
                Ok((stream, fallback_decision))
            }
        }
    }
}

fn is_simple_tool(tool_name: &str) -> bool {
    matches!(tool_name,
        "sys_disk" | "sys_memory" | "sys_cpu" | "sys_uptime" |
        "sys_network" | "sys_processes" | "pkg_search" | "pkg_list" |
        "fs_list" | "fs_read" | "fs_head" | "fs_tail" | "fs_size"
    )
}
```

---

## 5. llama-server Lifecycle Management

```rust
pub struct LlamaServer {
    binary_path: PathBuf,
    model_path: PathBuf,
    transport: LocalTransport,
    context_size: usize,
    process: Option<tokio::process::Child>,
}

impl LlamaServer {
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running().await {
            return Ok(());
        }

        let mut cmd = tokio::process::Command::new(&self.binary_path);
        cmd.arg("--model").arg(&self.model_path)
            .arg("--ctx-size").arg(self.context_size.to_string())
            .arg("--threads").arg(num_cpus::get().to_string());

        // Phase 2: TCP on localhost. Future: Unix domain socket.
        match &self.transport {
            LocalTransport::Tcp { port } => {
                cmd.arg("--port").arg(port.to_string());
            }
            LocalTransport::UnixSocket { path } => {
                cmd.arg("--host").arg(path.as_os_str());
            }
        }

        let child = cmd
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        self.process = Some(child);

        // Wait for server to be ready
        self.wait_for_ready(Duration::from_secs(30)).await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut process) = self.process.take() {
            process.kill().await?;
        }
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        let url = match &self.transport {
            LocalTransport::Tcp { port } => format!("http://localhost:{}/health", port),
            LocalTransport::UnixSocket { path } => {
                // Use hyperlocal or reqwest unix socket support
                format!("http://unix:{}:/health", path.display())
            }
        };
        reqwest::get(&url)
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    async fn wait_for_ready(&self, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > timeout {
                return Err(LocalError::ServerStartTimeout);
            }
            if self.is_running().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn switch_model(&mut self, model_path: &Path) -> Result<()> {
        self.stop().await?;
        self.model_path = model_path.to_path_buf();
        self.start().await
    }
}
```

---

## 6. Model Management

```rust
pub struct ModelManager {
    models_dir: PathBuf,   // /var/lib/levsha/models/
}

pub struct ModelInfo {
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub is_active: bool,
}

const AVAILABLE_MODELS: &[(&str, &str)] = &[
    ("llama-3-8b-q4", "https://huggingface.co/..."),
    ("llama-3-70b-q4", "https://huggingface.co/..."),
    ("mistral-7b-q4", "https://huggingface.co/..."),
    ("codellama-13b-q4", "https://huggingface.co/..."),
    ("phi-3-mini-q4", "https://huggingface.co/..."),
];

impl ModelManager {
    pub fn list_downloaded(&self) -> Result<Vec<ModelInfo>> {
        let mut models = Vec::new();
        for entry in std::fs::read_dir(&self.models_dir)? {
            let entry = entry?;
            if entry.path().extension().map_or(false, |e| e == "gguf") {
                let meta = entry.metadata()?;
                models.push(ModelInfo {
                    name: entry.path().file_stem().unwrap().to_string_lossy().into(),
                    filename: entry.file_name().to_string_lossy().into(),
                    size_bytes: meta.len(),
                    is_active: false, // set externally
                });
            }
        }
        Ok(models)
    }

    pub async fn download(
        &self,
        model_name: &str,
        progress_tx: mpsc::Sender<DownloadProgress>,
    ) -> Result<PathBuf> {
        let url = AVAILABLE_MODELS.iter()
            .find(|(name, _)| *name == model_name)
            .map(|(_, url)| *url)
            .ok_or(ModelError::NotFound(model_name.to_string()))?;

        let dest = self.models_dir.join(format!("{}.gguf", model_name));
        let response = reqwest::get(url).await?;
        let total = response.content_length().unwrap_or(0);

        let mut file = tokio::fs::File::create(&dest).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.try_next().await? {
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            let _ = progress_tx.send(DownloadProgress {
                downloaded,
                total,
                speed_bps: 0, // calculated externally
            }).await;
        }

        Ok(dest)
    }

    pub fn delete(&self, model_name: &str) -> Result<()> {
        let path = self.models_dir.join(format!("{}.gguf", model_name));
        std::fs::remove_file(&path)?;
        Ok(())
    }

    pub fn disk_usage(&self) -> Result<u64> {
        let mut total = 0u64;
        for entry in std::fs::read_dir(&self.models_dir)? {
            let entry = entry?;
            total += entry.metadata()?.len();
        }
        Ok(total)
    }
}
```

---

## 7. Request Format Conversion

```rust
impl MessageRequest {
    /// Convert to Anthropic API format (existing)
    pub fn to_anthropic_format(&self) -> serde_json::Value {
        serde_json::json!({
            "model": &self.model,
            "max_tokens": self.max_tokens,
            "system": &self.system,
            "messages": &self.messages,
            "tools": &self.tools,
            "stream": true
        })
    }

    /// Convert to OpenAI-compatible format for llama-server
    pub fn to_openai_format(&self, model: &str) -> serde_json::Value {
        let mut messages = vec![
            serde_json::json!({"role": "system", "content": &self.system})
        ];
        for msg in &self.messages {
            messages.push(msg.to_openai_format());
        }

        serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": self.max_tokens,
            "stream": true
        })
    }
}
```

---

## 8. Implementation Stages

**Stage 1 — Backend Abstraction (1-2 days)**
1. Define `LlmBackend` trait.
2. Refactor existing Anthropic client to implement the trait.
3. No behavioral change — cloud-only still works.

**Stage 2 — OpenAI-Compatible Client (1-2 days)**
1. Implement `LocalBackend` with OpenAI format.
2. OpenAI SSE parser.
3. Request format conversion.
4. Test against standalone llama-server.

**Stage 3 — Backend Router (1-2 days)**
1. Implement `BackendRouter` with auto/cloud/local modes.
2. Routing heuristics (message length, tool complexity).
3. Fallback logic.
4. IPC messages for mode switching.

**Stage 4 — llama-server Management (1 day)**
1. `LlamaServer` lifecycle (start, stop, health check).
2. systemd service file for llama-server.
3. Model switching (stop, change model, start).

**Stage 5 — Model Management (1-2 days)**
1. Model download with progress.
2. List, delete, disk usage.
3. Skill manifest and tool JSON schemas.

**Stage 6 — UI Integration (0.5 day)**
1. Status bar backend indicator.
2. Offline banner display.
3. Routing info card.

---

## 9. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `router.rs` | Routing decisions for various message types |
| `openai_compat.rs` | OpenAI SSE parsing, format conversion |
| `models.rs` | Model listing, disk usage calculation |

### Integration Tests

| Test | Method |
|------|--------|
| Cloud → Local fallback | Mock cloud failure, verify local used |
| Routing simple query | Send "what time is it?", verify routed to local |
| Routing complex query | Send multi-tool request, verify routed to cloud |
| Self-improvement always cloud | Set auto mode, send self-improvement, verify cloud |
| Mode switching | Switch to local-only, verify all queries go local |
| Model download | Download test file, verify progress callbacks |
| llama-server lifecycle | Start, health check, stop, verify states |
