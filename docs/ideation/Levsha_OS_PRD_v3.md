# Levsha OS — Product Requirements Document

**The Chat Is the Computer**

Version 3.0 · February 2026 · Confidential

---

## 1. Executive Summary

Levsha OS is a minimal Linux distribution where the entire user interface is a single, beautiful, full-screen chat. There are no traditional windows, no desktop icons, no file manager, and no application launcher. The chat is the computer.

Every interaction with the system happens through natural language conversation. Applications do not exist as standalone GUI programs. Instead, they are **skills**: modular capabilities that the system acquires and integrates into the chat context. A chat session in Levsha OS is the equivalent of a window or process in a traditional OS.

The system is designed to be **literally self-improving** — capable of reading its own source code, writing patches, rebuilding, and restarting with new capabilities or bug fixes. This is achieved through a built-in development skill powered by Claude Code or OpenCode. Self-improvement is the long-term vision; the MVP focuses on proving the core thesis: a usable, beautiful computer where the chat is the only interface.

> **Core Philosophy:** The chat is not a layer on top of the OS. The chat IS the OS. Everything below it — the kernel, drivers, filesystem — exists solely to serve the conversation.

---

## 2. Vision & Guiding Principles

### 2.1 Vision Statement

To build an operating system so intuitive that the only thing a user needs to know is how to type what they want — and so alive that it eventually evolves to meet them halfway. The OS disappears. Only the conversation remains.

### 2.2 Guiding Principles

| Principle | Description |
|---|---|
| **Chat-First, Chat-Only** | The chat is the sole interface. There is no fallback shell, no TTY, no plan B. If it can't be done in chat, it can't be done yet. |
| **Skills, Not Apps** | Software is not installed as a standalone application. It is acquired as a skill that extends what the chat can do. |
| **Session = Process** | Each chat session is the equivalent of a process or window. Multi-tasking means multiple concurrent sessions (future). |
| **Literally Self-Improving** | The OS can modify its own source code, rebuild, and restart. Bugs can be self-patched. Features can be built on demand. (Phase 2) |
| **Radically Minimal** | The base system has only what is needed to boot, render the chat GUI, connect to a network, and talk to an LLM. Everything else is a skill. |
| **Beautiful by Default** | The GUI is not an afterthought. It must be visually polished, calm, and premium-feeling from day one. Less functionality is acceptable; ugly UI is not. |
| **AI Quality is Non-Negotiable** | Bad AI responses destroy the entire UX. The system uses a cloud API backend to ensure high-quality responses. Local models are a future option. |

---

## 3. Target Users

### 3.1 Primary Audience

- Power users and developers who live in the terminal and want a faster, more intelligent command-line experience.
- AI/ML enthusiasts who want a system built around language model interaction from the ground up.
- Minimalists and tinkerers who find traditional desktop environments bloated and prefer to build their environment from scratch.

### 3.2 Secondary Audience

- Non-technical users who are comfortable with chat interfaces (messaging apps, ChatGPT) but intimidated by traditional OS paradigms.
- Embedded/kiosk/single-purpose device operators who need a lightweight, task-oriented system.

---

## 4. System Architecture

### 4.1 Architectural Layers

| Layer | Component | Responsibility |
|---|---|---|
| **L0 — Kernel** | Linux kernel (minimal config) | Hardware abstraction, process scheduling, memory management, device drivers. |
| **L1 — Base System** | Init + core userland | Boot sequence, networking (auto-configured ethernet in MVP), filesystem, audio/video subsystem, Wayland compositor. |
| **L2 — Intelligence Engine** | LLM runtime (API-based) | Prompt routing to cloud API (Anthropic Claude), tool/function calling, skill orchestration, memory/context persistence. |
| **L3 — Chat Shell** | Chat GUI (Wayland native) | The user-facing chat interface rendered as a polished, full-screen Wayland GUI application. This is the entire UX. |

> **Key Architectural Decision:** The LLM engine (L2) sits between the base system and the UI. It is the orchestration layer — it interprets user intent, dispatches system calls, manages skills, and composes responses. In a traditional OS, this role is played by a shell (bash) or a window manager. In Levsha OS, the LLM replaces both.

