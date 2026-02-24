# Levsha OS — Implementation Plan

## Context

The Levsha OS project has complete documentation (PRD, Build Guide, agent definitions) but zero implementation code. The goal is to go from zero to a **bootable ISO** that displays a full-screen chat GUI connected to Claude API. This plan implements `docs/technical/Levsha_OS_Build_Guide.md` and the MVP scope from the PRD.

**Strategy:** Skeleton first, flesh later. Build the thinnest vertical slice through all 4 layers (L0→L3), get it booting in a VM, then iterate on quality.

---

## Key Architectural Decisions

1. **Single binary** — Engine embedded in Chat Shell as a Rust library (tokio::mpsc channels for IPC). Simplest deployment.
2. **Cage as Wayland compositor** — Minimal wlroots kiosk compositor that runs one app full-screen. Zero config, ~2MB. Perfect match for "no desktop, no taskbar."
3. **GTK4 + libadwaita** — Best Fedora integration, font rendering, dark mode for free. Mature Rust bindings via `gtk4-rs`.
4. **Overlay pattern** — System configs live in `base/overlay/` mirroring the target filesystem, copied into the ISO via kickstart `%post`.

---

## Phase 0: Scaffolding & Contracts (~30 min, sequential)

Team lead only — no agents needed.

| Task | Deliverable |
|------|-------------|
| Create all directories | `chat-shell/`, `engine/`, `skills/`, `base/`, `infra/`, `tests/`, `docs/architecture/`, `branding/` |
| Init Rust workspace | `Cargo.toml` (workspace) + `chat-shell/Cargo.toml` + `engine/Cargo.toml` with skeleton `main.rs`/`lib.rs` |
| Define L2↔L3 protocol | `docs/architecture/l2-l3-protocol.md` — message types: UserMessage, AssistantChunk, ToolCall, ToolResult, ConfirmationRequest/Response, ToolStatus, Error |
| Define tool schema format | `docs/architecture/tool-schema.md` — Anthropic-compatible JSON Schema for skill tools |
| Unify config file path | Resolve discrepancy: Build Guide uses `/etc/levsha/levsha.conf` (INI), PRD 03 uses `/etc/levsha/engine.toml`, PRD 08 uses `/etc/levsha/config.toml`. Pick one canonical path and format (recommend `/etc/levsha/config.toml`, TOML). Update Build Guide and all PRDs to match. |

**Files:** 8 files total

---

## Phase 1: Parallel Core Development (~2-3 hrs, 5 agents in parallel)

**Team:** `levsha-core`

### Track A — Base System (`base-system` agent)
| Task | Files |
|------|-------|
| Production kickstart | `base/kickstart/levsha-os.ks` — from Build Guide, adds `cage` package, VirtIO/QXL guest drivers, sets GRUB timeout to 0 (instant boot per BF-03), points to staging dir |
| Filesystem overlay | `base/overlay/etc/levsha/config.toml` (unified config, TOML format), `base/overlay/etc/systemd/system/levsha-chat-watchdog.service`, `base/overlay/etc/systemd/system/getty@tty1.service.d/autologin.conf`, `base/overlay/etc/fonts/local.conf`, `base/overlay/etc/sudoers.d/levsha-dnf`, `base/overlay/home/levsha/.bash_profile` |
| Session launcher | `base/overlay/usr/bin/levsha-session` — shell script: `exec cage -- /usr/bin/levsha-chat` |
| Disable VT switching | Cage config or kernel cmdline to block Ctrl+Alt+F2 TTY escape (SA acceptance criteria: "no escape hatch"). Disable all getty services except tty1. |

### Track B — Infrastructure (`infra` agent)
| Task | Files |
|------|-------|
| Build script | `infra/build.sh` — build Rust, stage artifacts, validate kickstart, run livemedia-creator |
| Makefile | `Makefile` — targets: `toolchain`, `chat-shell`, `engine`, `stage`, `iso`, `test-vm`, `clean` |
| Build container | `infra/Containerfile` — Fedora with Rust toolchain + GTK4/libadwaita dev packages |

