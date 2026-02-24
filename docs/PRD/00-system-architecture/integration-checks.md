# 00 — System Architecture: Integration Checks

**Module:** Cross-Module Integration Verification
**Phase:** 1 (MVP)
**Status:** Draft

---

## Purpose

Individual module acceptance criteria verify that each component works in isolation. Integration checks verify that modules are **wired together correctly** — that data actually flows between them, protocols are compatible, and the full stack behaves as a connected system.

Every integration check below tests a **seam** between two or more modules. A check passes only when the full round-trip across that seam succeeds.

---

## 1. IPC Wiring: Chat Shell (L3) <-> Engine (L2)

The Unix domain socket at `/run/levsha/engine.sock` is the primary integration seam.

### IC-01 — Socket Connection Establishment

| Aspect | Check |
|--------|-------|
| Precondition | Engine is running and listening on `/run/levsha/engine.sock` |
| Action | Chat Shell starts and attempts connection |
| Pass criteria | Chat Shell connects within 5 seconds. Engine logs "client connected". Chat Shell status bar shows "connected". |
| Failure mode | Chat Shell shows "Connecting..." indefinitely, status bar never updates |

### IC-02 — User Message Delivery

| Aspect | Check |
|--------|-------|
| Precondition | L3 and L2 connected via IPC |
| Action | User types "hello" in Chat Shell input field and presses Enter |
| Pass criteria | Engine receives `{"type": "user_message", "content": "hello"}` on the socket. Engine logs the received message. |
| Failure mode | Message never arrives at Engine. Engine receives malformed JSON. |

### IC-03 — Streaming Token Round-Trip

| Aspect | Check |
|--------|-------|
| Precondition | L3 and L2 connected. User message sent. |
| Action | Engine sends `stream_start`, then 5 `token` messages, then `stream_end` over IPC |
| Pass criteria | All 5 tokens appear in Chat Shell in order. Message bubble grows incrementally. No tokens lost or duplicated. Auto-scroll follows new content. |
| Failure mode | Tokens appear out of order, are missing, or arrive in a single batch instead of streaming. |

### IC-04 — Cancel Signal

| Aspect | Check |
|--------|-------|
| Precondition | Engine is streaming tokens to Chat Shell |
| Action | User presses Ctrl+C in Chat Shell |
| Pass criteria | Chat Shell sends `{"type": "cancel"}` to Engine. Engine stops the API call and streaming. Partial message is preserved in Chat Shell. UI returns to IDLE state. |
| Failure mode | Cancel signal not received by Engine. Streaming continues after cancel. Chat Shell stuck in STREAMING state. |

### IC-05 — Error Propagation

| Aspect | Check |
|--------|-------|
| Precondition | L3 and L2 connected |
| Action | Engine sends `{"type": "error", "code": "api_unreachable", "message": "Cannot reach API"}` |
| Pass criteria | Chat Shell renders a styled error message (copper border, warm background per theme.design.md). Retry affordance is visible. Status bar shows "disconnected". |
| Failure mode | Error message not rendered. Raw JSON visible. Status bar not updated. Chat Shell crashes. |

### IC-06 — Status Update Propagation

| Aspect | Check |
|--------|-------|
| Precondition | L3 and L2 connected |
| Action | Engine sends `{"type": "status", "connection": "connected", "backend": "claude-sonnet"}` |
| Pass criteria | Chat Shell status bar displays "claude-sonnet" and "connected". When Engine sends `"connection": "disconnected"`, status bar updates to "disconnected" with visual change. |
| Failure mode | Status bar shows stale data. Backend name not updated. |

### IC-07 — Protocol Version Compatibility

| Aspect | Check |
|--------|-------|
| Precondition | Engine and Chat Shell built from same commit |
| Action | Exchange all defined message types in both directions |
| Pass criteria | Every message type defined in the protocol crate is correctly serialized by sender and deserialized by receiver. No `unknown field` or `missing field` errors. |
| Failure mode | Serialization mismatch between L2 and L3. Fields renamed in one side but not the other. |

---

## 2. Engine (L2) <-> Claude API

