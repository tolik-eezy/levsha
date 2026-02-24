# Levsha OS — Module 10: Self-Improvement Implementation Plan

## Context

Module 10 (Self-Improvement) has been redesigned to delegate source code editing to an external coding agent (Claude Code or OpenCode) rather than reimplementing a full coding agent from scratch. This cuts the tool surface from 11 tools to 3 and eliminates the `SourceManager` entirely.

**PRD:** `docs/PRD/10-self-improvement/prd.md`
**Design:** `docs/PRD/10-self-improvement/design.md`
**Technical Plan:** `docs/PRD/10-self-improvement/technical-plan.md`

### Architecture Summary

```
User request → Engine detects self-improvement task
  → Phase 1: Spawn coding agent (claude / opencode) as subprocess
     Agent works autonomously in /usr/src/levsha/
     JSONL output streams to split-view panel
  → Phase 2: Deploy
     git commit → cargo build → cargo test → checkpoint → diff → confirm → deploy → health check
```

### Tool Surface (3 tools)

| Tool | Description |
|------|-------------|
| `self_improve` | Spawn coding agent subprocess with user's request |
| `deploy_build` | Build + test + checkpoint + diff + deploy on approval |
| `rollback` | Revert to previous checkpoint |

---

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| Split-View (12) | Required | Agent activity stream and diff rendering use the content panel |
| Skills System (04) | Required | Self-improvement is a built-in skill |
| Intelligence Engine (03) | Required | Tool dispatch, IPC for split-view events |
| Persistence (07) | Required | Conversation survives component restarts |
| Git (system) | Required | Must be in the base image |
| Rust toolchain (system) | Required | On-device compilation |
| Claude Code or OpenCode (system) | Required | External coding agent binary |

---

## Dependency Graph

```
Stage 1 — coding_agent.rs
    │
    ├── Stage 2 — Engine integration (handlers)
    │       │
    │       ├── Stage 3 — Skill definitions + system prompt
    │       │
    │       └── Stage 4 — Config + testing
    │
    └── Stage 5 — Cleanup (independent, after Stage 2)
```

---

## Stage 1 — Coding Agent Module (~3 hrs, `engine` agent)

**Team:** `levsha-selfimprove`
**Files:** `engine/src/self_improve/coding_agent.rs`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| `CodingAgentBackend` enum | `engine` | `coding_agent.rs` | Define `ClaudeCode` and `OpenCode` variants with backend-specific CLI flag logic. |
| `CodingAgentConfig` struct | `engine` | `coding_agent.rs` | Config struct: `backend`, `binary_path`, `source_root`, `timeout`, `max_turns`. Include `default_claude_code()` constructor. |
| `AgentEvent` enum | `engine` | `coding_agent.rs` | Event types: `Thinking`, `FileRead`, `FileEdit`, `BashCommand`, `CodeSearch`, `Complete`, `Error`, `Timeout`, `Unknown`. |
| `CodingAgentSession::spawn()` | `engine` | `coding_agent.rs` | Spawn agent binary as `tokio::process::Child`. Set working dir to source root. Capture stdout, pipe JSONL lines through `parse_jsonl_event()` into `mpsc::Sender<AgentEvent>`. Handle timeout with `tokio::select!`. |
| `CodingAgentSession::kill()` | `engine` | `coding_agent.rs` | Graceful shutdown: SIGTERM → wait 5s → SIGKILL. |
| `CodingAgentSession::wait()` | `engine` | `coding_agent.rs` | Wait for child process exit, return `ExitStatus`. |
| `parse_jsonl_event()` | `engine` | `coding_agent.rs` | Parse Claude Code `stream-json` format. Map `assistant` messages with `thinking` content to `Thinking`. Map `tool_use` with `Read`/`Edit`/`Write`/`Bash`/`Grep`/`Glob` to corresponding event types. Map `result` to `Complete`/`Error`. Unknown → `Unknown`. |
| JSONL parsing unit tests | `engine` | `coding_agent.rs` (tests) | Fixture-based tests for each JSONL event type. Test malformed JSON handling (returns `Unknown`). Test multi-line events. |
| Timeout unit test | `engine` | `coding_agent.rs` (tests) | Spawn a mock script that hangs, verify `Timeout` event emitted and process killed. |