### Track C — Intelligence Engine (`engine` agent)
| Task | Files |
|------|-------|
| Core engine | `engine/src/lib.rs`, `types.rs`, `config.rs`, `context.rs`, `skill_loader.rs` |
| Anthropic API client | `engine/src/api_client.rs` — SSE streaming, tool_use parsing, error handling |
| Tool executor | `engine/src/tool_executor.rs` — shell command execution via tokio::process::Command |
| Persistence layer (P0) | `engine/src/history.rs` — SQLite with WAL mode from day one (not deferred). Schema: messages table with columns for id, role (user/assistant/system/tool_call/tool_result), content, tool_name, tool_args, exit_code, timestamp, completion_status. Auto-create tables on first run. DB path: `/var/lib/levsha/history.db` owned by `levsha` user. |
| Context window mgmt (P0) | `engine/src/context.rs` — rolling window of recent messages for API requests. Token counting at 4 chars/token (conservative). 4096-token response reserve. Oldest messages truncated first. System prompt + skill prompts never truncated. Truncation notification prepended when messages are dropped. |
| Welcome message (P0) | `engine/src/welcome.rs` — on first launch (empty DB), insert welcome message as first assistant message. On subsequent boots, restore conversation from DB (no repeat welcome). After history clear, re-show welcome. |
| History clear (P0) | Wire "clear history" / "start fresh" as a recognized command. Triggers destructive command confirmation flow. On confirm: VACUUM database, re-insert welcome message. |

### Track D — Chat Shell GUI (`chat-shell` agent)
| Task | Files |
|------|-------|
| GTK4 app scaffold | `chat-shell/src/main.rs`, `app.rs`, `window.rs` — full-screen, no decorations |
| Chat UI | `chat-shell/src/chat_view.rs`, `message_widget.rs`, `input_bar.rs` |
| Status bar (P0) | `chat-shell/src/status_bar.rs` — displays current time (HH:MM), connection state (connected/disconnected), AI backend name (e.g., "claude-sonnet"). Updates on connection status changes from engine. |
| Dark theme | `chat-shell/src/style.css` — dark palette, typography, code blocks. WCAG AA contrast ratios (4.5:1 body, 3:1 large). No white flashes on load. |
| Streaming | `chat-shell/src/streaming.rs` — token-by-token rendering with auto-scroll (pauses if user scrolls up, shows "scroll to bottom" affordance) |
| Keyboard shortcuts (P0) | `chat-shell/src/keybindings.rs` — Enter (send), Shift+Enter (newline), Ctrl+C (cancel streaming), Ctrl+L (clear visible chat, history preserved), Up arrow (recall previous message when input empty), Ctrl+A (select all), Ctrl+V (paste) |
| Error display (P0) | `chat-shell/src/error_display.rs` — styled inline error messages (visually distinct from normal messages: different color/border). Covers: network unreachable, API error, timeout, rate limit. Each includes a "Retry" affordance. Status bar reflects disconnected state. |

### Track E — Skills (`skills-dev` agent)
| Task | Files |
|------|-------|
| Package manager skill | `skills/built-in/package-manager/skill.yaml`, `prompts/package-manager.md`, `tools/{install,remove,search,update,list}.json` |
| System info skill | `skills/built-in/sysinfo/skill.yaml`, `prompts/sysinfo.md`, `tools/{disk_usage,memory,cpu_info,uptime,network,processes}.json` |
| System prompt | `skills/built-in/system-prompt.md` |

**Files:** ~52 files total across 5 tracks

---

## Phase 2: Integration & First Boot (~1-2 hrs, mostly sequential)

**Team:** `levsha-integration` — agents: `infra`, `engine`, `chat-shell`, `base-system`

### 2A — Wiring (sequential, each depends on the previous)