### IC-10 — API Request Construction

| Aspect | Check |
|--------|-------|
| Precondition | Engine running with valid API key in `/etc/levsha/config.toml` |
| Action | User sends a message through the full stack |
| Pass criteria | Engine constructs a valid Anthropic Messages API request containing: system prompt (base + all skill prompts), conversation history, tool definitions from loaded skills. Request succeeds (200 OK). |
| Failure mode | API returns 400 (malformed request). System prompt missing skill fragments. Tool definitions missing or invalid schema. |

### IC-11 — Streaming SSE to IPC Token Bridge

| Aspect | Check |
|--------|-------|
| Precondition | API request sent, streaming response active |
| Action | API streams SSE events with `content_block_delta` containing text tokens |
| Pass criteria | Each SSE text delta is converted to an IPC `token` message and forwarded to Chat Shell within 16ms. No tokens dropped. Token order preserved. |
| Failure mode | SSE parser drops events. Tokens batched instead of forwarded individually. Token order scrambled. |

### IC-12 — Tool Call Extraction and Execution

| Aspect | Check |
|--------|-------|
| Precondition | API response contains `tool_use` content block |
| Action | API returns a tool call (e.g., `disk_usage` from system-info skill) |
| Pass criteria | Engine extracts tool name and parameters. Looks up tool in registry (matches a loaded skill). Executes corresponding command. Captures stdout/stderr. Sends `tool_result` back to API in next request. Final response incorporates tool output. |
| Failure mode | Tool name not found in registry. Command not executed. Tool result not sent back. API receives empty tool result. |

### IC-13 — Multi-Turn Tool Calling

| Aspect | Check |
|--------|-------|
| Precondition | API may request multiple sequential tool calls |
| Action | User asks "install ffmpeg and check disk space" |
| Pass criteria | Engine handles multiple tool_use blocks. Executes each tool sequentially. Sends all results back. API receives complete tool results and generates final summary. Chat Shell renders the complete response. |
| Failure mode | Only first tool call executed. Second tool call lost. API receives partial results. |

---

## 3. Tool Execution: Engine (L2) <-> Base System (L1)

### IC-20 — Command Execution Path

| Aspect | Check |
|--------|-------|
| Precondition | Engine running with root access |
| Action | Engine executes `df -h` via `tokio::process::Command` |
| Pass criteria | Command runs on base system. stdout captured correctly. Exit code 0 returned. Output matches actual disk state. |
| Failure mode | Command not found (binary missing from base system). Permission denied. stdout not captured. |

### IC-21 — Package Installation End-to-End

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running. `cowsay` not installed. |
| Action | User types "install cowsay" in Chat Shell |
| Pass criteria | (1) Chat Shell sends message to Engine. (2) Engine calls Claude API with package-manager skill tools. (3) API returns `package_install` tool call. (4) Engine executes `dnf install -y cowsay`. (5) dnf succeeds on base system. (6) Engine sends result back to API. (7) API generates success response. (8) Chat Shell renders "cowsay installed" with success indicator. (9) `which cowsay` returns valid path. |
| Failure mode | Any step in the chain fails silently. Package not actually installed despite success message. |

### IC-22 — Destructive Command Confirmation Round-Trip

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running. `cowsay` installed. |
| Action | User types "remove cowsay" |
| Pass criteria | (1) Engine detects `dnf remove` as destructive. (2) Engine sends `confirm_request` to Chat Shell via IPC. (3) Chat Shell renders distinct confirmation prompt (copper border, warning styling per theme). (4) User approves. (5) Chat Shell sends `confirm_response(approved=true)` to Engine. (6) Engine executes `dnf remove -y cowsay`. (7) Package removed. (8) Success response rendered. |
| Also test | User denies: `confirm_response(approved=false)` → Engine cancels → Chat Shell shows cancellation message. Package still installed. |
| Failure mode | Destructive command executes without confirmation. Confirmation prompt not rendered. User response not delivered to Engine. |

