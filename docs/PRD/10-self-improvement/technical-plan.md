# 10 — Self-Improvement System: Technical Plan

**Module:** Self-Improvement (L2 + L3)
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

The self-improvement system delegates source code editing to an external coding agent and handles build/deploy internally.

```
engine/
  src/
    self_improve/
      mod.rs              # Public API, self-improvement coordinator
      coding_agent.rs     # Coding agent subprocess spawn, JSONL parser
      git.rs              # Git operations (commit, diff, log, tag)
      builder.rs          # Build orchestration (cargo build/test)
      deployer.rs         # Binary swap, service restart, health check
      checkpoint.rs       # Checkpoint creation and restore

skills/
  built-in/
    self-improvement/
      skill.yaml          # Skill manifest
      prompts/
        self-improvement.md  # System prompt fragment
      tools/
        self_improve.json    # Spawn coding agent
        deploy_build.json    # Build + checkpoint + deploy
        rollback.json        # Revert to checkpoint
```

### Dependencies

```toml
# Added to engine/Cargo.toml
git2 = "0.19"          # libgit2 bindings for Git operations
```

> **Removed:** `notify` crate (no longer needed — build output is streamed directly from cargo, not watched via filesystem). `ripgrep` system dependency is also removed — the coding agent bundles its own search tools.

---

## 2. Coding Agent

The coding agent module spawns an external coding agent (Claude Code or OpenCode) as a child process and parses its structured JSONL output into events for the split-view panel.

### Configuration

```rust
#[derive(Debug, Clone)]
pub enum CodingAgentBackend {
    ClaudeCode,
    OpenCode,
}

#[derive(Debug, Clone)]
pub struct CodingAgentConfig {
    pub backend: CodingAgentBackend,
    pub binary_path: PathBuf,       // e.g., /usr/bin/claude or /usr/bin/opencode
    pub source_root: PathBuf,       // /usr/src/levsha/
    pub timeout: Duration,          // default: 10 minutes
    pub max_turns: Option<usize>,   // optional turn limit for the agent
}

impl CodingAgentConfig {
    pub fn default_claude_code() -> Self {
        Self {
            backend: CodingAgentBackend::ClaudeCode,
            binary_path: PathBuf::from("/usr/bin/claude"),
            source_root: PathBuf::from("/usr/src/levsha"),
            timeout: Duration::from_secs(600),
            max_turns: None,
        }
    }
}
```

### Agent Events

```rust
/// Events parsed from the agent's JSONL output stream.
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent is thinking / reasoning
    Thinking { content: String },

    /// Agent is reading a file
    FileRead { path: String, content_preview: Option<String> },

    /// Agent edited a file
    FileEdit { path: String, diff_snippet: Option<String> },

    /// Agent ran a shell command
    BashCommand { command: String, output_preview: Option<String> },

    /// Agent searched for code
    CodeSearch { pattern: String, match_count: Option<usize> },

    /// Agent completed successfully
    Complete { summary: String, duration: Duration },

    /// Agent encountered an error
    Error { message: String },

    /// Agent timed out
    Timeout,

    /// Raw JSONL line that doesn't match known categories
    Unknown { raw: String },
}
```

### Subprocess Management