| Task | Agent | Description |
|------|-------|-------------|
| Cross-compile setup | `infra` | Verify Rust compiles in Fedora container (GTK4 deps). Extract binaries. |
| Wire L3↔L2 channels | `engine` + `chat-shell` | Engine as library dep in `chat-shell/Cargo.toml`. Create tokio::mpsc channel pair. Verify all 7 message types from protocol doc compile and serialize: UserMessage, StreamChunk, StreamEnd, ConfirmRequest, ConfirmResponse, ToolStatus, Error. |
| Wire L2↔API | `engine` | Engine reads API key from `/etc/levsha/config.toml` (or env var for local dev). Sends a real request to Claude API. Parses SSE stream. Verify: first token arrives on the mpsc channel within 2s. |
| Wire L2↔Skills | `engine` | Skill loader discovers `skills/built-in/*/skill.yaml`, parses manifests, loads prompt fragments and tool JSON schemas. Verify: tool definitions appear in the outgoing API request body. |
| Wire L2↔L1 (tool execution) | `engine` | When API returns a `tool_use` block, tool executor runs the command via `tokio::process::Command`. Captures stdout + stderr + exit code. Sends result back to API as `tool_result`. Verify: ask "what is my uptime?" → `uptime` executes → output returned to LLM → rendered in chat. |
| Wire L2↔SQLite | `engine` | Every message (user, assistant, tool_call, tool_result, system) is written to SQLite immediately. Verify: after 3 exchanges, `sqlite3 history.db "SELECT count(*) FROM messages"` returns correct count. |
| Wire L3↔History (startup restore) | `chat-shell` + `engine` | On startup, engine loads messages from SQLite, sends them to chat-shell for display. Verify: restart the app → previous conversation appears in the chat view. |
| Wire L3↔Status bar | `chat-shell` + `engine` | Engine sends connection status events to chat-shell. Status bar renders: time (HH:MM), connection state, backend name. Verify: disconnect network → status bar shows "disconnected". Reconnect → shows "connected". |
| Wire L3↔Error display | `chat-shell` + `engine` | Engine sends Error events on API failure. Chat-shell renders styled inline error with retry affordance. Verify: use invalid API key → styled auth error appears in chat (not a panic or raw HTTP dump). |
| Wire welcome message flow | `engine` | First launch (empty DB) → welcome message inserted and displayed. Kill and restart → conversation restored, no duplicate welcome. Verify both paths. |
| Wire Ctrl+C → cancel | `chat-shell` + `engine` | Ctrl+C during streaming sends a cancel signal to the engine, which aborts the in-flight API request. The partial response is finalized. Verify: start a long response → Ctrl+C → streaming stops, partial text preserved, input re-enabled. |

### 2B — Build & Boot

| Task | Agent | Description |
|------|-------|-------------|
| Build ISO | `infra` + `base-system` | Run build pipeline → `Levsha-OS-0.1.0.iso`. Verify: build script stages binary + skills + overlay + config into the ISO. |
| First boot in QEMU | `infra` | Boot ISO, verify: auto-login → cage → chat-shell full-screen. No intermediary screens. |
| Boot wiring smoke test | `infra` + `base-system` | Verify the full boot chain is actually connected: systemd starts getty → auto-login fires → `.bash_profile` runs → `levsha-session` launches cage → cage launches `levsha-chat` → chat-shell initializes engine → engine reads config → engine loads skills → welcome message appears. Each step depends on the previous — a break anywhere means black screen or crash. |
| Watchdog wiring test | `base-system` | Kill the chat-shell process (`kill -9`). Verify: systemd watchdog restarts it within 2 seconds. Conversation history survives (loaded from SQLite). |
| Kickstart↔Overlay verify | `base-system` | Verify all overlay files are actually copied to correct paths in the ISO filesystem: `config.toml` in `/etc/levsha/`, `levsha-chat` binary in `/usr/bin/`, skills in `/usr/share/levsha/skills/`, fonts config in `/etc/fonts/`, sudoers in `/etc/sudoers.d/`. |

**Deliverable:** First bootable ISO with working chat, all component seams verified

---

## Phase 3: Polish & Security (~1-2 hrs, 5 agents in parallel)

**Team:** `levsha-polish` — agents: `chat-shell`, `engine`, `security`, `qa`, `base-system`