### IC-23 — Command Timeout Handling

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running |
| Action | Engine executes a command that hangs (e.g., `sleep 60`) |
| Pass criteria | Engine kills process after 30s timeout. Sends timeout error to API. Chat Shell renders timeout message. System is not left with zombie processes. |
| Failure mode | Process runs forever. Engine hangs. No timeout message. |

---

## 4. Skills Wiring: Skills <-> Engine <-> API

### IC-30 — Skill Loading and Prompt Injection

| Aspect | Check |
|--------|-------|
| Precondition | Engine starts with skills in `/usr/share/levsha/skills/` |
| Action | Engine loads skills at startup |
| Pass criteria | (1) `package-manager` skill loaded (5 tools registered). (2) `system-info` skill loaded (6 tools registered). (3) System prompt contains both skill prompt fragments. (4) API request includes all 11 tool definitions. |
| Verify | Send API request, inspect that `tools` array has 11 entries. Inspect `system` prompt contains package-manager and system-info prompt text. |
| Failure mode | Skills not found. Tools not registered. Prompt fragments missing. API request has empty tools array. |

### IC-31 — Skill Tool Invocation Routing

| Aspect | Check |
|--------|-------|
| Precondition | Skills loaded. API can invoke tools. |
| Action | User asks "what's my disk usage?" |
| Pass criteria | (1) API returns `disk_usage` tool call. (2) Engine finds `disk_usage` in system-info skill registry. (3) Engine executes corresponding command (`df -B1`). (4) Output parsed and sent back to API. (5) Final response contains formatted disk info. (6) Chat Shell renders it correctly. |
| Failure mode | Tool name mismatch between API schema and engine registry. Wrong command executed. Output not parsed. |

### IC-32 — Unknown Tool Handling

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running |
| Action | API returns a tool_use with an unknown tool name (edge case) |
| Pass criteria | Engine returns a tool_result with error indicating tool not found. API generates an appropriate response. No crash. |
| Failure mode | Engine panics on unknown tool. Silent failure. |

---

## 5. Persistence Wiring: Engine (L2) <-> SQLite <-> Chat Shell (L3)

### IC-40 — Message Write and Read Back

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running. Fresh database. |
| Action | User sends "hello", receives response. Then restart Chat Shell. |
| Pass criteria | (1) Engine writes user message to SQLite. (2) Engine writes assistant response to SQLite. (3) Chat Shell restarts. (4) Chat Shell reads history from SQLite. (5) Both messages appear in Chat Shell after restart, in correct order. |
| Failure mode | Messages not written. Messages not loaded on restart. Wrong order. Missing messages. |

### IC-41 — Streaming Message Persistence

