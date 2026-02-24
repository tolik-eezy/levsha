# Levsha OS Architecture Overview

**Version:** 0.1.0 (MVP)

---

## The Chat Is the Computer

Levsha OS is a minimal Linux distribution where the entire user interface is a single full-screen chat. There are no windows, no desktop environment, no file manager. The user communicates with the system in natural language, and the system executes commands through an LLM-powered intelligence engine.

---

## 4-Layer Architecture

```
┌─────────────────────────────────────────────────┐
│  L3 — Chat Shell                                │
│  GTK4 + libadwaita full-screen GUI (Rust)       │
│  Message rendering, streaming, input, search     │
├────────────────┬────────────────────────────────┤
│                │  tokio::mpsc channels           │
│                │  (ShellToEngine / EngineToShell) │
├────────────────┴────────────────────────────────┤
│  L2 — Intelligence Engine                       │
│  Rust library: API client, skill loader,        │
│  tool executor, context manager, risk classifier │
├─────────────────────────────────────────────────┤
│  L1 — Base System                               │
│  Fedora 41 minimal, systemd, cage (Wayland),    │
│  NetworkManager, auto-login, VT lockdown        │
├─────────────────────────────────────────────────┤
│  L0 — Kernel                                    │
│  Linux kernel (Fedora default, minimal config)  │
└─────────────────────────────────────────────────┘
```

---

## Layer Descriptions

### L0 — Kernel

The stock Fedora 41 Linux kernel with no custom configuration. Provides hardware abstraction, process scheduling, memory management, and device drivers.

### L1 — Base System

A minimal Fedora 41 installation with only the packages required to run the chat shell:

- **Init:** systemd manages all services and the boot sequence
- **Compositor:** cage -- a wlroots-based Wayland kiosk compositor (~2 MB) that runs a single application full-screen with zero configuration
- **Networking:** NetworkManager for automatic network setup (DHCP, DNS)
- **Auto-login:** getty@tty1 is configured to auto-login the `levsha` user via a systemd drop-in
- **Session launcher:** `levsha-session` (a 2-line shell script) invokes `cage -- /usr/bin/levsha-chat`
- **Boot splash:** Custom Plymouth theme with Levsha branding
- **Filesystem overlay:** Production configs deployed via `base/overlay/` (systemd units, sudoers, fonts, config.toml)

### L2 — Intelligence Engine

A Rust library (`levsha-engine`) embedded directly in the chat shell binary. It manages all LLM interaction and system command execution:

| Subsystem | Source | Responsibility |
|-----------|--------|----------------|
| API Client | `engine/src/api_client.rs` | Anthropic Claude API, SSE streaming, exponential retry |
| Skill Loader | `engine/src/skill_loader.rs` | Discovers skills from disk, loads YAML manifests and JSON tool definitions |
| Tool Executor | `engine/src/tool_executor.rs` | Renders command templates, executes via `tokio::process::Command`, captures output |
| Context Builder | `engine/src/context.rs` | Assembles system prompt + skill prompts + conversation history within token budget |
| Risk Classifier | `engine/src/risk_classifier.rs` | Regex-based destructive command detection (High / Medium / Low risk levels) |
| History | `engine/src/history.rs` | SQLite persistence with WAL mode at `/var/lib/levsha/history.db` |
| Config | `engine/src/config.rs` | TOML configuration loading with dev-mode fallback |

The engine communicates with L3 via `tokio::mpsc` channels (3 message types Shell-to-Engine, 7 message types Engine-to-Shell). See [L2-L3 Protocol](l2-l3-protocol.md) for the full specification.

### L3 — Chat Shell

A GTK4 + libadwaita application written in Rust, running full-screen inside cage:

| Component | Source | Responsibility |
|-----------|--------|----------------|
| Window | `chat-shell/src/window.rs` | Full-screen window, engine spawning, message dispatch |
| Chat View | `chat-shell/src/chat_view.rs` | Scrollable message list, typing indicator, search integration |
| Message Widget | `chat-shell/src/message_widget/mod.rs` | Rich rendering: markdown (bold, italic, inline code), tables, ordered/unordered lists, fenced code blocks |
| Syntax Highlighting | `chat-shell/src/message_widget/syntax.rs` | Language-aware highlighting for Python, Rust, Bash, JavaScript, Go, JSON, YAML |
| Input Bar | `chat-shell/src/input_bar.rs` | Multi-line text input, message history navigation (Up/Down), send on Enter |
| Status Bar | `chat-shell/src/status_bar.rs` | Connection status, clock, LLM model indicator |
| Search | `chat-shell/src/search.rs` | Ctrl+F search overlay with match highlighting |

Visual theme: warm parchment background (`#FBF8F3`) with copper accents (`#C67A52`).

---

## Data Flow

```
User types message
        │
        ▼
  ┌──────────┐  ShellToEngine::UserMessage   ┌──────────────┐
  │ Chat     │ ─────────────────────────────► │ Intelligence │
  │ Shell    │                                │ Engine       │
  │ (L3)     │  EngineToShell::StreamChunk    │ (L2)         │
  │          │ ◄───────────────────────────── │              │
  └──────────┘  EngineToShell::StreamEnd      └──────┬───────┘
                                                     │
                                              ┌──────┴───────┐
                                              │ Claude API   │
                                              │ (Anthropic)  │
                                              │ SSE stream   │
                                              └──────┬───────┘
                                                     │
                                              ┌──────┴───────┐
                                              │ Tool         │
                                              │ Execution    │
                                              │ (L1 shell)   │
                                              └──────────────┘
```