| Track | Agent | Tasks |
|-------|-------|-------|
| Rich rendering | `chat-shell` | Code blocks with syntax highlighting (Python, Rust, Bash, JSON, YAML minimum), tables with aligned columns, progress indicators (animated spinner + completion checkmark/X), ordered/unordered lists |
| Animations | `chat-shell` | Message fade-in (150-250ms ease-out), smooth momentum scrolling at 60fps, typing indicator while awaiting first token, smooth state transitions |
| Chat search (P1) | `chat-shell` | `chat-shell/src/search.rs` — Ctrl+F opens search overlay at top, case-insensitive substring match, Enter/Shift+Enter navigate between matches, Escape dismisses |
| Destructive cmd confirm | `security` | Risk classifier (`engine/src/risk_classifier.rs`): patterns for `rm -rf`, `mkfs.*`, `dd if=`, `fdisk`, `wipefs`, `parted`, `systemctl stop/disable` on core services, bulk package removal. Confirmation dialog UI in chat-shell. |
| Error handling | `engine` + `chat-shell` | Engine: exponential backoff retries (3 attempts, 1s/2s/4s), `Retry-After` header parsing for 429, graceful handling of 401/500/502/503/timeout/malformed response. Chat Shell: render styled error messages from engine `Error` events (CS-07). |
| Boot splash (P1) | `base-system` | Plymouth theme with Levsha OS logo and animated progress indicator. Resolve conflict: Build Guide kickstart removes Plymouth — re-add it. Smooth transition to compositor (no flicker). |
| Branding assets (P1) | `chat-shell` | `branding/logo.png`, `branding/boot-splash/` Plymouth theme assets. Staged into ISO by build script. |
| Unit tests | `qa` | Engine: API parsing, risk classification, context window truncation, history persistence. Chat Shell: message formatting, keyboard shortcuts |
| Smoke tests | `qa` | `tests/smoke_test.sh` — BATS-based VM boot validation |

### Phase 3 Integration Wiring Checks

These verify that Phase 3 features are wired end-to-end, not just implemented in isolation:

| Check | Components | Verification |
|-------|-----------|--------------|
| Destructive confirm round-trip | `security` ↔ `engine` ↔ `chat-shell` | User asks "remove curl" → engine detects destructive pattern → sends ConfirmRequest to chat-shell → chat-shell renders styled confirmation prompt → user approves → engine executes → result rendered. Also test: user rejects → engine informs LLM → LLM acknowledges cancellation in chat. |
| Risk classifier ↔ skill tools | `security` ↔ `engine` ↔ `skills` | Package manager `remove` tool output is intercepted by risk classifier before execution. System info tools (read-only) pass through without confirmation. |
| Error retry wiring | `engine` ↔ `chat-shell` | Styled error with "Retry" appears → user triggers retry → engine re-sends the last request → response streams normally. Not just display — the retry action must actually re-invoke the API. |
| History clear full loop | `engine` ↔ `chat-shell` ↔ `security` | User says "clear history" → engine detects as destructive → ConfirmRequest sent → user confirms → DB vacuumed → welcome message re-inserted → chat-shell clears view and shows welcome. Verify DB is actually empty except welcome. |
| Context window ↔ API | `engine` | After 50+ messages, verify the API request payload only contains the rolling window (not the full history). Verify system prompt + skill prompts are always present. Verify truncation notification is prepended when messages are dropped. |
| Streaming ↔ rich rendering | `chat-shell` | A response containing a code block streams token-by-token. Verify: during streaming, partial markdown is handled gracefully (no broken rendering). On stream completion, code block is fully syntax-highlighted. |
| Tool status ↔ progress indicator | `engine` ↔ `chat-shell` | Engine sends ToolStatus(started) → chat-shell shows animated spinner with tool description. Engine sends ToolStatus(completed) → spinner replaced with checkmark/X. Verify the spinner is not stuck if tool execution fails. |
| Boot splash ↔ compositor transition (P1) | `base-system` ↔ `chat-shell` | Plymouth splash renders during boot → transitions to cage compositor → chat-shell appears. No flicker, no black frame, no mode-switch artifact between Plymouth and Wayland. |

---

## Phase 4: Docs & Final ISO (~30 min, 3 agents)

**Team:** `levsha-release` — agents: `docs`, `infra`, `qa`

| Task | Agent | Files |
|------|-------|-------|
| Architecture docs | `docs` | `docs/architecture/overview.md`, ADRs for cage/embedded-engine/GTK4 |
| Dev guide | `docs` | `docs/dev-guide/getting-started.md` |
| Final ISO rebuild | `infra` | Rebuild with all polish |
| Final validation | `qa` | Boot time < 30s, RAM < 1GB, ISO size < 1.5GB, streaming works, persistence works, no TTY escape, welcome message logic, history clear |

