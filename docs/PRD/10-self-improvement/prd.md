# 10 — Self-Improvement System: Product Requirements

**Module:** Self-Improvement (L2 + L3)
**Phase:** 2
**Status:** Draft

---

## 1. Overview

Self-improvement is the defining feature of Levsha OS — the OS can diagnose bugs in its own source code, write patches, rebuild itself, and restart with new capabilities. This is what separates Levsha OS from every other operating system.

The self-improvement system consists of three subsystems:

1. **Coding Agent Delegation** — Instead of reimplementing source editing tools from scratch, Levsha spawns an external coding agent (Claude Code or OpenCode) as a subprocess. The agent handles all source investigation, editing, and testing autonomously. Its JSONL output streams into the Chat Shell's split-view so the user can watch progress in real time.
2. **Git-Based Versioning** — All source modifications are committed to a local Git repository. Every change is tracked, diffable, and reversible.
3. **Safe Restart & Rollback** — Before any restart, the system creates a recovery checkpoint. Failed boots automatically revert to the last known-good state.

Theming (dark/light mode, color palette changes, font adjustments) is a special case of self-improvement — the OS modifies its own Chat Shell code to apply theme changes.

---

## 2. Core Concept: The OS That Rewrites Itself

```
User: "The font rendering looks blurry on my display"
       │
       ▼
  Intelligence Engine identifies this as a self-improvement request
       │
       ▼
  Spawns coding agent (Claude Code / OpenCode) as a subprocess
       │
       ▼
  Split-view opens: agent activity streams in the right panel
       │
       ▼
  Agent autonomously investigates source, writes patches, runs tests
       │
       ▼
  Agent session completes — user reviews the final diff
       │
       ▼
  User approves → build, checkpoint, deploy, restart
       │
       ▼
  Chat Shell restarts with the fix.
  Conversation history is preserved.
```

---

## 3. Functional Requirements

### 3.1 Coding Agent Delegation (SI-01)

| Field | Value |
|-------|-------|
| **ID** | SI-01 |
| **Priority** | P0 |
| **Requirement** | The self-improvement system delegates source code investigation and editing to an external coding agent (Claude Code or OpenCode), then handles build, deploy, and rollback internally. |

**Details:**

The self-improvement skill exposes three tools:

| Tool | Description |
|------|-------------|
| `self_improve` | Spawn the coding agent as a subprocess with the user's request. The agent works in the OS source tree (`/usr/src/levsha/`), reading, searching, and editing files autonomously. Its JSONL output streams into the split-view panel. |
| `deploy_build` | After the agent finishes: build the modified component (`cargo build --release`), run tests (`cargo test`), create a checkpoint of current binaries, show the final diff to the user, and on approval deploy + restart. |
| `rollback` | Revert to the previous version of a component (restore from checkpoint). |

**Coding Agent Subprocess:**

- The engine spawns the agent binary (e.g., `claude` or `opencode`) as a child process with `--print jsonl` or equivalent flag to produce structured output.
- The agent's working directory is set to `/usr/src/levsha/`.
- The user's request is passed as the initial prompt.
- JSONL events are parsed and streamed to the split-view panel in real time: thinking, file reads, edits, bash commands, and agent status.
- The agent process has a configurable timeout (default: 10 minutes).
- On completion (or timeout/error), the engine collects the exit status and transitions to the deploy phase.

**Acceptance Criteria:**

- [ ] `self_improve` spawns a coding agent subprocess in the source tree.
- [ ] Agent JSONL output streams into the split-view panel in real time.
- [ ] The split-view shows categorized activity: thinking, file reads, edits, commands.
- [ ] Agent timeout is enforced; timeout kills the subprocess gracefully (SIGTERM, then SIGKILL).
- [ ] `deploy_build` builds, tests, checkpoints, shows diff, and deploys on user approval.
- [ ] `rollback` restores from the most recent checkpoint.
- [ ] All deployments require user confirmation before applying.

### 3.2 Git-Based Self-Versioning (SI-02)

| Field | Value |
|-------|-------|
| **ID** | SI-02 |
| **Priority** | P0 |
| **Requirement** | All source modifications are committed to a local Git repository with full version history. |