**Deliverable:** `coding_agent.rs` compiles, JSONL parsing works against fixtures, subprocess lifecycle (spawn/kill/wait) works.

---

## Stage 2 — Engine Integration (~3 hrs, `engine` agent)

**Files:** `engine/src/self_improve/mod.rs`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| `self_improve` handler | `engine` | `mod.rs` | Receive user prompt from tool call. Spawn `CodingAgentSession`. Forward `AgentEvent` stream to split-view panel via `ContentOpen` + `ContentUpdate` IPC messages. On agent completion, report status to chat. |
| Agent → split-view bridge | `engine` | `mod.rs` | Map `AgentEvent` variants to split-view content: `Thinking` → prefixed text, `FileRead` → file path + preview, `FileEdit` → diff snippet, `BashCommand` → command + output, `Complete`/`Error`/`Timeout` → status line. |
| `deploy_build` handler | `engine` | `mod.rs` | Sequence: (1) `GitManager::commit_all()` with auto-generated message, (2) `BuildOrchestrator::build()` streaming progress to split-view, (3) `BuildOrchestrator::test()`, (4) `CheckpointManager::create_checkpoint()`, (5) show `git diff` in split-view + confirmation dialog, (6) on approval `DeployManager::deploy()`. |
| `rollback` handler | `engine` | `mod.rs` | Call `CheckpointManager::restore()` with provided checkpoint ID (or latest). Restart affected service. Report result. |
| Tool registration | `engine` | `mod.rs`, `engine/src/tool_executor.rs` | Register `self_improve`, `deploy_build`, `rollback` in the engine's tool registry. Wire to handlers. |
| Error handling | `engine` | `mod.rs` | Agent spawn failure → report error to chat. Build failure → report error, no deploy. Test failure → report warning, ask user whether to proceed. Deploy failure → automatic rollback. |

**Deliverable:** All three tool handlers work end-to-end. `self_improve` spawns agent and streams to split-view. `deploy_build` runs the full pipeline. `rollback` restores from checkpoint.

---

## Stage 3 — Skill Definitions + System Prompt (~1 hr, `skills-dev` agent)

**Files:** `skills/built-in/self-improvement/`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| `self_improve.json` | `skills-dev` | `tools/self_improve.json` | Tool schema: `prompt` (string, required) — the user's request. Optional: `target_component` (enum: chat-shell, engine, skills, config). |
| `deploy_build.json` | `skills-dev` | `tools/deploy_build.json` | Tool schema: `component` (enum: chat-shell, engine, full, required). |
| `rollback.json` | `skills-dev` | `tools/rollback.json` | Tool schema: `checkpoint_id` (string, optional — defaults to latest). |
| System prompt | `skills-dev` | `prompts/self-improvement.md` | Describe the two-phase flow. Instruct the LLM: detect self-improvement requests, use `self_improve` to spawn the coding agent, then `deploy_build` to apply. Explain `rollback` for reverting. Include safety notes (always confirm before deploy). |
| Skill manifest | `skills-dev` | `skill.yaml` | Update to version 0.2.0. List 3 tools. Add `config` section for `agent_backend`, `agent_binary`, `agent_timeout`. Update `requires.packages` (add `claude-code`, remove `ripgrep`). |
| Remove old tool schemas | `skills-dev` | `tools/` | Delete: `source_read.json`, `source_search.json`, `source_write.json`, `source_diff.json`, `build.json`, `build_status.json`, `test.json`, `deploy.json`, `git_log.json`, `git_commit.json`. |

**Deliverable:** Skill directory contains 3 tool JSONs, updated manifest, and system prompt. Old schemas removed.

---

## Stage 4 — Configuration + End-to-End Testing (~2 hrs, `engine` + `qa` agents)