---

## Dependency Graph

```
Phase 0 (scaffolding, contracts)
    │
    ├── Track A: base-system ──┐
    ├── Track B: infra ────────┤
    ├── Track C: engine ───────┼── Phase 2A (wiring) ── Phase 2B (build & boot) ── Phase 3 (polish) ── Phase 4 (release)
    ├── Track D: chat-shell ───┤
    └── Track E: skills-dev ───┘
```

Phase 0 → Phase 1 (parallel) → Phase 2A (sequential wiring) → Phase 2B (build + boot verification) → Phase 3 (parallel + integration checks) → Phase 4 (final validation)

**Key principle:** Each seam between components gets an explicit wiring check. A component passing its own tests is necessary but not sufficient — data must actually flow across every boundary.

---

## Risk Mitigations

| Risk | Mitigation |
|------|------------|
| GTK4 Rust bindings complexity | Start with simplest possible UI (GtkTextView + GtkEntry). Polish in Phase 3. |
| macOS → Linux cross-compilation | ALL Rust compilation inside Fedora container (Containerfile). Never attempt native cross-compile for GTK4. |
| `cage` not in Fedora repos | Fallback: `labwc` or compile cage from source in build container |
| livemedia-creator fails | Fallback: `livecd-creator` (simpler). Makefile supports both methods. |
| Plymouth vs kickstart conflict | Build Guide kickstart removes Plymouth (`-plymouth`), but PRD 08 requires a boot splash. Resolution: re-add Plymouth in Phase 3 with custom theme. Phase 1-2 boots without splash (acceptable for early testing). |
| Config path inconsistency | Resolved in Phase 0 by picking `/etc/levsha/config.toml` (TOML). All overlay files and code use this single path. |

---

## Verification

### After Phase 2 — Component + Wiring

**Component checks:**
- [ ] ISO boots in QEMU in < 30 seconds (GRUB timeout = 0)
- [ ] Auto-login → cage → chat-shell appears full-screen
- [ ] Ctrl+Alt+F2 does NOT switch to a TTY (no escape hatch)

**L3↔L2 wiring (Chat Shell ↔ Engine):**
- [ ] Type a message → engine receives UserMessage via mpsc channel → sends to API → StreamChunks flow back → rendered token-by-token in chat
- [ ] Ctrl+C during streaming → engine cancels in-flight request → partial response preserved → input re-enabled
- [ ] Status bar updates live: shows time, "connected"/"disconnected", backend name — changes reflect engine state in real time

**L2↔API wiring (Engine ↔ Claude):**
- [ ] Engine reads API key from `/etc/levsha/config.toml` → authenticates with Anthropic API → receives responses
- [ ] Invalid API key → styled auth error in chat (not panic or raw HTTP)

**L2↔L1 wiring (Engine ↔ System via tool calling):**
- [ ] Ask "what is my uptime?" → engine includes sysinfo tool defs in API request → LLM returns tool_use → engine executes `uptime` → stdout captured → tool_result sent back to API → final answer rendered
- [ ] Ask "install cowsay" → engine includes pkg-manager tool defs → LLM returns tool_use → engine executes `sudo dnf install -y cowsay` → result rendered

**L2↔Skills wiring (Engine ↔ Skill Loader):**
- [ ] On startup, engine discovers and loads both built-in skills from `/usr/share/levsha/skills/`
- [ ] Skill prompt fragments are present in the system prompt sent to API
- [ ] All 11 tool definitions (5 pkg-manager + 6 sysinfo) are included in every API request

**L2↔SQLite wiring (Engine ↔ Persistence):**
- [ ] Every message is persisted immediately (user, assistant, tool_call, tool_result, system)
- [ ] Reboot VM → previous conversation restored in chat view from SQLite
- [ ] Force-kill VM process → restart → no DB corruption, messages intact (WAL)

**Welcome message wiring:**
- [ ] First boot (empty DB) → welcome message displayed as first assistant message, stored in DB
- [ ] Second boot → conversation restored, welcome NOT repeated
- [ ] Welcome message content matches PRD (mentions package management and system info capabilities)