```rust
pub struct CodingAgentSession {
    config: CodingAgentConfig,
    child: Option<tokio::process::Child>,
}

impl CodingAgentSession {
    /// Spawn the coding agent with the given prompt.
    /// Returns a stream of AgentEvents parsed from JSONL output.
    pub async fn spawn(
        config: &CodingAgentConfig,
        prompt: &str,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        let mut cmd = tokio::process::Command::new(&config.binary_path);

        match config.backend {
            CodingAgentBackend::ClaudeCode => {
                cmd.arg("--print")
                   .arg("--output-format").arg("stream-json")
                   .arg("--max-turns").arg(
                       config.max_turns.unwrap_or(50).to_string()
                   )
                   .arg("--prompt").arg(prompt);
            }
            CodingAgentBackend::OpenCode => {
                cmd.arg("--non-interactive")
                   .arg("--output").arg("jsonl")
                   .arg("--prompt").arg(prompt);
            }
        }

        cmd.current_dir(&config.source_root)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();

        // Spawn a task to parse JSONL lines and forward as AgentEvents
        let timeout = config.timeout;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let deadline = tokio::time::Instant::now() + timeout;

            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                let event = parse_jsonl_event(&line);
                                if event_tx.send(event).await.is_err() {
                                    break; // receiver dropped
                                }
                            }
                            Ok(None) => break, // EOF
                            Err(_) => break,
                        }
                    }
                    _ = tokio::time::sleep_until(deadline) => {
                        let _ = event_tx.send(AgentEvent::Timeout).await;
                        break;
                    }
                }
            }
        });

        Ok(Self { config: config.clone(), child: Some(child) })
    }

    /// Kill the agent subprocess gracefully (SIGTERM, then SIGKILL after 5s).
    pub async fn kill(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.child {
            // Send SIGTERM
            child.start_kill()?;
            // Wait up to 5 seconds for graceful exit
            match tokio::time::timeout(
                Duration::from_secs(5), child.wait()
            ).await {
                Ok(_) => {}
                Err(_) => {
                    // Force kill
                    child.kill().await?;
                }
            }
        }
        Ok(())
    }

    /// Wait for the agent to finish and return its exit status.
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        if let Some(ref mut child) = self.child {
            Ok(child.wait().await?)
        } else {
            Err(anyhow::anyhow!("Agent not running"))
        }
    }
}

/// Parse a single JSONL line into an AgentEvent.
fn parse_jsonl_event(line: &str) -> AgentEvent {
    // Parse the JSON and map known event types to AgentEvent variants.
    // Claude Code stream-json format:
    //   {"type": "assistant", "message": {"content": [{"type": "thinking", "thinking": "..."}]}}
    //   {"type": "assistant", "message": {"content": [{"type": "tool_use", "name": "Read", ...}]}}
    //   {"type": "result", "result": "..."}
    //
    // Unknown formats are wrapped in AgentEvent::Unknown.
    let Ok(json) = serde_json::from_str::<serde_json::Value>(line) else {
        return AgentEvent::Unknown { raw: line.to_string() };
    };

    match json.get("type").and_then(|t| t.as_str()) {
        Some("assistant") => parse_assistant_event(&json),
        Some("result") => parse_result_event(&json),
        _ => AgentEvent::Unknown { raw: line.to_string() },
    }
}
```

---

## 3. Git Operations

Git operations are used for committing agent changes before deploy, generating diffs for the confirmation dialog, and tagging checkpoints.

```rust
pub struct GitManager {
    repo: git2::Repository,
}

impl GitManager {
    pub fn open(source_root: &Path) -> Result<Self> {
        let repo = git2::Repository::open(source_root)?;
        Ok(Self { repo })
    }

    pub fn current_hash(&self) -> Result<String> {
        let head = self.repo.head()?.peel_to_commit()?;
        Ok(head.id().to_string())
    }

    pub fn commit_all(&self, message: &str) -> Result<git2::Oid> {
        let mut index = self.repo.index()?;
        index.add_all(["*"], git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;
        let head = self.repo.head()?.peel_to_commit()?;
        let sig = git2::Signature::now("Levsha OS", "self-improve@levsha.os")?;
        let oid = self.repo.commit(
            Some("HEAD"), &sig, &sig, message, &tree, &[&head]
        )?;
        Ok(oid)
    }

    pub fn diff_working(&self) -> Result<String> {
        let head = self.repo.head()?.peel_to_tree()?;
        let diff = self.repo.diff_tree_to_workdir(Some(&head), None)?;
        let mut output = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            output.push(line.origin());
            output.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })?;
        Ok(output)
    }

    pub fn diff_file_list(&self) -> Result<Vec<String>> {
        let head = self.repo.head()?.peel_to_tree()?;
        let diff = self.repo.diff_tree_to_workdir(Some(&head), None)?;
        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files.push(path.to_string_lossy().to_string());
                }
                true
            },
            None, None, None
        )?;
        Ok(files)
    }

    pub fn log(&self, count: usize) -> Result<Vec<CommitInfo>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.take(count)
            .map(|oid| {
                let commit = self.repo.find_commit(oid?)?;
                Ok(CommitInfo {
                    hash: commit.id().to_string()[..8].to_string(),
                    message: commit.message().unwrap_or("").to_string(),
                    timestamp: commit.time().seconds(),
                })
            })
            .collect()
    }

    pub fn tag_checkpoint(&self, tag_name: &str) -> Result<()> {
        let head = self.repo.head()?.peel_to_commit()?;
        let sig = git2::Signature::now("Levsha OS", "self-improve@levsha.os")?;
        self.repo.tag(
            tag_name,
            head.as_object(),
            &sig,
            &format!("Checkpoint: {}", tag_name),
            false
        )?;
        Ok(())
    }
}
```