### 4.2 The Chat Shell (L3) — GUI

The Chat Shell is the only graphical surface in the system. It is a custom Wayland application with one purpose: render a beautiful, functional chat.

**Design mandate:** The Chat Shell must feel like a premium product from first launch. Visual polish, smooth animations, considered typography, and a calm color palette are non-negotiable. It is better to ship fewer features with a finished feel than more features that look rough.

Core properties:

- Full-screen chat interface with conversation history, input field, and a minimal status bar (time, connection status, AI backend indicator).
- No window decorations, no taskbar, no system tray. The chat occupies the entire screen.
- Smooth scrolling, fluid token-by-token streaming, and subtle animations for state transitions.
- Keyboard-driven by default. Mouse/touch supported.
- Typography: carefully chosen default font (monospace for code, proportional for conversation), generous line height, comfortable contrast.
- Supports rendering inside chat: styled text, code blocks with syntax highlighting, tables, and progress indicators for long-running tasks.
- Dark mode by default. Light mode available.

### 4.3 The Intelligence Engine (L2)

In the MVP, the intelligence engine uses a **cloud API backend with a hardcoded API key** (for development purposes). This ensures the highest possible AI quality from day one — there is no local model in Phase 1.

Responsibilities:

- **Intent Parsing:** Understanding what the user wants from natural language input.
- **Skill Dispatch:** Routing requests to the appropriate built-in skill.
- **Tool Use / Function Calling:** Executing system commands and returning real output. All tools run with full system access — no sandboxing.
- **Context Management:** Maintaining conversation history and system state across the session.

### 4.4 Interaction Flow

```
User types message
       │
       ▼
┌─────────────────┐
│   Chat Shell    │  ← L3: renders GUI, captures input
│   (Wayland)     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Intelligence   │  ← L2: parses intent, selects skill,
│     Engine      │     builds tool calls, sends to
│  (cloud API)    │     cloud API
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Base System    │  ← L1: executes commands, returns output
│  (Linux)        │
└────────┬────────┘
         │
         ▼
   Result flows back up through L2 → L3 → screen
```

---

## 5. Core Concepts

### 5.1 Skills (Not Apps)

In traditional operating systems, software is packaged as an application with its own window, menus, and lifecycle. In Levsha OS, software is packaged as a **skill**. A skill is a bundle that contains:

| Component | Description |
|---|---|
| System Prompt Fragment | Instructions injected into the LLM context when the skill is active. |
| Tool Definitions | Function-calling schemas that the LLM can invoke. |
| Dependencies | System packages, libraries, or binaries the skill requires. |
| Assets | Static files, models, configs bundled with the skill. |
| Manifest | Metadata: name, version, author, required hardware, conflicts. |

Skills run with **full system access**. There is no sandbox, no permission model, no capability restrictions. A skill can do anything the OS can do.

#### Skill Manifest Example

```yaml
# skill.yaml
name: image-editor
version: 1.2.0
description: "Edit, resize, crop, and filter images"
author: levsha-community
requires:
  packages:
    - imagemagick
    - libheif
  hardware:
    gpu: optional
conflicts: []
prompt: prompts/image-editor.md
tools:
  - tools/crop.json
  - tools/resize.json
  - tools/filter.json
assets:
  - assets/presets/
```

In the MVP, only built-in skills exist. Skill install/remove from external repositories is Phase 2.

### 5.2 Chat Sessions as Processes

A chat session in Levsha OS is the analog of a window or process in a traditional OS:

| Traditional OS | Levsha OS |
|---|---|
| Open an application | Start a new chat session with a skill context |
| Window | Chat session (with its own conversation thread) |
| Alt-Tab between windows | Switch between chat sessions (future) |
| Process (background) | Detached chat session running a long task (future) |
| Notifications | Inline messages or badges on background sessions (future) |
| Kill a process | "Stop that task" or close the session |

In the MVP, only a single session exists. The entire system is one continuous conversation.

### 5.3 Self-Improvement: The OS That Rewrites Itself (Phase 2)

This is the defining long-term feature of Levsha OS and what separates it from every other operating system.

