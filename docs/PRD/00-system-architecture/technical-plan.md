# 00 — System Architecture: Technical Plan

**Module:** System Architecture (4-Layer Model)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Technology Choices Per Layer

| Layer | Technology | Version / Notes |
|---|---|---|
| **L0 — Kernel** | Linux (Fedora default) | Fedora 41+ kernel. No custom patches. |
| **L1 — Base System** | Fedora minimal, systemd, NetworkManager, PipeWire | Kickstart-based ISO build via lorax/livemedia-creator. |
| **L1 — Compositor** | cage or labwc (wlroots-based) | Single full-screen surface. Minimal config. |
| **L2 — Intelligence Engine** | Rust | Long-running daemon. HTTPS client (reqwest), JSON (serde), SQLite (rusqlite). |
| **L2 — API** | Anthropic Claude Messages API | Streaming enabled. Hardcoded API key. |
| **L2 — Persistence** | SQLite | Single file at `/var/lib/levsha/conversation.db`. WAL mode for crash safety. |
| **L3 — Chat Shell** | Rust + GTK4 + libadwaita | Wayland-native. Full-screen. Light theme (see theme.design.md). |
| **L3 — Alt option** | Rust + Iced | Pure Rust alternative if GTK4 integration proves problematic. |
| **IPC** | Unix domain socket | JSON-lines protocol. Socket at `/run/levsha/engine.sock`. |
| **ISO Tooling** | lorax / livemedia-creator | Fedora-native. Kickstart file defines the image. |
| **VM Testing** | QEMU/KVM + VirtualBox | Primary dev on QEMU. VirtualBox for compatibility testing. |

---

## 2. Implementation Steps

### Step 1: Skeleton Rust Workspace

Create a Cargo workspace with two crates:

```
Levsha.OS/
  chat-shell/          # L3 crate (binary)
    Cargo.toml
    src/main.rs
  engine/              # L2 crate (binary)
    Cargo.toml
    src/main.rs
  Cargo.toml           # workspace root
```

**Dependencies (chat-shell):** gtk4, libadwaita, tokio, serde, serde_json
**Dependencies (engine):** tokio, reqwest, serde, serde_json, rusqlite, tokio-stream

**Done when:** Both crates compile. Chat Shell opens a blank GTK4 window. Engine starts and logs "ready".

### Step 2: IPC Layer

Implement the Unix socket IPC between L3 and L2.

- Engine listens on `/run/levsha/engine.sock`
- Chat Shell connects on startup
- JSON-lines protocol (one JSON object per line, newline-delimited)
- Message types as defined in design.md section 4.1
- Tokio async on both sides

**Done when:** Chat Shell sends a `user_message` to Engine, Engine echoes it back, Chat Shell displays it.

### Step 3: Intelligence Engine Core

Implement the Claude API client in L2:

- HTTPS client using reqwest with streaming (SSE parsing)
- System prompt construction: base prompt + skill prompts
- Conversation history management (append user/assistant messages, rolling window)
- Tool schema registration (load from skill YAML/JSON files)
- Token forwarding to L3 via IPC as they arrive

**Done when:** User types a message in Chat Shell, Engine sends it to Claude API, streaming tokens appear in Chat Shell.

### Step 4: Tool Calling

Implement tool execution in L2:

- Parse `tool_use` blocks from API response
- Spawn subprocess via `tokio::process::Command`
- Capture stdout + stderr with 30-second timeout
- Send `tool_result` back to API in the next request
- Handle multi-turn tool use (API may request multiple tool calls)

**Done when:** User asks "what's my disk usage?" and gets real `df -h` output rendered in chat.

### Step 5: Built-In Skills

Create the two MVP skills:

```
skills/
  built-in/
    package-manager/
      skill.yaml        # manifest
      prompt.md          # system prompt fragment
      tools/
        install.json     # tool schema
        remove.json
        search.json
        update.json
        list.json
    system-info/
      skill.yaml
      prompt.md
      tools/
        disk.json
        memory.json
        cpu.json
        uptime.json
        network.json
        processes.json
```

**Done when:** Package install/remove and system queries work end-to-end through chat.

### Step 6: Destructive Command Confirmation

Implement the safety flow:

- L2 maintains a list of destructive command patterns (rm -rf, mkfs, dd, dnf remove, etc.)
- Before executing a matched command, L2 sends `confirm_request` to L3
- L3 renders a visually distinct confirmation prompt (different background, warning icon)
- User approves or denies; L3 sends `confirm_response` to L2
- L2 proceeds or cancels accordingly

**Done when:** "remove vim" triggers a confirmation prompt. Approving executes it. Denying cancels.

### Step 7: Persistence

Implement SQLite-backed conversation history:

- Schema: `messages(id, role, content, timestamp, metadata)`
- WAL mode for crash safety
- On Engine startup, load recent history into context window
- On Chat Shell startup, request history from Engine for display
- Support `clear_history` command

**Done when:** Conversation survives Engine restart. Chat Shell reconnects and shows full history.