| Aspect | Check |
|--------|-------|
| Precondition | Engine streaming a response |
| Action | Kill Engine mid-stream (simulate crash) |
| Pass criteria | (1) Partial message is in SQLite with `is_complete=false`. (2) Engine restarts. (3) Chat Shell reconnects. (4) History shows partial message (or it's cleaned up gracefully). (5) User can send new message. |
| Failure mode | Database corrupted. Partial message causes crash on load. History lost entirely. |

### IC-42 — History Survives Full Reboot

| Aspect | Check |
|--------|-------|
| Precondition | VM running with conversation history |
| Action | Reboot the VM |
| Pass criteria | (1) VM shuts down. (2) VM boots back up. (3) Chat Shell displays previous conversation history from SQLite. (4) User can continue conversation. |
| Failure mode | Database file lost. History not loaded after boot. WAL file not properly checkpointed. |

### IC-43 — Context Window Construction

| Aspect | Check |
|--------|-------|
| Precondition | Database has 200+ messages |
| Action | User sends a new message |
| Pass criteria | Engine loads recent messages from SQLite. Token count stays within API limits. Oldest messages truncated. Conversation boundaries respected (no orphaned tool results). API request contains coherent history. |
| Failure mode | Too many tokens sent (API rejects). Context window includes orphaned tool results. Messages loaded in wrong order. |

### IC-44 — Clear History Flow

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running with history |
| Action | User triggers clear history (Ctrl+L in Chat Shell, or "clear history" command) |
| Pass criteria | (1) Chat Shell clears in-memory message list. (2) Engine receives clear command. (3) Engine clears/resets SQLite conversation. (4) Next message starts fresh context. (5) After restart, old messages are gone. |
| Failure mode | Only in-memory cleared but database still has old messages. Context window still includes cleared messages. |

---

## 6. Boot Chain: L1 -> Compositor -> L3 -> L2

### IC-50 — Full Boot Sequence

| Aspect | Check |
|--------|-------|
| Precondition | ISO booted in QEMU (cold start) |
| Action | Power on VM |
| Pass criteria | (1) BIOS/UEFI → kernel. (2) Plymouth splash displays Levsha logo with copper diamond and loading dots. (3) systemd starts services. (4) Auto-login occurs (no login prompt). (5) Cage compositor starts. (6) Chat Shell launches full-screen. (7) Engine connects and sends welcome message. (8) Status bar shows "connected" and backend name. (9) Input field is focused. (10) Total time < 30 seconds. |
| Failure mode | Any step stalls. Login prompt appears. Black screen between splash and Chat Shell. Welcome message missing. Status bar shows "disconnected". |

### IC-51 — Service Dependency Chain

| Aspect | Check |
|--------|-------|
| Precondition | System booting |
| Action | Verify systemd ordering |
| Pass criteria | (1) NetworkManager starts first. (2) levsha-engine.service starts After NetworkManager. (3) Engine reports READY via sd_notify. (4) Chat Shell connects to Engine socket. (5) No races: Chat Shell doesn't crash if Engine isn't ready yet (retries). |
| Verify | `systemctl list-dependencies levsha-engine.service` shows correct ordering. `journalctl` shows Engine READY before Chat Shell connection. |
| Failure mode | Engine starts before network is available. Chat Shell crashes because socket doesn't exist yet. |

### IC-52 — Chat Shell Crash Recovery

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running in VM |
| Action | `kill -9 $(pidof levsha-chat-shell)` |
| Pass criteria | (1) systemd detects crash. (2) Restarts Chat Shell within 2 seconds. (3) Chat Shell reconnects to Engine. (4) History reloaded from SQLite. (5) Status bar shows "connected". (6) User can continue conversation. |
| Failure mode | Chat Shell not restarted. Blank screen. History lost. Engine socket in bad state after reconnect. |

### IC-53 — Engine Crash Recovery

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running in VM |
| Action | `kill -9 $(pidof levsha-engine)` |
| Pass criteria | (1) Chat Shell detects disconnection. (2) Status bar shows "disconnected". (3) Error message rendered (not a blank screen or crash). (4) systemd restarts Engine within 2 seconds. (5) Engine re-initializes: loads config, opens DB, loads skills, opens socket. (6) Chat Shell reconnects automatically. (7) Status bar returns to "connected". (8) User can resume conversation. |
| Failure mode | Chat Shell crashes when Engine dies. No reconnection. Socket file stale (EADDRINUSE on restart). History lost. |

### IC-54 — No TTY Escape

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running in VM |
| Action | Press Ctrl+Alt+F2, Ctrl+Alt+F3, etc. |
| Pass criteria | Nothing happens. No VT switch. Chat Shell remains on screen. |
| Failure mode | User reaches a TTY login prompt or blank console. |

---

## 7. Network Edge Cases

### IC-60 — Offline Boot

| Aspect | Check |
|--------|-------|
| Precondition | VM with no network adapter (or cable disconnected) |
| Action | Boot the system |
| Pass criteria | (1) System boots to Chat Shell (no hang on network wait). (2) Status bar shows "disconnected". (3) Input field is focused and responsive. (4) User can type a message. (5) Error message appears (styled, not crash). (6) Previous history is displayed if database exists. |
| Failure mode | Boot hangs waiting for network. Chat Shell crashes without API. Blank screen. |

### IC-61 — Network Loss During Streaming

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running, actively streaming a response |
| Action | Disconnect VM network adapter mid-stream |
| Pass criteria | (1) Engine detects connection loss. (2) Engine sends error to Chat Shell via IPC. (3) Chat Shell renders partial message + error. (4) Status bar updates to "disconnected". (5) Retry affordance visible. (6) When network restored, user can retry. |
| Failure mode | Streaming hangs forever. No error displayed. Engine retries infinitely without informing user. |

### IC-62 — Network Recovery

| Aspect | Check |
|--------|-------|
| Precondition | System booted offline, showing "disconnected" |
| Action | Connect network adapter |
| Pass criteria | (1) Engine detects connectivity (periodic check or NetworkManager signal). (2) Status bar updates to "connected" with backend name. (3) User can send messages and receive responses. |
| Failure mode | Status bar never updates. Engine doesn't detect recovery. User must reboot. |

---

## 8. Visual Integration

### IC-70 — Theme Consistency Across Components

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running |
| Action | Visual inspection of all UI elements |
| Pass criteria | (1) Background is warm parchment (#FBF8F3), not white or dark. (2) All text is warm brown (#3A3228), never pure black. (3) Input field focus border is copper (#C67A52). (4) Error messages use copper border and warm background. (5) Confirmation prompts use copper styling. (6) Status bar uses bg-secondary (#F5F1EA). (7) Code blocks use bg-code (#F7F4EE). (8) No component uses off-theme colors. |
| Failure mode | GTK4 default theme leaks through. Components use different color palettes. |

### IC-71 — Font Rendering Chain

| Aspect | Check |
|--------|-------|
| Precondition | System booted with font packages installed |
| Action | Visual inspection of rendered text |
| Pass criteria | (1) Conversation text renders in IBM Plex Sans (or fallback per theme.design.md). (2) Code blocks render in IBM Plex Mono. (3) Subpixel antialiasing active (no jagged edges). (4) Font hinting is "slight". (5) Text is legible at 1280x720 and 1920x1080. |
| Verify | `fc-match "IBM Plex Sans"` returns correct font. fontconfig config at `/etc/fonts/local.conf` has correct settings. |
| Failure mode | Wrong font rendered (fallback to Noto or DejaVu). No antialiasing. Blurry text. |

### IC-72 — Smooth Transition: Plymouth -> Chat Shell

| Aspect | Check |
|--------|-------|
| Precondition | System booting |
| Action | Watch boot sequence |
| Pass criteria | (1) Plymouth shows warm parchment background (#FBF8F3) with copper logo. (2) Transition to compositor is seamless (no flash, no mode change visible). (3) Chat Shell appears on same warm background. (4) No black frames, no white flashes, no VT switch visible. |
| Failure mode | Black screen between Plymouth and Chat Shell. White flash. Mode switch visible (text mode briefly appears). |

---

## 9. Performance Integration

### IC-80 — 60fps During Streaming

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running in reference VM (2 vCPU, 2 GB RAM) |
| Action | Stream a long response (500+ tokens) while observing frame rate |
| Pass criteria | Chat Shell maintains 60fps throughout streaming. No dropped frames during token appends. Auto-scroll is smooth. |
| Verify | Debug frame counter overlay or external measurement tool shows consistent 60fps. |
| Failure mode | Frame drops during streaming. Visible jank on scroll. Layout jumps on token append. |

### IC-81 — Memory Budget

| Aspect | Check |
|--------|-------|
| Precondition | System booted, idle after one conversation exchange |
| Action | Measure RSS of all Levsha processes |
| Pass criteria | Chat Shell < 350 MB RSS. Engine < 150 MB RSS. Total system < 1 GB. |
| Verify | `ps aux | grep levsha` for per-process RSS. `free -m` for total system. |
| Failure mode | Memory exceeds budget. Leak over time with conversation length. |

### IC-82 — Token Append Latency

| Aspect | Check |
|--------|-------|
| Precondition | Full stack running |
| Action | Measure time from Engine sending `token` IPC message to pixel on screen |
| Pass criteria | < 16ms (one frame at 60fps). Token appears in the same or next frame after IPC delivery. |
| Failure mode | Token append triggers expensive re-layout. Multiple frame delays. |

---

## 10. End-to-End Smoke Tests

These are full-stack tests that exercise the entire chain. Each must pass on the final ISO.

### IC-90 — First Boot Welcome

```
Boot fresh ISO in QEMU
  -> Plymouth splash appears (parchment bg, copper logo)
  -> Chat Shell launches (< 30s total)
  -> Welcome message displayed:
       "Welcome. I'm your operating system.
        Everything you need -- just ask."
  -> Status bar: "claude-sonnet · connected · HH:MM"
  -> Input field focused, cursor blinking
```

### IC-91 — Simple Conversation

```
Type: "hello, what can you do?"
  -> Typing indicator appears (three pulsing dots)
  -> Streaming response begins (tokens appear incrementally)
  -> Response describes package management and system info capabilities
  -> Message complete, cursor returns to input
```

### IC-92 — System Query

```
Type: "how much disk space do I have?"
  -> Engine calls Claude API
  -> API returns disk_usage tool call
  -> Engine executes df command on base system
  -> Engine sends result back to API
  -> API generates formatted response
  -> Chat Shell renders disk usage table/info
  -> Data matches actual df -h output
```

### IC-93 — Package Install

```
Type: "install cowsay"
  -> API returns package_install tool call
  -> Engine executes dnf install -y cowsay
  -> Progress indicator shown during install
  -> Success message with checkmark rendered
  -> Verify: which cowsay returns /usr/bin/cowsay
```

### IC-94 — Destructive Command with Confirmation

```
Type: "remove cowsay"
  -> Engine detects destructive command
  -> Confirmation prompt rendered (copper border, warning style)
  -> User types "yes" or clicks confirm
  -> Engine executes dnf remove
  -> Success message rendered
  -> Verify: which cowsay returns nothing
```

### IC-95 — Conversation Persistence Across Reboot

```
After IC-91 through IC-94:
  -> Reboot VM
  -> Chat Shell launches
  -> Previous conversation visible (welcome + all messages)
  -> User can continue conversation
```

### IC-96 — Error Recovery

```
Disconnect VM network adapter
  -> Status bar shows "disconnected"
  -> Type: "hello"
  -> Error message rendered (styled, not crash)
  -> Reconnect network
  -> Status bar returns to "connected"
  -> Type: "hello" again
  -> Response received successfully
```

---

## Integration Check Priority

| Priority | Checks | Rationale |
|----------|--------|-----------|
| **P0 — Must pass for MVP** | IC-01 through IC-06, IC-10 through IC-12, IC-20 through IC-22, IC-30, IC-31, IC-40 through IC-42, IC-50 through IC-53, IC-60, IC-70, IC-90 through IC-96 | Core functionality chain must be fully wired |
| **P1 — Should pass** | IC-07, IC-13, IC-23, IC-32, IC-43, IC-44, IC-54, IC-61, IC-62, IC-71, IC-72, IC-80 through IC-82 | Robustness and polish |

---

## Running Integration Checks

### Local Development

During development, L3 and L2 run as regular processes on the developer machine:

```sh
# Terminal 1: Start Engine
cd engine && cargo run

# Terminal 2: Start Chat Shell
cd chat-shell && cargo run

# Terminal 3: Run integration checks
cd tests && ./integration-checks.sh
```

The IPC socket path defaults to `/tmp/levsha-dev/engine.sock` in development mode.

### VM Testing

On the built ISO in QEMU:

```sh
# Boot ISO
./infra/run-qemu.sh

# Run smoke tests via serial console or SSH
./tests/smoke_test.sh
./tests/integration_test.sh
```

### CI

Integration checks run as part of the nightly build:

1. Build both Rust binaries
2. Start Engine in background
3. Start Chat Shell in headless mode (if supported) or skip visual checks
4. Run IC-01 through IC-44 (IPC, persistence, skills, tool execution)
5. ISO build triggers full IC-50+ checks in QEMU

---

*All integration checks reference `theme.design.md` for visual specifications. No module may bypass the integration seam — data must flow through the defined IPC protocol and persistence layer, never through shortcuts or backdoors.*