Levsha OS will ship with a **built-in development skill** powered by Claude Code (or OpenCode as an open-source alternative). This skill has full access to the OS's own source code, build system, and service management. It can:

1. **Read its own source code** — the Chat Shell, the intelligence engine, skill definitions, configuration, build scripts — everything.
2. **Diagnose bugs** — when something goes wrong, the user can describe the issue and the system examines its own code, logs, and behavior to identify the root cause.
3. **Write patches** — using Claude Code / OpenCode, the system generates code changes to fix bugs or add features.
4. **Build itself** — the system runs its own build pipeline on the modified source.
5. **Test** — run automated tests or a smoke check to verify the change doesn't break critical functionality.
6. **Hot-restart** — restart the affected component with the new code. If the change affects the core system, a full reboot is triggered with user consent.

#### Self-Improvement Flow (Phase 2)

```
User: "The font rendering looks blurry on my display"
         │
         ▼
   Intelligence Engine identifies this as a self-improvement request
         │
         ▼
   Development Skill (Claude Code) activated
         │
         ▼
   Reads Chat Shell rendering source code
         │
         ▼
   Identifies the issue, generates a patch
         │
         ▼
   Builds the modified Chat Shell
         │
         ▼
   Shows diff to user, asks for confirmation
         │
         ▼
   User approves → Chat Shell restarts with the fix.
   Conversation history is preserved.
```

#### Safety Model (Phase 2)

- **User consent:** Every self-modification requires explicit user approval before applying. The system shows the diff and asks for confirmation.
- **Git-based versioning:** All source modifications are committed to a local Git repository. The user can roll back to any previous state.
- **Snapshot before restart:** Before any restart, the system creates a recovery checkpoint. If the new build fails to boot, the system automatically reverts.

#### What Self-Improvement Can Modify

| Target | Examples |
|---|---|
| **Chat Shell (L3)** | Fix rendering bugs, add UI features, improve animations, change layout, add new widget types, theming. |
| **Intelligence Engine (L2)** | Improve prompt routing, optimize tool calling, fix inference pipeline bugs. |
| **Skill System** | Create entirely new skills on demand, fix broken skill definitions. |
| **System Configuration (L1)** | Fix driver issues, adjust power management, tune network settings. |
| **Itself** | The development skill can improve its own prompts and patching strategies. |

> **Note:** Theming (dark/light mode switching, color palette changes, font adjustments) is not a standalone feature. It falls under self-improvement — the OS modifies its own UI code to apply theme changes. In the MVP, only a single default theme ships.

---

## 6. MVP Scope (Phase 1)

The MVP is brutally minimal. It proves one thing: a usable, beautiful computer where the chat is the only interface.

### 6.1 Development Environment

The MVP runs as a **virtual machine** (VirtualBox or QEMU). It does not need to run on bare metal. Network connectivity is provided via virtualized ethernet — there is no Wi-Fi configuration.

### 6.2 What's In

| Feature | Details |
|---|---|
| **Bootable ISO** | A minimal Linux image that boots directly into the Chat Shell GUI. Runs in VirtualBox / QEMU. |
| **Beautiful Chat GUI** | A full-screen, polished Wayland-native chat interface with smooth animations, considered typography, and a premium feel. Dark mode default. This is the product — it must be stunning. |
| **Single Chat Session** | One full-screen conversation. No session management, no tabs, no splits. |
| **Cloud API Backend** | Hardcoded API key (Anthropic Claude) for development. No local model, no model selection. Just works. |
| **Package Manager Skill (built-in)** | Install, remove, update, and search system packages via chat. |
| **System Info Skill (built-in)** | Query disk usage, memory, CPU, uptime, network status, running processes. |
| **Persistent Conversation** | Conversation history survives reboot (local SQLite). |
| **Streaming Responses** | Token-by-token output with smooth rendering. |
| **Destructive Command Confirmation** | Dangerous operations (rm -rf, mkfs, dd) require explicit user confirmation via a visually distinct prompt. |

### 6.3 What's Out (Phase 2+)