### Step 8: Chat Shell Polish

Visual quality pass on L3:

- Typography: Inter or similar proportional font for conversation, JetBrains Mono for code
- Code blocks with syntax highlighting (tree-sitter or syntect)
- Table rendering
- Smooth scrolling with momentum
- Token-by-token animation (fade-in or typewriter effect)
- Status bar: time, connection indicator, model name
- Error display styling
- Progress indicators for long-running operations

**Done when:** Chat Shell meets the visual quality bar described in the PRD. Screenshots look premium.

### Step 9: Fedora Base System

Build the L1 layer:

- Kickstart file defining the minimal Fedora install
- Package list: kernel, systemd, NetworkManager, pipewire, mesa, cage/labwc, fonts
- Filesystem overlay: levsha-engine and levsha-chat-shell binaries, systemd units, config files
- Auto-login: systemd autologin on tty1, compositor auto-start
- Service units: `levsha-engine.service`, `levsha-chat-shell.service` (managed by compositor)
- API key placement: `/etc/levsha/config.toml` with hardcoded key

**Done when:** Kickstart file builds a bootable image via livemedia-creator.

### Step 10: ISO Build and VM Testing

Wire up the full build pipeline:

- `infra/build-iso.sh` script wrapping livemedia-creator
- QEMU launch script with appropriate flags (2 vCPU, 2GB RAM, virtio)
- Boot test: ISO boots to Chat Shell in under 30 seconds
- Smoke test: send a message, get a response, run a tool call

**Done when:** A single command produces an ISO that boots in QEMU to a working Chat Shell.

---

## 3. Dependencies Between Layers

```
  Step 1 (Workspace)
    |
    +---> Step 2 (IPC)
    |       |
    |       +---> Step 3 (Engine Core) ---> Step 4 (Tool Calling)
    |       |                                    |
    |       |                                    v
    |       |                              Step 5 (Skills)
    |       |                                    |
    |       |                                    v
    |       |                              Step 6 (Destructive Confirm)
    |       |
    |       +---> Step 7 (Persistence)
    |       |
    |       +---> Step 8 (Chat Shell Polish)
    |
    +---> Step 9 (Base System) ---> Step 10 (ISO Build)
```

**Critical path:** Steps 1 -> 2 -> 3 -> 4 -> 5 is the core functionality chain.

**Parallelizable work:**
- Step 8 (Chat Shell Polish) can proceed in parallel with Steps 4-6 once Step 2 is done.
- Step 9 (Base System) can begin once Step 1 is done — it doesn't depend on the Rust code being feature-complete.
- Step 7 (Persistence) can proceed in parallel with Steps 4-6 once Step 2 is done.

---

## 4. Build Order

| Phase | Steps | Agents | Duration Estimate |
|---|---|---|---|
| **Foundation** | 1 (Workspace), 2 (IPC) | chat-shell, engine | — |
| **Core Loop** | 3 (Engine Core), 4 (Tool Calling), 5 (Skills) | engine, skills-dev | — |
| **Safety & State** | 6 (Destructive Confirm), 7 (Persistence) | engine, security | — |
| **Polish** | 8 (Chat Shell Polish) | chat-shell | — |
| **System** | 9 (Base System), 10 (ISO Build) | base-system, infra | — |

---

## 5. Integration Strategy

### 5.1 Local Development

During development, L3 and L2 run as regular processes on the developer's machine (macOS or Linux). No VM needed for most development:

- L2 connects to the real Claude API
- L3 renders in a Wayland window (or XWayland on macOS with appropriate setup)
- IPC socket at a development path (e.g., `/tmp/levsha-dev/engine.sock`)
- No L1/L0 needed — the developer's OS serves as L1

### 5.2 Cross-Module Integration Checks

Individual module acceptance criteria verify components in isolation. **Integration checks verify that modules are wired together correctly** — that data flows between them, protocols are compatible, and the full stack behaves as a connected system.

See **`integration-checks.md`** for the complete integration verification matrix covering:

| Seam | Checks | Priority |
|------|--------|----------|
| L3 <-> L2 IPC | IC-01 through IC-07: socket connection, message delivery, streaming round-trip, cancel signal, error propagation, status updates, protocol compatibility | P0 |
| L2 <-> Claude API | IC-10 through IC-13: request construction, SSE-to-IPC bridge, tool call extraction, multi-turn tool calling | P0 |
| L2 <-> L1 Commands | IC-20 through IC-23: command execution path, package install end-to-end, destructive confirmation round-trip, timeout handling | P0 |
| Skills <-> Engine <-> API | IC-30 through IC-32: skill loading and prompt injection, tool invocation routing, unknown tool handling | P0 |
| Engine <-> SQLite <-> Chat Shell | IC-40 through IC-44: write and read back, streaming persistence, reboot survival, context window construction, clear history | P0 |
| Boot Chain (L1 -> L3 -> L2) | IC-50 through IC-54: full boot sequence, service dependencies, crash recovery (both sides), no TTY escape | P0 |
| Network Edge Cases | IC-60 through IC-62: offline boot, network loss during streaming, network recovery | P0/P1 |
| Visual Integration | IC-70 through IC-72: theme consistency, font rendering chain, Plymouth-to-Chat Shell transition | P1 |
| Performance Integration | IC-80 through IC-82: 60fps during streaming, memory budget, token append latency | P1 |
| End-to-End Smoke Tests | IC-90 through IC-96: first boot welcome, simple conversation, system query, package install, destructive confirm, persistence across reboot, error recovery | P0 |