**Details:**

- The complete OS source tree is stored at `/usr/src/levsha/` as a Git repository.
- After the coding agent completes its edits, all changes are committed before deployment. The commit message is auto-generated describing the change.
- The user can request the commit history via chat: "what changes have been made?", "show me the last 5 patches".
- The user can diff any two versions: "show me what changed in the last update".
- The source tree includes: `chat-shell/`, `engine/`, `skills/`, `base/overlay/`, and configuration files.

**Acceptance Criteria:**

- [ ] OS source is a Git repository at `/usr/src/levsha/`.
- [ ] Every self-modification creates a Git commit before deploy.
- [ ] User can view commit history via chat.
- [ ] User can view diffs between versions via chat.

### 3.3 Safe Restart & Rollback (SI-03)

| Field | Value |
|-------|-------|
| **ID** | SI-03 |
| **Priority** | P0 |
| **Requirement** | Before any self-modification deployment, the system creates a recovery checkpoint. Failed boots automatically revert. |

**Details:**

The deploy flow (triggered by `deploy_build`):

1. **Build** — Compile the modified component.
2. **Test** — Run automated tests on the built artifact.
3. **Checkpoint** — Copy current working binaries to a backup location.
4. **Show diff** — Display the changes to the user and ask for confirmation.
5. **Deploy** — Copy new binaries to their runtime locations.
6. **Restart** — Restart the affected systemd service (or full reboot for kernel/init changes).
7. **Health check** — After restart, verify the component is healthy (responds within timeout).
8. **Rollback on failure** — If health check fails, automatically restore from checkpoint and restart.

Checkpoint locations:
- `/var/lib/levsha/checkpoints/` — stores previous binaries and configs.
- Each checkpoint is timestamped and linked to its Git commit hash.

**Acceptance Criteria:**

- [ ] Checkpoint is created before every deployment.
- [ ] Failed health check triggers automatic rollback.
- [ ] User can manually rollback: "undo the last change", "rollback to previous version".
- [ ] Rollback restores both binaries and configuration.
- [ ] Conversation history is preserved across restarts.

### 3.4 Component-Scoped Restarts (SI-04)

| Field | Value |
|-------|-------|
| **ID** | SI-04 |
| **Priority** | P1 |
| **Requirement** | Self-modifications restart only the affected component, not the entire system, when possible. |

**Details:**

| Modification Target | Restart Scope |
|---------------------|---------------|
| Chat Shell (L3) — CSS, layout, widgets | Restart Chat Shell process only. Conversation preserved via SQLite. |
| Chat Shell (L3) — structural changes | Restart Chat Shell process. |
| Engine (L2) — API client, tool calling | Restart the full `levsha-chat` process (engine is embedded). |
| Skill definitions | Hot-reload skill files without restart. |
| Base system configs | Depends on config type. Network: `systemctl restart NetworkManager`. Fonts: no restart needed. |
| Kernel / init | Full system reboot with user consent. |

**Acceptance Criteria:**

- [ ] CSS/style changes restart only the Chat Shell process.
- [ ] Skill definition changes are hot-reloaded.
- [ ] Full reboot requires explicit user consent.
- [ ] Conversation history is preserved across component restarts.

### 3.5 Theming via Self-Improvement (SI-05)

| Field | Value |
|-------|-------|
| **ID** | SI-05 |
| **Priority** | P1 |
| **Requirement** | Theme changes (dark/light mode, colors, fonts) are implemented as self-modifications to the Chat Shell CSS and code. |

**Details:**

Theming is not a standalone feature with a theme engine. Instead, when a user asks to change the theme, the self-improvement system:

1. Spawns the coding agent with a prompt targeting `style.css` and relevant rendering code.
2. The agent reads the current styles, generates modifications for the requested theme change.
3. The agent's work streams in the split-view so the user can see progress.
4. On agent completion, the standard deploy flow applies (build, checkpoint, confirm, deploy, restart).

Example interactions:
- "Switch to dark mode" → agent modifies CSS color variables.
- "Make the font bigger" → agent modifies CSS font-size rules.
- "I want a blue accent color" → agent modifies CSS accent variables.
- "The contrast is too low" → agent adjusts text/background colors in CSS.