| Feature | Phase |
|---|---|
| Self-improvement (Claude Code / OpenCode) | Phase 2 |
| Git-based self-versioning and rollback | Phase 2 |
| Skill install/remove from repositories | Phase 2 |
| Split-view content rendering | Phase 2 |
| Theming via chat (part of self-improvement) | Phase 2 |
| Multi-session / split-session | Phase 2 |
| Local LLM support | Phase 2 |
| User-configurable API key | Phase 2 |
| Voice input/output | Phase 3 |
| GPU-accelerated inference | Phase 3 |
| Multi-user support | Phase 3 |
| Skill marketplace | Phase 3 |
| Filesystem skill (ls, cp, mv, rm, cat, edit) | Separate project, Phase 2 skill |
| Network configuration skill (Wi-Fi, IP config) | Separate project, Phase 2 skill |
| Text editing skill | Separate project, Phase 2 skill |
| Web browser skill | Separate project, Phase 2+ skill |
| Image editing skill | Separate project, Phase 2+ skill |

### 6.4 First Boot Experience (MVP)

The first boot is simple. There is no setup wizard. The system boots, the chat appears, and it's ready.

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│                        ◆ Levsha OS                           │
│                                                              │
│  Welcome. I'm your operating system.                         │
│  Everything you need — just ask.                             │
│                                                              │
│  I can manage your packages and tell you about your system.  │
│  More skills are coming soon.                                │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐     │
│  │ _                                                   │     │
│  └─────────────────────────────────────────────────────┘     │
│                                                              │
│  ▸ claude-sonnet  ▸ connected  ▸ 14:32                       │
└──────────────────────────────────────────────────────────────┘
```

No language selection. No Wi-Fi. No API key entry. No user name prompt. It just works.

---

## 7. Detailed Functional Requirements

### 7.1 Boot & Initialization

| ID | Requirement | Priority |
|---|---|---|
| BR-01 | System boots from ISO in VirtualBox/QEMU to Chat Shell GUI in under 30 seconds. | P0 |
| BR-02 | No login screen. Single-user, auto-login directly into the chat GUI. | P0 |
| BR-03 | Network is auto-configured via DHCP on virtualized ethernet. No user interaction required. | P0 |
| BR-04 | API key is hardcoded in the system configuration for development. No user-facing key management. | P0 |
| BR-05 | On first launch, a brief welcome message introduces the system and its current capabilities. No multi-step setup wizard. | P0 |
| BR-06 | Boot splash shows Levsha OS logo with a polished, animated progress indicator consistent with the GUI's visual language. | P1 |

### 7.2 Chat Shell GUI

| ID | Requirement | Priority |
|---|---|---|
| CS-01 | Full-screen Wayland-native chat GUI with scrollable history, input field at bottom, and a minimal status bar (time, connection status, AI backend indicator). | P0 |
| CS-02 | Visual design must meet a high bar: smooth animations (scrolling, message appearance, transitions), careful typography (proportional for text, monospace for code), comfortable spacing, and a cohesive color palette. | P0 |
| CS-03 | Dark mode by default. Single theme in MVP. | P0 |
| CS-04 | Input supports multi-line text entry and standard shortcuts (Ctrl+C to cancel, Ctrl+L to clear, Up for history). | P0 |
| CS-05 | System responses render: styled text, code blocks with syntax highlighting, tables, and progress indicators for long-running tasks. | P0 |
| CS-06 | Streaming responses: LLM output appears token-by-token with smooth rendering. | P0 |
| CS-07 | If the AI backend is unreachable (network issue, API error), the Chat Shell displays a clear, styled error message with a retry option. There is no fallback to a terminal or TTY. | P0 |
| CS-08 | Chat can be scrolled and searched (Ctrl+F). | P1 |

### 7.3 Intelligence Engine

| ID | Requirement | Priority |
|---|---|---|
| IE-01 | Connects to Anthropic Claude API using a hardcoded API key on boot. Ready to accept input immediately after Chat Shell appears. | P0 |
| IE-02 | Supports tool/function calling. LLM executes real system commands with full access and returns real output. | P0 |
| IE-03 | Destructive commands require explicit user confirmation displayed as a visually distinct prompt in the chat. | P0 |
| IE-04 | Context window includes: system prompt, active skill prompts, and recent conversation history (rolling window). | P0 |
| IE-05 | Graceful error handling: API failures, timeouts, and rate limits are surfaced as friendly messages in the chat with retry options. | P0 |

### 7.4 Built-In Skills (MVP)

Only two skills ship in the MVP. Both are built-in and cannot be removed.

#### 7.4.1 Package Manager Skill

| ID | Requirement | Priority |
|---|---|---|
| PK-01 | Install packages via chat (e.g., "install ffmpeg", "I need a C compiler"). | P0 |
| PK-02 | Remove packages via chat (e.g., "uninstall imagemagick"). | P0 |
| PK-03 | Search for packages (e.g., "is there a package for PDF editing?"). | P0 |
| PK-04 | Update all packages (e.g., "update everything"). | P0 |
| PK-05 | List installed packages (e.g., "what's installed?"). | P0 |
| PK-06 | Uses the base distro's native package manager (pacman/apt/dnf). | P0 |

#### 7.4.2 System Info Skill

| ID | Requirement | Priority |
|---|---|---|
| SY-01 | Report disk usage (e.g., "how much disk space do I have?"). | P0 |
| SY-02 | Report memory usage (e.g., "how much RAM is free?"). | P0 |
| SY-03 | Report CPU info and load (e.g., "what CPU is this?", "is the system busy?"). | P0 |
| SY-04 | Report uptime (e.g., "how long has the system been running?"). | P0 |
| SY-05 | Report network status (e.g., "am I connected?", "what's my IP?"). | P0 |
| SY-06 | List running processes (e.g., "what's running right now?"). | P0 |

### 7.5 Persistence

| ID | Requirement | Priority |
|---|---|---|
| PM-01 | Full conversation history is stored locally (SQLite) and survives reboot. | P0 |
| PM-02 | User can clear conversation history via chat ("clear history", "start fresh"). | P0 |

---

## 8. Non-Functional Requirements

### 8.1 Performance

- First token latency (API): under 1 second on a reasonable internet connection.
- Streaming token rendering: smooth and consistent, no visual jank.
- Full system memory footprint (OS + Chat Shell): under 1GB at idle (no local model in MVP).
- Boot to interactive chat GUI: under 30 seconds from cold start in VM.
- GUI frame rate: consistent 60fps for scrolling, animations, and transitions.

### 8.2 Reference Environment (MVP)

| Spec | Requirement |
|---|---|
| Hypervisor | VirtualBox 7+ or QEMU/KVM |
| VM CPU | 2+ vCPUs |
| VM RAM | 2 GB (no local model in MVP) |
| VM Storage | 8 GB virtual disk |
| VM Display | 1280×720 minimum, 1920×1080 recommended |
| VM Network | NAT or Bridged (ethernet, auto-DHCP) |
| Host Internet | Required (cloud API backend) |

### 8.3 Security

- All tools run with full system access. No sandboxing.
- Destructive operations require explicit confirmation.
- API key is hardcoded in config (development phase only). Production key management is Phase 2.
- No network telemetry beyond API calls to the configured LLM provider.

### 8.4 Reliability

- If the API is unreachable, the Chat Shell displays a clear error with retry. There is no fallback to a terminal.
- Conversation history is write-ahead logged; no data loss on power failure or VM crash.
- If the Chat Shell process crashes, systemd restarts it automatically.

---

## 9. Technology Stack

| Component | Choice | Rationale |
|---|---|---|
| **Base Distribution** | TBD — Fedora / Arch / Ubuntu minimal | Must be chosen for GUI toolkit support. See decision framework below. |
| **Init System** | systemd | Broad compatibility, reliable service management. |
| **Display Server** | Wayland (wlroots-based compositor) | Modern, lean, secure. No X11. |
| **Chat Shell Toolkit** | GTK4 + libadwaita / Iced (Rust) / Tauri | GTK4 + libadwaita gives the most polished look fastest. Iced is pure Rust. Tauri allows web tech for rapid UI iteration. |
| **Chat Shell Language** | Rust | Performance, single binary, excellent Wayland ecosystem. |
| **LLM Backend** | Anthropic Claude API (HTTP) | Direct HTTP calls. No SDK dependency. Hardcoded key in MVP. |
| **Persistence** | SQLite | Lightweight, zero-config, single-file. |
| **Package Manager** | Native to base distro (pacman / apt / dnf) | Whatever the base distro uses. |

### 9.1 Base Distro Decision Framework

The base distro choice is driven by **GUI capability**, since the Chat Shell's visual quality is the #1 MVP requirement:

| Criterion | Fedora | Arch | Ubuntu Minimal |
|---|---|---|---|
| GTK4 / libadwaita support | Excellent (primary platform) | Good (AUR) | Good |
| Wayland maturity | Excellent | Good | Good |
| Font rendering (subpixel, HiDPI) | Excellent | Manual config | Good |
| Package ecosystem | Large | Very large (AUR) | Very large |
| Minimal base size | Medium | Small | Medium |
| Custom ISO tooling | lorax / livemedia-creator | archiso | cubic / live-build |
| VM guest support | Excellent | Good | Excellent |

**Recommendation:** Fedora for fastest path to polished GUI with least friction. Arch if maximum control over the base is preferred. Alpine is eliminated due to musl compatibility issues with GUI toolkits.

---

## 10. User Experience Scenarios (MVP)

### 10.1 Package Management

```
> I need to work with some images, what tools are available?