1. User types a message and presses Enter
2. Chat Shell sends `ShellToEngine::UserMessage` via mpsc channel
3. Engine persists the message in SQLite
4. Engine builds context (system prompt + skill prompts + recent history within token budget)
5. Engine sends a streaming request to the Anthropic Claude API
6. Claude responds with either text (streamed as SSE) or a `tool_use` block
7. For text: engine forwards `StreamChunk` messages to the shell for live rendering
8. For tool calls: engine renders the command template, checks the risk classifier, executes the command, and sends the result back to Claude for a follow-up response
9. When the response is complete, engine sends `StreamEnd`

Tool calls may trigger the destructive command guard -- see below.

---

## Destructive Command Confirmation

The risk classifier (`engine/src/risk_classifier.rs`) scans rendered shell commands against regex patterns before execution:

| Risk Level | Examples | Action |
|------------|----------|--------|
| **High** | `rm -rf /`, `mkfs.*`, `dd if=`, `reboot`, `shutdown`, `fdisk`, `parted` | Confirmation required |
| **Medium** | `dnf remove`, `systemctl stop sshd` | Confirmation required |
| **Low** | Redirects to block devices (`> /dev/sda`) | Confirmation required |

When a command matches:

1. Engine sends `ConfirmRequest` to the shell (with the command and a description)
2. Shell displays a confirmation widget with Approve/Reject buttons
3. User clicks Approve or Reject
4. Shell sends `ConfirmResponse` back to the engine
5. Engine executes the command only if approved

---

## Skills System

Skills are self-contained bundles that define what tools the LLM can use. Each skill contains:

- `skill.yaml` -- manifest with metadata and tool references
- `prompts/*.md` -- instructions injected into the system prompt
- `tools/*.json` -- tool definitions (Anthropic-compatible JSON Schema + execution block)

**Built-in skills (MVP):**

| Skill | Tools | Description |
|-------|-------|-------------|
| `package-manager` | `package_install`, `package_remove`, `package_search`, `package_update`, `package_list` (5 tools) | dnf package management |
| `sysinfo` | `uptime`, `memory`, `disk_usage`, `cpu_info`, `network`, `processes` (6 tools) | System information queries |

Total: **11 tools** across 2 skills.

Tool definitions use a split design: `name`, `description`, and `input_schema` are sent to the Anthropic API; the `execution` block (command template, timeout, run_as) stays on the engine side. See [Tool Schema](tool-schema.md) for the full format.

---

## Boot Chain

```
BIOS/UEFI
    │
    ▼
GRUB (timeout=0, hidden menu)
    │
    ▼
Linux kernel (Fedora 41 default)
    │
    ▼
systemd (default target)
    │
    ├── Plymouth splash (custom Levsha theme)
    │
    ▼
getty@tty1 (autologin → levsha user)
    │
    ▼
.bash_profile → exec levsha-session
    │
    ▼
levsha-session → exec cage -- /usr/bin/levsha-chat
    │
    ▼
levsha-chat (GTK4 window, full-screen)
    │
    ├── Creates tokio::mpsc channels
    ├── Spawns Engine on background thread (tokio runtime)
    │
    ▼
Engine init:
    ├── Loads config (/etc/levsha/config.toml or config.dev.toml)
    ├── Opens SQLite history database
    ├── Loads skills from /usr/share/levsha/skills/
    ├── Connects to Anthropic API
    ├── Sends ConnectionStatus to shell
    ├── Restores history or sends welcome message
    └── Enters main event loop
```

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Base distro | Fedora 41 | Best GTK4/libadwaita support, mature Wayland, polished font rendering |
| Compositor | cage (wlroots) | Minimal kiosk compositor (~2 MB), designed for single-app use, zero config |
| GUI toolkit | GTK4 0.9 + libadwaita 0.7 (Rust) | Native Wayland, dark mode support, excellent font rendering on Fedora |
| LLM backend | Anthropic Claude API | SSE streaming, native tool use support, large context window |
| IPC | tokio::mpsc channels | Zero-copy, no serialization overhead, single-binary deployment |
| Persistence | SQLite (WAL mode) | Crash-safe, single-file, no server process, portable |
| Build system | Fedora container + Makefile | Reproducible builds, GTK4 dev deps in container, works from macOS host |
| ISO tooling | livemedia-creator / lorax | Standard Fedora ISO generation with kickstart files |
| VM testing | QEMU | Cross-platform, supports both aarch64 (ARM Mac with HVF) and x86_64 |

---

## Key Design Decisions

| Decision | Rationale | ADR |
|----------|-----------|-----|
| cage as Wayland compositor | Minimal kiosk, ~2 MB, zero config, perfect for single-app OS | [ADR-001](adr-001-cage-compositor.md) |
| Engine embedded as Rust library | Single binary, zero-copy IPC, simplest deployment | [ADR-002](adr-002-embedded-engine.md) |
| GTK4 + libadwaita for GUI | Best Fedora integration, mature ecosystem, excellent font rendering | [ADR-003](adr-003-gtk4-gui.md) |

---

## Related Documents

- [L2-L3 Protocol](l2-l3-protocol.md) -- Full IPC protocol specification between Chat Shell and Intelligence Engine
- [Tool Schema](tool-schema.md) -- Tool definition format for skills
- [ADR-001: cage compositor](adr-001-cage-compositor.md)
- [ADR-002: Embedded engine](adr-002-embedded-engine.md)
- [ADR-003: GTK4 GUI](adr-003-gtk4-gui.md)
