# 00 — System Architecture: Design Document

**Module:** System Architecture (4-Layer Model)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Architectural Overview

Levsha OS uses a strict 4-layer architecture. Each layer communicates only with its immediate neighbors. The LLM Intelligence Engine (L2) is the central orchestration layer — it replaces the role of a traditional shell and window manager.

```
+------------------------------------------------------+
|                    L3 — Chat Shell                    |
|          Wayland-native GUI (Rust + GTK4)             |
|  Renders conversation, captures input, status bar     |
+----------------------------+-------------------------+
                             |
                        IPC (local)
                             |
+----------------------------+-------------------------+
|                 L2 — Intelligence Engine               |
|        Intent parsing, skill dispatch, tool calling    |
|        Claude API client, context management           |
+----------+-------------------+-----------------------+
           |                   |
      Claude API          Tool execution
      (HTTPS)             (subprocess)
           |                   |
+----------+-------------------+-----------------------+
|                   L1 — Base System                     |
|  Fedora minimal, systemd, Wayland compositor,          |
|  networking (DHCP), filesystem, audio/video             |
+------------------------------------------------------+
|                    L0 — Kernel                         |
|          Linux kernel (Fedora default)                 |
+------------------------------------------------------+
```

---

## 2. Layer Responsibilities and Boundaries

### L0 — Kernel

**Owns:** Hardware abstraction, process scheduling, memory management, device drivers, virtual filesystem layer.

**Boundary:** L0 is entirely managed by Fedora's default kernel. No custom kernel patches or modules in MVP. L1 configures kernel parameters via sysctl and module loading at boot.

**Does not own:** Anything user-visible. L0 is invisible to L1 and above except through standard syscalls.

### L1 — Base System

**Owns:**
- Boot sequence (UEFI/BIOS -> GRUB -> kernel -> systemd)
- Service management (systemd units for compositor, Chat Shell, Intelligence Engine)
- Wayland compositor (wlroots-based or Mutter)
- Network auto-configuration (NetworkManager, DHCP)
- Filesystem layout and mounts
- Audio/video subsystem (PipeWire)
- System packages (dnf)

**Boundary:** L1 provides the runtime environment. It launches the Wayland compositor, then starts L3 (Chat Shell) and L2 (Intelligence Engine) as systemd services. L1 does not interpret user intent — it only executes commands issued by L2 via subprocess calls.

**Does not own:** User interaction, LLM communication, conversation state.

### L2 — Intelligence Engine

**Owns:**
- Claude API client (HTTPS, streaming)
- Intent parsing (delegated to the LLM)
- Skill orchestration (loading skill prompts and tool schemas)
- Tool calling (executing system commands, returning output to LLM)
- Conversation context management (system prompt + skill prompts + rolling history)
- Conversation persistence (SQLite)
- Destructive command detection and confirmation flow

**Boundary:** L2 is a long-running daemon. It receives user messages from L3 via local IPC, constructs API requests, handles streaming responses, and sends rendered output back to L3. When the LLM requests a tool call, L2 spawns a subprocess on L1, captures output, and feeds it back into the conversation.

**Does not own:** GUI rendering, visual layout, input capture. L2 produces structured data (text, code blocks, tables, confirmation prompts); L3 decides how to render them.

### L3 — Chat Shell

**Owns:**
- Full-screen Wayland GUI rendering
- User input capture (keyboard, mouse)
- Conversation display (message bubbles, code blocks, syntax highlighting, tables)
- Streaming token rendering (smooth, token-by-token display)
- Status bar (time, connection status, AI backend indicator)
- Animations and visual transitions
- Error display (API unreachable, timeouts)
- Destructive command confirmation UI

**Boundary:** L3 is a pure presentation layer. It sends raw user text to L2 and renders whatever L2 sends back. L3 does not parse intent, invoke commands, or manage conversation history.

**Does not own:** Business logic, system command execution, API communication, persistence.

---

## 3. Data Flow

### 3.1 Normal Message Flow

```
User keystroke
    |
    v
L3: Captures text in input field
L3: User presses Enter
L3: Sends message to L2 via IPC   ------>  L2: Receives message
L3: Shows user message bubble              L2: Appends to conversation history
L3: Shows "thinking" indicator             L2: Builds API request:
                                               - system prompt
                                               - skill prompts (pkg mgr, sysinfo)
                                               - conversation history (rolling window)
                                               - user message
                                           L2: Sends to Claude API (HTTPS, streaming)
                                               |
                                               v
                                           Claude API returns streaming response
                                               |
                                               v
L3: Receives token stream  <------  L2: Forwards tokens to L3 via IPC
L3: Renders tokens one by one              L2: Appends full response to history
L3: Scrolls to bottom                     L2: Persists to SQLite
```

### 3.2 Tool Call Flow

```
Claude API response includes a tool_use block
    |
    v
L2: Parses tool call (e.g., execute_command: "df -h")
L2: Checks if command is destructive
    |
    +-- If destructive:
    |       L2: Sends confirmation request to L3
    |       L3: Renders confirmation prompt (visually distinct)
    |       User approves or denies
    |       L3: Sends decision to L2
    |       If denied: L2 sends "cancelled" to API, flow ends
    |
    +-- If safe (or approved):
            L2: Spawns subprocess on L1: /bin/sh -c "df -h"
            L2: Captures stdout + stderr
            L2: Sends tool_result back to Claude API
            Claude API returns final response with formatted output
            L2: Forwards to L3 for rendering
```