Let me search for image-related packages.

  Found several options:
  ┌───────────────────────────────────────────────────┐
  │  imagemagick    — CLI image manipulation suite    │
  │  gimp           — full image editor (GUI)         │
  │  ffmpeg         — video/image conversion          │
  │  libvips        — fast image processing library   │
  └───────────────────────────────────────────────────┘

Want me to install any of these?

> install imagemagick and ffmpeg

  ◐ Installing imagemagick...
  ✓ imagemagick 7.1.1 installed

  ◐ Installing ffmpeg...
  ✓ ffmpeg 6.1.1 installed

Both are ready to use. Note: I don't have an image editing skill
yet, but you can use these tools via their command-line interfaces.
I can run commands for you if you tell me what you need.

> convert photo.png to jpeg

Running: convert photo.png photo.jpg

  ✓ Done. photo.jpg created (247 KB).
```

### 10.2 System Info

```
> how's the system doing?

  ┌─── System Status ────────────────────────────────┐
  │                                                   │
  │  CPU      2 vCPUs (x86_64) — 12% load            │
  │  Memory   847 MB / 2048 MB (41%)                  │
  │  Disk     2.1 GB / 8.0 GB used (26%)             │
  │  Uptime   1 hour, 23 minutes                      │
  │  Network  connected — 10.0.2.15 (NAT)            │
  │                                                   │
  └───────────────────────────────────────────────────┘