**Acceptance Criteria:**

- [ ] User can request theme changes via natural language.
- [ ] Theme changes are handled by the coding agent modifying Chat Shell CSS.
- [ ] Changes follow the standard self-improvement flow (agent session, diff, confirm, deploy).
- [ ] Both dark and light mode are achievable.

---

## 4. What Self-Improvement Can Modify

| Target | Examples |
|--------|----------|
| **Chat Shell (L3)** | Fix rendering bugs, add UI features, improve animations, change layout, add new widget types, theming. |
| **Intelligence Engine (L2)** | Improve prompt routing, optimize tool calling, fix inference pipeline bugs. |
| **Skill System** | Create entirely new skills on demand, fix broken skill definitions. |
| **System Configuration (L1)** | Fix driver issues, adjust power management, tune network settings. |
| **Itself** | The coding agent can improve the self-improvement skill's own prompts and configuration. |

---

## 5. Safety Model

| Safety Mechanism | Description |
|------------------|-------------|
| **User consent** | Every self-modification requires explicit user approval before deploying. The system shows the final diff and asks for confirmation. |
| **Git versioning** | All changes are committed. User can roll back to any previous state. |
| **Checkpoint before restart** | Before any restart, working binaries are backed up. |
| **Automatic rollback** | If a build fails to start or health check fails, the system automatically reverts. |
| **Build isolation** | Builds happen in a separate directory. Runtime binaries are only replaced on successful build + user approval. |
| **Agent sandboxing** | The coding agent subprocess runs within the source tree only. It cannot modify runtime binaries or system files directly — that is handled exclusively by the `deploy_build` flow. |
| **Destructive command confirmation** | Any self-modification that modifies critical system files goes through the existing destructive command confirmation flow. |

---

## 6. Source Tree Layout

```
/usr/src/levsha/
├── chat-shell/        # L3 source
├── engine/            # L2 source
├── skills/            # Skill definitions
│   └── built-in/
├── base/
│   └── overlay/       # System configs
├── Cargo.toml         # Rust workspace
├── Makefile           # Build system
└── .git/              # Version history
```

---

## 7. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Intelligence Engine (03) | Upstream | Coding agent tools are dispatched by the engine. |
| Chat Shell (02) | Target | Primary modification target for UI improvements and theming. |
| Skills System (04) | Upstream | Self-improvement uses the skill framework. |
| Persistence (07) | Upstream | Conversation must survive component restarts. |
| Base System (01) | Target | System config modifications affect L1. |
| Git (new dep) | Infrastructure | Required for version tracking. Must be in the base image. |
| Rust toolchain (new dep) | Infrastructure | Required for building chat-shell and engine from source on-device. |
| Claude Code or OpenCode (new dep) | Infrastructure | External coding agent binary. At least one must be installed. |

---

## 8. New System Requirements

The self-improvement system adds significant requirements to the base image:

| Package | Purpose | Approximate Size |
|---------|---------|-----------------|
| `git` | Version control for source tree | ~30 MB |
| `rustup` + Rust toolchain | Compile Rust source on-device | ~500 MB |
| `gcc`, `make` | Build toolchain | ~200 MB |
| `gtk4-devel`, `libadwaita-devel` | GTK4 development headers | ~100 MB |
| `pkg-config` | Build dependency resolution | ~1 MB |
| `claude-code` or `opencode` | External coding agent | ~50 MB |

> **Note:** `ripgrep` is no longer a separate dependency — the coding agent bundles its own search tools.

**Impact:** The base image grows from ~1.5 GB to ~2.5 GB. VM storage requirement increases from 8 GB to 16 GB minimum.

**Alternative (P2 consideration):** Cross-compile on a remote build server to avoid shipping a full toolchain. The deploy_build tool would upload source to a build service and download artifacts. This significantly reduces disk requirements but adds network dependency.

---

## 9. Out of Scope

- Remote build servers (future optimization).
- Automated self-improvement without user involvement (always requires confirmation).
- Self-improvement of the kernel itself (too risky, future phase).
- A/B testing of modifications.
- Multi-user approval workflows.
- Reimplementing source editing tools (file read/write/search) — delegated to the coding agent.