**Files:** `engine/src/config.rs`, `base/overlay/etc/levsha/config.toml`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Config section | `engine` | `engine/src/config.rs` | New `[self_improve]` section: `agent_backend = "claude-code"`, `agent_binary = "/usr/bin/claude"`, `agent_timeout = 600`, `max_turns = 50`. |
| Config parsing | `engine` | `engine/src/config.rs` | Parse `[self_improve]` into `CodingAgentConfig` at engine startup. Validate binary exists at path. |
| Default config | `engine` | `base/overlay/etc/levsha/config.toml` | Add `[self_improve]` section with defaults. |
| Mock agent script | `qa` | `tests/fixtures/mock_agent.sh` | Bash script that outputs JSONL lines mimicking a coding agent session: thinking, file read, file edit, bash command, complete. Used for integration tests. |
| Integration test: full flow | `qa` | `tests/self_improve_integration.rs` | Spawn mock agent → verify JSONL events parsed → verify split-view messages sent → run deploy_build with a test repo → verify checkpoint created → verify binary swapped. |
| Integration test: timeout | `qa` | `tests/self_improve_integration.rs` | Spawn mock agent that hangs → verify timeout fires → verify process killed → verify Timeout event emitted. |
| Integration test: agent error | `qa` | `tests/self_improve_integration.rs` | Spawn mock agent that exits with error code → verify Error event → verify no deploy attempted. |
| Integration test: rollback | `qa` | `tests/self_improve_integration.rs` | Deploy binary that fails health check → verify automatic rollback → verify checkpoint restored. |

**Deliverable:** Config wired, mock agent works, all integration tests pass.

---

## Stage 5 — Cleanup (~0.5 hr, `engine` agent)

**Files:** `engine/src/self_improve/`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Remove `source.rs` | `engine` | `engine/src/self_improve/source.rs` | Delete file if it exists. |
| Remove `source_tests.rs` | `engine` | `engine/src/self_improve/source_tests.rs` | Delete file if it exists. |
| Remove old handlers | `engine` | `engine/src/self_improve/mod.rs` | Remove any handler code for the old 11-tool surface (source_read, source_write, source_search, source_diff, build, build_status, test, deploy, git_log, git_commit). |
| Update cross-references | `engine` | Various | Grep for stale references to `SourceManager`, `source_read`, etc. in other modules. Update or remove. |
| Update `Cargo.toml` | `engine` | `engine/Cargo.toml` | Remove `notify` crate dependency (was used for filesystem watching). |

**Deliverable:** No stale code or references. `cargo build` and `cargo test` pass cleanly.

---

## L3 — Chat Shell UI Work (~2 hrs, `chat-shell` agent, parallel with Stages 2-4)

**Files:** `chat-shell/src/`

This track can run in parallel with engine work since it uses the established split-view IPC protocol.

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Agent activity stream renderer | `chat-shell` | `chat-shell/src/content_panel/agent_activity.rs` | New content panel mode: render `AgentEvent` stream with categorized prefixes (💭 Thinking, 🔍 Reading, ✏️ Editing, ▶ Running, etc.). Auto-scroll to latest event. User scroll-up pauses auto-scroll. |
| Event prefix styling | `chat-shell` | `chat-shell/src/content_panel/agent_activity.rs` | Apply design tokens: file paths in `$accent-copper`, added lines in `$accent-green`, code in IBM Plex Mono, category labels in weight 600. Dashed separators between sections. |
| Post-agent diff view | `chat-shell` | `chat-shell/src/content_panel/diff_view.rs` | After agent completes, switch split-view to show `git diff` output with standard diff coloring (green additions, copper removals). Reuse existing diff rendering if available, or implement per design spec. |
| Deploy confirmation dialog | `chat-shell` | `chat-shell/src/deploy_confirm.rs` | Styled confirmation dialog per design spec: warning icon, file change summary, Apply/Cancel buttons. Copper border, `$danger-bg` background. |
| Health check status | `chat-shell` | `chat-shell/src/health_check_status.rs` | In-chat indicators: pulsing dots during check, green checkmark on success, copper alert on failure + rollback message. |
| Version history card | `chat-shell` | `chat-shell/src/content_panel/version_history.rs` | Render version timeline: current version (green dot), checkpoints (gray dots), commit messages, timestamps. "Rollback to..." button. |