Everything looks healthy.

> what processes are using the most memory?

  ┌─── Top Processes by Memory ──────────────────────┐
  │  PID   NAME              MEM     CPU              │
  │  1142  levsha-chat       312 MB  3.2%             │
  │  892   systemd-journald   48 MB  0.1%             │
  │  1089  pipewire           23 MB  0.0%             │
  │  1     systemd            12 MB  0.0%             │
  └──────────────────────────────────────────────────┘
```

### 10.3 General Conversation

```
> explain what docker is

Docker is a platform for building and running containerized
applications. A container packages your code along with all its
dependencies into a single unit that runs consistently across
environments.

Think of it like a lightweight virtual machine, but instead of
emulating hardware, containers share the host kernel and isolate
only the userspace. This makes them much faster to start and more
efficient with resources.

The key concepts are images (read-only templates), containers
(running instances of images), and registries (where images are
stored and shared, like Docker Hub).

Want me to install Docker on this system?

> not now, thanks

No problem. Just ask whenever you're ready.
```

### 10.4 Error Handling

```
> update all packages

  ◐ Updating package database...
  ✗ Failed to reach package mirror.

  I can't connect to the package repository right now. This could
  be a temporary network issue. Want me to try again?

  [ Retry ]    [ Skip ]

> retry

  ◐ Retrying...
  ✓ Package database updated.
  ◐ Upgrading 7 packages...
  ✓ All packages up to date.