---

## 4. Build Orchestration

```rust
pub struct BuildOrchestrator {
    source_root: PathBuf,
}

pub enum BuildTarget {
    ChatShell,
    Engine,
    Full,
}

pub struct BuildResult {
    pub success: bool,
    pub artifacts: Vec<PathBuf>,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

impl BuildOrchestrator {
    pub async fn build(
        &self,
        target: BuildTarget,
        progress_tx: mpsc::Sender<BuildProgress>,
    ) -> Result<BuildResult> {
        let (package, artifact_name) = match target {
            BuildTarget::ChatShell => ("chat-shell", "levsha-chat"),
            BuildTarget::Engine => ("engine", "levsha-engine"),
            BuildTarget::Full => return self.build_full(progress_tx).await,
        };

        let mut child = tokio::process::Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("-p").arg(package)
            .current_dir(&self.source_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stderr = child.stderr.take().unwrap();
        let reader = BufReader::new(stderr);

        // Stream build output line-by-line
        let mut lines = reader.lines();
        let mut output = String::new();
        while let Some(line) = lines.next_line().await? {
            output.push_str(&line);
            output.push('\n');
            let progress = parse_cargo_progress(&line);
            let _ = progress_tx.send(progress).await;
        }

        let status = child.wait().await?;
        let artifact = self.source_root
            .join("target/release")
            .join(artifact_name);

        Ok(BuildResult {
            success: status.success(),
            artifacts: vec![artifact],
            stdout: String::new(),
            stderr: output,
            duration: Duration::default(), // tracked externally
        })
    }

    pub async fn test(&self, target: BuildTarget) -> Result<TestResult> {
        let package = match target {
            BuildTarget::ChatShell => "chat-shell",
            BuildTarget::Engine => "engine",
            BuildTarget::Full => "--workspace",
        };

        let output = tokio::process::Command::new("cargo")
            .arg("test")
            .arg("-p").arg(package)
            .current_dir(&self.source_root)
            .output()
            .await?;

        Ok(TestResult {
            success: output.status.success(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            errors: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}
```

---

## 5. Checkpoint and Rollback