**Deliverable:** All L3 components render correctly. Agent activity stream shows live updates. Diff view, confirmation dialog, health check status, and version history all match design spec.

---

## Execution Plan

### Parallel Tracks

```
Time →

Track A (engine):   [Stage 1: coding_agent.rs] → [Stage 2: handlers] → [Stage 4: config] → [Stage 5: cleanup]
                                                        │
Track B (chat-shell): ─────────────────────────── [L3 UI work] ──────────────────────────────
                                                        │
Track C (skills-dev): ─────────────────────────── [Stage 3: skill defs] ──────────────────────
                                                        │
Track D (qa):         ────────────────────────────────────────── [Stage 4: integration tests] ─
```

### Team Setup

```bash
# Create worktrees
git worktree add ../Levsha.OS-wt-engine main
git worktree add ../Levsha.OS-wt-chat-shell main
git worktree add ../Levsha.OS-wt-skills-dev main

# Create team
TeamCreate(team_name="levsha-selfimprove")

# Spawn agents
Task(name="engine",     prompt="Work in ../Levsha.OS-wt-engine/ ...")
Task(name="chat-shell", prompt="Work in ../Levsha.OS-wt-chat-shell/ ...")
Task(name="skills-dev", prompt="Work in ../Levsha.OS-wt-skills-dev/ ...")
```

### Merge Order

1. Merge `engine` branch (Stage 1 + 2) — core functionality
2. Merge `skills-dev` branch (Stage 3) — skill definitions
3. Merge `chat-shell` branch (L3) — UI components
4. Run integration tests (Stage 4) from main worktree
5. Cleanup (Stage 5) from main worktree

---

## Estimated Effort

| Stage | Agent(s) | Estimated Time |
|-------|----------|----------------|
| Stage 1 — Coding Agent Module | `engine` | 2-3 hours |
| Stage 2 — Engine Integration | `engine` | 2-3 hours |
| Stage 3 — Skill Definitions | `skills-dev` | 1 hour |
| Stage 4 — Config + Testing | `engine` + `qa` | 1-2 hours |
| Stage 5 — Cleanup | `engine` | 0.5 hours |
| L3 — Chat Shell UI | `chat-shell` | 2 hours |
| **Total (sequential)** | | **~9-11 hours** |
| **Total (parallel, 3 agents)** | | **~5-6 hours** |

---

## Verification Checklist

### Core Functionality

- [ ] `self_improve` tool spawns coding agent subprocess in `/usr/src/levsha/`
- [ ] Agent JSONL output streams to split-view in real time
- [ ] Agent events categorized correctly (thinking, reads, edits, commands)
- [ ] Agent timeout enforced (default 10 min)
- [ ] Timeout kills subprocess gracefully (SIGTERM → SIGKILL)
- [ ] `deploy_build` commits, builds, tests, checkpoints, shows diff, deploys on approval
- [ ] `rollback` restores from latest checkpoint
- [ ] Deploy requires explicit user confirmation
- [ ] Failed health check triggers automatic rollback

### UI

- [ ] Agent activity stream renders with correct prefixes and styling
- [ ] Auto-scroll works; user scroll-up pauses auto-scroll
- [ ] Post-agent diff view shows all changes with green/copper coloring
- [ ] Deploy confirmation dialog matches design spec
- [ ] Health check status indicators work (pulsing, success, failure, rollback)
- [ ] Version history card renders timeline correctly

### Safety

- [ ] Agent subprocess runs only in source tree
- [ ] Agent cannot modify `/usr/bin` directly (only `deploy_build` can)
- [ ] Git commit created before every deploy
- [ ] Cloud API always used for self-improvement (never local model)
- [ ] Conversation history preserved across component restarts

### Configuration

- [ ] `[self_improve]` config section parsed correctly
- [ ] Agent binary path validated at startup
- [ ] Config defaults work when section is omitted
- [ ] Both `claude-code` and `opencode` backends supported

### Cleanup