```

---

## 11. Phased Roadmap

### Phase 1 — MVP (Months 1–3)

**Goal:** Prove the core thesis — a beautiful, usable computer where chat is the only interface.

- Beautiful Chat Shell GUI (Wayland native, single session, dark mode)
- Bootable ISO for VirtualBox / QEMU
- Cloud API backend (Anthropic Claude, hardcoded key)
- Package manager skill (built-in)
- System info skill (built-in)
- Persistent conversation history
- Streaming responses
- Destructive command confirmation
- Boot splash

### Phase 2 — Self-Improvement & Skills (Months 4–8)

**Goal:** The OS becomes alive — it can modify itself and grow new capabilities.

- Self-improvement skill (Claude Code / OpenCode)
- Git-based self-versioning with rollback
- Theming via self-improvement (the OS modifies its own UI)
- Skill install/remove from Git-based repositories
- Split-view content rendering (inline media, web views, file previews)
- User-configurable API key
- Local LLM support (bundled model + llama.cpp)
- Intelligent backend routing (local vs. API)
- Additional core skills as separate projects:
  - Filesystem skill (ls, cp, mv, rm, cat, edit)
  - Network configuration skill
  - Text editor skill
  - Code editor skill

### Phase 3 — Multi-Session & Ecosystem (Months 9–14)

- Multiple concurrent chat sessions with split-view navigation
- Voice input/output (whisper.cpp + TTS)
- GPU-accelerated local inference
- Community skill repository with search
- Skill creation wizard ("help me build a skill for...")
- Multi-user support
- Notification system for background sessions

### Phase 4 — Scale & Expand (Months 15+)

- Bare-metal installation support
- ARM64 support (Raspberry Pi, Apple Silicon)
- Accessibility (screen reader, high contrast)
- Enterprise features (remote management, fleet provisioning)
- Federated skill sharing
- Model fine-tuning on local data (opt-in)

---

## 12. Risks & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| **API dependency makes MVP unusable offline** | Can't use the system without internet | High (by design) | Accepted for MVP. Local model support in Phase 2 eliminates this. Clear messaging: MVP requires internet. |
| **Hardcoded API key is a security risk** | Key exposure if ISO is shared | Medium | MVP is internal/development only. User-configurable key management in Phase 2. |
| **GUI development takes too long** | Delayed MVP, unpolished UI ships | Medium | Consider Tauri for rapid UI iteration, migrate to GTK4 for production. Keep scope ruthlessly minimal. |
| **Only two skills makes the system feel useless** | Users hit walls immediately | High | The LLM can still run arbitrary commands via tool calling — skills just make it better at specific domains. Package manager + sysinfo cover the basics. Phase 2 adds skills rapidly. |
| **LLM hallucinates dangerous commands** | Data loss, broken system | High | Destructive command confirmation. Risk-tier classification of system commands. |
| **Bad API latency ruins the experience** | Sluggish, frustrating UX | Medium | Streaming responses mask latency. Status indicators keep the user informed. |
| **Self-modification breaks the system (Phase 2)** | Unbootable OS, data loss | High | Git versioning, automatic rollback on failed boot, filesystem snapshots, mandatory diff review. |
| **User expects a full desktop OS** | Disappointment at limitations | High | Crystal-clear positioning. MVP is a proof of concept. The OS literally builds features you ask for (Phase 2). |

---

## 13. Success Metrics

| Metric | Target (MVP) |
|---|---|
| Boot to interactive chat GUI (in VM) | < 30 seconds |
| First-token latency (API) | < 1 second |
| System idle RAM (no local model) | < 1 GB |
| GUI frame rate | ≥ 60 fps |
| Successful command execution rate | > 85% without user correction |
| Package install success rate | > 95% |
| System info query accuracy | 100% |
| Conversation persists across reboot | 100% |
| Recovery from API failure | Graceful error + retry in < 3 seconds |

---

## 14. Naming & Identity

**Levsha** (Левша) — the Left-Hander — is a character from Nikolai Leskov's 1881 story about a craftsman from Tula who shoes a steel flea with tiny horseshoes invisible to the naked eye. He represents extraordinary craftsmanship at an impossibly small scale: doing incredible things with minimal resources.

Levsha OS embodies this spirit: a tiny, minimal system that does extraordinary things through conversation. It shoes the flea.

---

*Levsha OS — the chat is the computer.*