```rust
pub struct CheckpointManager {
    checkpoint_dir: PathBuf,  // /var/lib/levsha/checkpoints/
    max_checkpoints: usize,
}

pub struct Checkpoint {
    pub id: String,           // timestamp + git hash
    pub git_hash: String,
    pub timestamp: i64,
    pub binaries: Vec<PathBuf>,
    pub configs: Vec<PathBuf>,
}

impl CheckpointManager {
    pub fn create_checkpoint(
        &self,
        git_hash: &str,
        components: &[DeployTarget],
    ) -> Result<Checkpoint> {
        let id = format!("{}-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"), &git_hash[..8]);
        let checkpoint_path = self.checkpoint_dir.join(&id);
        std::fs::create_dir_all(&checkpoint_path)?;

        let mut binaries = Vec::new();
        let mut configs = Vec::new();

        for target in components {
            let (src_binary, src_configs) = target.current_paths();
            let dst = checkpoint_path.join(target.name());
            std::fs::create_dir_all(&dst)?;
            std::fs::copy(&src_binary, dst.join(src_binary.file_name().unwrap()))?;
            binaries.push(src_binary.clone());
            for config in src_configs {
                std::fs::copy(&config, dst.join(config.file_name().unwrap()))?;
                configs.push(config.clone());
            }
        }

        // Write metadata
        let meta = CheckpointMeta { id: id.clone(), git_hash: git_hash.to_string(),
            timestamp: chrono::Utc::now().timestamp() };
        let meta_path = checkpoint_path.join("metadata.json");
        std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;

        self.prune_old_checkpoints()?;
        Ok(Checkpoint { id, git_hash: git_hash.to_string(),
            timestamp: meta.timestamp, binaries, configs })
    }

    pub fn restore(&self, checkpoint_id: &str) -> Result<()> {
        let checkpoint_path = self.checkpoint_dir.join(checkpoint_id);
        let _meta: CheckpointMeta = serde_json::from_str(
            &std::fs::read_to_string(checkpoint_path.join("metadata.json"))?
        )?;

        // Restore binaries from checkpoint
        for entry in std::fs::read_dir(&checkpoint_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.file_name() != "metadata.json" {
                self.restore_component(&entry.path())?;
            }
        }
        Ok(())
    }

    fn prune_old_checkpoints(&self) -> Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(&self.checkpoint_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by_key(|e| e.file_name());
        while entries.len() > self.max_checkpoints {
            let oldest = entries.remove(0);
            std::fs::remove_dir_all(oldest.path())?;
        }
        Ok(())
    }
}
```

---

## 6. Deploy Manager

```rust
pub struct DeployManager {
    checkpoint_mgr: CheckpointManager,
}

pub enum DeployTarget {
    ChatShell,
    Engine,
}

impl DeployTarget {
    fn binary_path(&self) -> &Path {
        match self {
            Self::ChatShell => Path::new("/usr/bin/levsha-chat"),
            Self::Engine => Path::new("/usr/bin/levsha-engine"),
        }
    }

    fn service_name(&self) -> &str {
        match self {
            Self::ChatShell => "levsha-chat.service",
            Self::Engine => "levsha-engine.service",
        }
    }
}

impl DeployManager {
    pub async fn deploy(
        &self,
        target: DeployTarget,
        artifact: &Path,
        git_hash: &str,
    ) -> Result<DeployResult> {
        // 1. Create checkpoint
        let checkpoint = self.checkpoint_mgr.create_checkpoint(
            git_hash, &[target.clone()]
        )?;

        // 2. Copy new binary
        std::fs::copy(artifact, target.binary_path())?;

        // 3. Restart service
        let restart = tokio::process::Command::new("systemctl")
            .arg("restart")
            .arg(target.service_name())
            .output()
            .await?;

        if !restart.status.success() {
            // Restart failed — rollback immediately
            self.checkpoint_mgr.restore(&checkpoint.id)?;
            return Err(DeployError::RestartFailed);
        }

        // 4. Health check (wait for service to be ready)
        match self.health_check(&target, Duration::from_secs(10)).await {
            Ok(()) => Ok(DeployResult::Success),
            Err(_) => {
                // Health check failed — rollback
                self.checkpoint_mgr.restore(&checkpoint.id)?;
                let _ = tokio::process::Command::new("systemctl")
                    .arg("restart")
                    .arg(target.service_name())
                    .output()
                    .await;
                Ok(DeployResult::RolledBack(checkpoint.id))
            }
        }
    }

    async fn health_check(&self, target: &DeployTarget, timeout: Duration) -> Result<()> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > timeout {
                return Err(DeployError::HealthCheckTimeout);
            }
            let status = tokio::process::Command::new("systemctl")
                .arg("is-active")
                .arg(target.service_name())
                .output()
                .await?;
            if status.status.success() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}
```

---

## 7. Skill Manifest

```yaml
name: self-improvement
version: 0.2.0
description: "Delegate source code editing to an external coding agent, then build, deploy, and rollback"
author: levsha
builtin: true

prompt: prompts/self-improvement.md
tools:
  - tools/self_improve.json
  - tools/deploy_build.json
  - tools/rollback.json

config:
  # Which coding agent backend to use: "claude-code" or "opencode"
  agent_backend: claude-code
  # Path to the agent binary
  agent_binary: /usr/bin/claude
  # Agent session timeout in seconds
  agent_timeout: 600

requires:
  packages:
    - git
    - rust
    - gcc
    - make
    - claude-code   # or opencode — at least one coding agent must be installed
```