- [ ] No references to `SourceManager`, `source_read`, `source_write`, `source_search`, `source_diff`
- [ ] No references to old tools: `build`, `build_status`, `test`, `deploy`, `git_log`, `git_commit`
- [ ] `source.rs` deleted
- [ ] `notify` crate removed from `Cargo.toml`
- [ ] `cargo build` and `cargo test` pass cleanly

---

## Risk Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Claude Code / OpenCode not installed | Agent spawn fails | Check binary at startup, warn user. Fall back to error message with install instructions. |
| Agent produces invalid output (non-JSONL) | Parse errors | `Unknown` event type catches all unparseable lines. Display raw output as fallback. |
| Agent modifies files outside source tree | Security | Agent's working dir is locked to `/usr/src/levsha/`. Monitor for escapes in integration tests. |
| Agent session very long (>10 min) | Resource usage | Configurable timeout with graceful kill. User can cancel via chat. |
| Build fails after agent edits | Broken state | No deploy on build failure. Source tree still has agent's edits — user can retry or rollback. |
| Health check false negative | Unnecessary rollback | Configurable health check timeout. Log the actual service status for debugging. |
| JSONL format changes between agent versions | Parse breakage | `Unknown` fallback for unrecognized events. Version-specific parsers if needed. |

---

## Files Created / Modified

### New Files

| File | Description |
|------|-------------|
| `engine/src/self_improve/coding_agent.rs` | Coding agent subprocess management + JSONL parser |
| `skills/built-in/self-improvement/tools/self_improve.json` | Tool schema for spawning coding agent |
| `skills/built-in/self-improvement/tools/deploy_build.json` | Tool schema for build + deploy pipeline |
| `chat-shell/src/content_panel/agent_activity.rs` | Agent activity stream renderer |
| `chat-shell/src/deploy_confirm.rs` | Deploy confirmation dialog |
| `chat-shell/src/health_check_status.rs` | Health check status indicators |
| `chat-shell/src/content_panel/version_history.rs` | Version history card |
| `tests/fixtures/mock_agent.sh` | Mock agent for integration tests |
| `tests/self_improve_integration.rs` | Integration tests |

### Modified Files

| File | Change |
|------|--------|
| `engine/src/self_improve/mod.rs` | New handlers for 3 tools, remove old 11-tool handlers |
| `engine/src/config.rs` | Add `[self_improve]` config section |
| `engine/src/tool_executor.rs` | Register 3 new tools |
| `engine/Cargo.toml` | Remove `notify` crate |
| `base/overlay/etc/levsha/config.toml` | Add `[self_improve]` defaults |
| `skills/built-in/self-improvement/skill.yaml` | Update to v0.2.0, 3 tools, config fields |
| `skills/built-in/self-improvement/prompts/self-improvement.md` | Rewrite for two-phase flow |
| `chat-shell/src/content_panel/diff_view.rs` | Add post-agent diff rendering (may be new) |

### Deleted Files

| File | Reason |
|------|--------|
| `engine/src/self_improve/source.rs` | External agent handles all file ops |
| `engine/src/self_improve/source_tests.rs` | Tests for deleted module |
| `skills/built-in/self-improvement/tools/source_read.json` | Replaced by `self_improve` |
| `skills/built-in/self-improvement/tools/source_search.json` | Replaced by `self_improve` |
| `skills/built-in/self-improvement/tools/source_write.json` | Replaced by `self_improve` |
| `skills/built-in/self-improvement/tools/source_diff.json` | Replaced by `self_improve` |
| `skills/built-in/self-improvement/tools/build.json` | Replaced by `deploy_build` |
| `skills/built-in/self-improvement/tools/build_status.json` | Replaced by `deploy_build` |
| `skills/built-in/self-improvement/tools/test.json` | Replaced by `deploy_build` |
| `skills/built-in/self-improvement/tools/deploy.json` | Replaced by `deploy_build` |
| `skills/built-in/self-improvement/tools/git_log.json` | No longer exposed as separate tool |
| `skills/built-in/self-improvement/tools/git_commit.json` | Handled internally by `deploy_build` |