**Boot chain wiring (L1→L3):**
- [ ] systemd → getty@tty1 → autologin → .bash_profile → levsha-session → cage → levsha-chat — each link actually triggers the next
- [ ] Kill chat-shell (`kill -9`) → systemd watchdog restarts it within 2s → history loaded from SQLite → chat view restored
- [ ] All overlay files present at correct paths in running ISO filesystem

### After Phase 4 — Full Integration

**All Phase 2 checks pass, plus:**

**Destructive command confirmation wiring:**
- [ ] "remove curl" → risk classifier triggers → ConfirmRequest sent to chat-shell → styled confirmation dialog rendered → user approves → command executes → result rendered
- [ ] User rejects confirmation → engine informs LLM → LLM acknowledges cancellation in chat
- [ ] Non-destructive commands (search, list, sysinfo) execute without confirmation prompt

**History clear full loop:**
- [ ] "clear history" → recognized as destructive → confirmation prompt → user confirms → DB vacuumed → welcome message re-inserted → chat-shell clears and shows fresh welcome
- [ ] After clear, DB contains only the welcome message (verified via `sqlite3`)

**Error handling wiring:**
- [ ] Network disconnect → engine retries with exponential backoff (1s/2s/4s) → after 3 failures sends Error to chat-shell → styled error with "Retry" rendered → status bar shows "disconnected"
- [ ] User triggers Retry → engine re-sends last request → on success, response streams normally → status bar returns to "connected"
- [ ] Rate limit (429) → engine parses Retry-After → auto-retries or surfaces wait time to user

**Context window ↔ API wiring:**
- [ ] After 50+ messages, API request contains rolling window (not full history). System prompt + skill prompts always present. Truncation notification prepended when messages are dropped.

**Streaming ↔ rendering wiring:**
- [ ] Response with code block streams token-by-token → partial markdown handled without broken rendering → on completion, full syntax highlighting applied
- [ ] ToolStatus(started) from engine → animated spinner in chat → ToolStatus(completed) → spinner replaced with checkmark or X

**Performance & size:**
- [ ] Idle RAM < 1 GB
- [ ] ISO size < 1.5 GB
- [ ] `make test-vm` passes smoke tests
- [ ] All unit tests pass (`cargo test`)

**P1 features:**
- [ ] Chat search (Ctrl+F) → search overlay opens → matches highlighted across all messages → Enter/Shift+Enter navigate → Escape dismisses
- [ ] Boot splash → Plymouth shows logo with animation → transitions to cage → chat-shell appears — no flicker or black frame between Plymouth and Wayland

---

## Total Files: ~68 new files across 4 phases
## Estimated Effort: 6-8 hours of agent execution time (heavily parallelized)

---

## Appendix: PRD Module Coverage

Checklist confirming every PRD module is addressed in this plan:

| PRD Module | Phase | Notes |
|------------|-------|-------|
| 00 — System Architecture | Phase 0 (contracts), Phase 2 (integration) | 4-layer model, IPC protocol, no-escape-hatch |
| 01 — Base System | Phase 1 Track A | Kickstart, overlay, VT lockdown, GRUB, VM drivers |
| 02 — Chat Shell | Phase 1 Track D, Phase 3 | GUI, status bar, shortcuts, dark theme, streaming, search, errors |
| 03 — Intelligence Engine | Phase 1 Track C | API client, tool executor, context window, error handling |
| 04 — Skills System | Phase 1 Track C + E | Skill loader, manifest format, prompt injection |
| 05 — Package Manager Skill | Phase 1 Track E | 5 tools, prompt, manifest |
| 06 — System Info Skill | Phase 1 Track E | 6 tools, prompt, manifest |
| 07 — Persistence | Phase 1 Track C | SQLite + WAL (day one), schema, history clear, welcome restore |
| 08 — Boot and First Run | Phase 1 Track A + C, Phase 3 | Auto-login, GRUB=0, welcome message, boot splash (P1) |
| 09 — ISO Build | Phase 1 Track B, Phase 2, Phase 4 | Build script, container, livemedia-creator, QEMU test |