---

## 8. Implementation Stages

**Stage 1 — Coding Agent Module (2-3 days)**
1. Implement `CodingAgentConfig`, `CodingAgentBackend`, `AgentEvent` enum.
2. Implement `CodingAgentSession::spawn()` — subprocess creation with JSONL stdout capture.
3. Implement `parse_jsonl_event()` — parse Claude Code `stream-json` format into `AgentEvent` variants.
4. Implement `CodingAgentSession::kill()` and `wait()` — graceful shutdown with SIGTERM/SIGKILL.
5. Write unit tests for JSONL parsing with sample event fixtures.

**Stage 2 — Engine Integration (2-3 days)**
1. Implement `self_improve` tool handler in `mod.rs` — receives user prompt, spawns agent, forwards `AgentEvent` stream to the split-view panel via IPC.
2. Implement `deploy_build` tool handler — runs git commit, cargo build, cargo test, checkpoint, shows diff, deploys on approval.
3. Wire `rollback` tool handler to existing `CheckpointManager::restore()`.
4. Register all three tools in the engine's tool registry.

**Stage 3 — Skill Definitions + System Prompt (1 day)**
1. Write `self_improve.json` tool schema — parameter: `prompt` (string), optional: `target_component` (enum).
2. Write `deploy_build.json` tool schema — parameter: `component` (enum: chat-shell, engine, full).
3. Write `rollback.json` tool schema — parameter: `checkpoint_id` (optional string, defaults to latest).
4. Write `self-improvement.md` system prompt fragment describing the two-phase flow.
5. Update `skill.yaml` manifest.

**Stage 4 — Configuration + Testing (1-2 days)**
1. Add `[self_improve]` section to `config.toml` — agent backend, binary path, timeout.
2. Parse config into `CodingAgentConfig` at engine startup.
3. End-to-end integration test: spawn a mock agent script, verify JSONL parsing, verify event stream.
4. Test timeout enforcement and graceful kill.

**Stage 5 — Cleanup (0.5 day)**
1. Remove `source.rs` and `source_tests.rs` from the codebase (if they exist).
2. Remove obsolete tool JSON schemas: `source_read.json`, `source_search.json`, `source_write.json`, `source_diff.json`, `build.json`, `build_status.json`, `test.json`, `deploy.json`, `git_log.json`, `git_commit.json`.
3. Update any cross-references in other PRDs or documentation.

---

## 9. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `coding_agent.rs` | JSONL event parsing, event type classification, malformed input handling |
| `coding_agent.rs` | Agent spawn with mock binary, stdout capture, timeout enforcement |
| `git.rs` | Commit creation, diff generation, diff file list, log parsing, tag creation |
| `builder.rs` | Cargo output parsing, progress extraction |
| `checkpoint.rs` | Checkpoint creation, pruning, metadata serialization |
| `deployer.rs` | Health check logic, rollback trigger conditions |

### Integration Tests

| Test | Method |
|------|--------|
| Full self-improvement flow | Spawn mock agent that edits a test file, then build, deploy to staging dir |
| Agent timeout | Spawn agent that hangs, verify timeout triggers and subprocess is killed |
| Agent error handling | Spawn agent that exits with error, verify error event is emitted |
| JSONL stream integrity | Feed known JSONL sequences, verify all events are parsed and forwarded |
| Rollback on build failure | Introduce syntax error post-agent, verify rollback after build failure |
| Rollback on health check failure | Deploy binary that exits immediately, verify rollback |
| Checkpoint pruning | Create max+1 checkpoints, verify oldest is removed |

### Safety Tests

| Test | Assertion |
|------|-----------|
| Deploy requires confirmation | No binary replacement without user approval |
| Cloud API always used | Self-improvement requests never routed to local model |
| Git commit before deploy | Deploy fails if working tree has uncommitted changes |
| Agent subprocess isolation | Agent runs only in source tree, cannot modify /usr/bin directly |