**Key principle:** A module is not "done" when it passes its own tests — it is done when it passes its integration checks with adjacent modules.

### 5.3 VM Integration Testing

Once L3 and L2 are functional, integrate with L1:

1. Build the Fedora kickstart image with placeholder binaries
2. Replace placeholders with actual levsha-engine and levsha-chat-shell binaries
3. Boot in QEMU, verify auto-login -> compositor -> Chat Shell chain
4. Run smoke tests (message, tool call, persistence across reboot)
5. Run integration checks IC-50 through IC-96 (boot chain, network, visual, e2e)

### 5.4 Continuous Integration

- Each PR builds both Rust crates (`cargo build --release`)
- Clippy and rustfmt checks enforced
- Unit tests for IPC protocol, API client, tool execution, persistence
- **Integration checks IC-01 through IC-44** run on every PR (IPC, persistence, skills, tool execution — no VM required)
- ISO build triggered on merge to main (heavy, not on every PR)
- **Integration checks IC-50 through IC-96** run as a nightly QEMU boot test

---

## 6. Risk Items

| Risk | Impact | Likelihood | Mitigation |
|---|---|---|---|
| **GTK4/libadwaita Rust bindings are immature** | Blocked on GUI features, workarounds needed | Medium | Evaluate Iced as a fallback. Keep L3 interface abstract enough to swap rendering backends. |
| **Wayland compositor choice limits flexibility** | cage may be too minimal (no keyboard shortcuts), Mutter too heavy | Medium | Start with cage for simplicity. Switch to labwc if more control is needed. Both are wlroots-based. |
| **IPC protocol changes break L3/L2 compat** | Integration issues after independent development | Low | Define protocol types in a shared Rust crate (`levsha-protocol`). Both sides import the same types. |
| **Fedora lorax tooling is poorly documented** | ISO build takes longer than expected | High | Allocate extra time for Step 9. Use existing Fedora Spin kickstart files as reference. Test early. |
| **Claude API streaming SSE parsing is tricky** | Dropped tokens, rendering glitches | Medium | Use a well-tested SSE parser crate. Write thorough tests with recorded API responses. |
| **SQLite WAL mode + crash recovery edge cases** | Data loss on hard VM shutdown | Low | Use WAL mode with synchronous=NORMAL. Test with forced kills. Conversation loss on crash is acceptable in MVP. |
| **Full root access for tool calls is dangerous** | User accidentally destroys the system | High (by design) | Destructive command confirmation (Step 6). MVP runs in a VM — consequences are limited. Document the risk. |
| **Font rendering in VM looks bad** | Fails the "beautiful by default" requirement | Medium | Include fontconfig overrides in L1 overlay. Ship Inter + JetBrains Mono. Test on multiple display resolutions. |

---

## 7. Directory Mapping

Final directory structure after all steps are complete:

```
Levsha.OS/
  Cargo.toml                    # workspace root
  chat-shell/                   # L3 — Chat Shell GUI
    Cargo.toml
    src/
      main.rs
      ui/                       # GTK4 widgets, layout, styling
      ipc/                      # client-side IPC (connects to engine)
  engine/                       # L2 — Intelligence Engine
    Cargo.toml
    src/
      main.rs
      api/                      # Claude API client, streaming
      ipc/                      # server-side IPC (listens for chat-shell)
      tools/                    # tool execution, subprocess management
      skills/                   # skill loader, prompt/tool aggregation
      persistence/              # SQLite conversation storage
      safety/                   # destructive command detection
  protocol/                     # shared IPC types (Rust crate)
    Cargo.toml
    src/lib.rs
  skills/
    built-in/
      package-manager/          # skill manifest + prompt + tool schemas
      system-info/              # skill manifest + prompt + tool schemas
  base/
    kickstart/
      levsha-minimal.ks         # Fedora kickstart for ISO
    overlay/                    # filesystem overlay (configs, services, branding)
      etc/levsha/config.toml
      usr/lib/systemd/system/levsha-engine.service
      usr/lib/systemd/system/levsha-chat-shell.service
  infra/
    build-iso.sh                # ISO build script (wraps livemedia-creator)
    run-qemu.sh                 # QEMU launch script for testing
  docs/
    PRD/                        # module-level PRD documents
    ideation/                   # original PRD
  tests/
    boot_test.sh                # VM boot smoke test
    smoke_test.sh               # end-to-end message flow test
```