### 3.3 Error Flow (API Unreachable)

```
L2: Sends request to Claude API
L2: Connection fails / times out
    |
    v
L2: Sends error event to L3:
    { type: "error", code: "api_unreachable", retryable: true }
    |
    v
L3: Renders styled error message:
    "I can't reach the AI backend right now. [Retry]"
L3: User clicks Retry
L3: Sends retry signal to L2
L2: Re-sends the last API request
```

---

## 4. Interface Contracts

### 4.1 L3 <-> L2 (IPC Protocol)

Communication between the Chat Shell and the Intelligence Engine uses a local Unix domain socket with a simple JSON-lines protocol.

**L3 -> L2 (upstream):**

| Message Type | Payload | Description |
|---|---|---|
| `user_message` | `{ text: string }` | User submitted a message |
| `confirm_response` | `{ request_id: string, approved: bool }` | User responded to a destructive command confirmation |
| `retry` | `{}` | User requested retry after an error |
| `clear_history` | `{}` | User requested history clear |

**L2 -> L3 (downstream):**

| Message Type | Payload | Description |
|---|---|---|
| `token` | `{ text: string }` | Single token from streaming response |
| `response_start` | `{}` | LLM response started |
| `response_end` | `{}` | LLM response completed |
| `confirm_request` | `{ request_id: string, command: string, risk: string }` | Destructive command needs user approval |
| `error` | `{ code: string, message: string, retryable: bool }` | Error occurred |
| `status` | `{ api_connected: bool, model: string }` | Status update for status bar |

### 4.2 L2 <-> Claude API (HTTPS)

Standard Anthropic Messages API over HTTPS with streaming enabled. L2 constructs the request:

```
POST https://api.anthropic.com/v1/messages
Headers:
  x-api-key: <hardcoded key>
  anthropic-version: 2023-06-01
  content-type: application/json
Body:
  {
    model: "claude-sonnet-4-20250514",
    max_tokens: 4096,
    stream: true,
    system: "<system prompt + skill prompts>",
    tools: [<tool schemas from active skills>],
    messages: [<conversation history>]
  }
```

### 4.3 L2 <-> L1 (Tool Execution)

L2 executes tools by spawning subprocesses:

```
Command:  /bin/sh -c "<command from tool call>"
CWD:      /home/levsha
User:     root (MVP — no privilege separation)
Timeout:  30 seconds (configurable per tool)
Capture:  stdout + stderr, combined
```

L2 returns the captured output as a `tool_result` in the next API request.

---

## 5. Key Architectural Decisions

| Decision | Choice | Rationale |
|---|---|---|
| **L2 is a separate daemon, not a library linked into L3** | Separate process | Crash isolation: L3 crash does not lose conversation state. L2 can be restarted independently. Cleaner separation of concerns. |
| **IPC via Unix socket, not D-Bus** | Unix socket + JSON-lines | Simpler to implement, debug, and test. No D-Bus dependency. Sufficient for MVP's single-session model. |
| **L2 owns persistence, not L3** | L2 writes SQLite | Conversation state is L2's domain. L3 is a pure renderer. On restart, L3 requests history from L2. |
| **No sandboxing for tool execution** | Full root access | MVP is single-user in a VM. Sandboxing adds complexity without benefit. Destructive command confirmation is the safety mechanism. |
| **Streaming at the IPC level** | Token-by-token forwarding | L2 forwards each token as it arrives from the API. L3 renders immediately. No buffering. This enables smooth streaming UX. |
| **Fedora as base distro** | Fedora minimal | Best GTK4/libadwaita support, excellent Wayland maturity, polished font rendering, strong ISO tooling (lorax). |
| **Single Wayland compositor** | wlroots-based or Mutter | The compositor runs a single full-screen surface (L3). No window management needed. Minimal compositor config. |

---

## 6. Process Topology (Runtime)

```
systemd (PID 1)
  |
  +-- wayland-compositor (e.g., cage or labwc)
  |     |
  |     +-- levsha-chat-shell (L3)   <-- full-screen Wayland client
  |
  +-- levsha-engine (L2)             <-- intelligence engine daemon
  |     |
  |     +-- (child processes for tool calls, transient)
  |
  +-- NetworkManager
  +-- pipewire
  +-- systemd-journald
```

- `cage` or `labwc` is a minimal Wayland compositor that runs a single application full-screen.
- `levsha-chat-shell` is the Rust GUI binary.
- `levsha-engine` is the Intelligence Engine daemon.
- Both are managed by systemd units with `Restart=always`.

---

## 7. Failure Modes

| Failure | Detection | Recovery |
|---|---|---|
| L3 (Chat Shell) crashes | systemd detects process exit | systemd restarts L3. L3 reconnects to L2, requests conversation history, re-renders. |
| L2 (Engine) crashes | systemd detects process exit | systemd restarts L2. L2 reloads conversation from SQLite. L3 reconnects automatically. |
| API unreachable | L2 connection timeout or HTTP error | L2 sends error event to L3. L3 shows styled error with retry button. |
| Tool call hangs | L2 subprocess timeout (30s) | L2 kills subprocess, returns timeout error to API, surfaces message to user. |
| Compositor crashes | systemd detects process exit | systemd restarts compositor + L3. L2 is unaffected (headless daemon). |
