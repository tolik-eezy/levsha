# Intelligence Engine (L2) — Design Document

**Module:** 03-intelligence-engine
**Layer:** L2 — Intelligence Engine
**Phase:** 1 (MVP)

---

## 1. Architecture Overview

The Intelligence Engine is a single Rust process that mediates all communication between the Chat Shell (L3) and the Base System (L1). It owns the conversation state, constructs API requests, parses streaming responses, executes tool calls, and enforces destructive command confirmation.

```
┌───────────────┐
│  Chat Shell   │  L3 — Wayland GUI
│   (Rust)      │
└──────┬────────┘
       │  IPC (Unix socket or in-process channel)
       ▼
┌───────────────────────────────────────────────┐
│           Intelligence Engine                  │  L2
│                                                │
│  ┌──────────┐  ┌───────────┐  ┌────────────┐ │
│  │ Context   │  │ API       │  │ Tool       │ │
│  │ Manager   │  │ Client    │  │ Executor   │ │
│  └──────────┘  └───────────┘  └────────────┘ │
│  ┌──────────┐  ┌───────────┐  ┌────────────┐ │
│  │ Prompt    │  │ Stream    │  │ Destructive│ │
│  │ Composer  │  │ Parser    │  │ Guard      │ │
│  └──────────┘  └───────────┘  └────────────┘ │
└───────────────────────────────────────────────┘
       │
       │  std::process::Command (root)
       ▼
┌───────────────┐
│  Base System  │  L1 — Linux (Fedora)
│   (Linux)     │
└───────────────┘
```

### Internal Components

| Component | Responsibility |
|-----------|---------------|
| **Context Manager** | Maintains conversation history, handles rolling window truncation, token counting. |
| **Prompt Composer** | Assembles the system prompt from base template + active skill prompt fragments. |
| **API Client** | Sends HTTP requests to the Anthropic API, handles auth, retries, error classification. |
| **Stream Parser** | Parses Server-Sent Events (SSE) from the streaming API response, emits text chunks and tool call events. |
| **Tool Executor** | Runs system commands, captures output, enforces timeouts. |
| **Destructive Guard** | Inspects commands before execution, blocks destructive commands pending user confirmation. |

---

## 2. Request Pipeline

A single user message flows through the engine as follows:

```
1. Chat Shell sends UserMessage via IPC
                │
2. Context Manager appends user message to history
                │
3. Prompt Composer builds the system prompt:
   [base prompt] + [skill prompt: pkg-manager] + [skill prompt: sysinfo]
                │
4. Context Manager composes the messages array:
   - Conversation history (truncated to fit context window)
   - New user message at the end
                │
5. API Client sends POST /v1/messages with:
   - system: composed system prompt
   - messages: conversation history
   - tools: merged tool definitions from all skills
   - stream: true
                │
6. Stream Parser reads SSE events:
   ├── content_block_delta (text) → StreamChunk to Chat Shell
   ├── content_block_delta (tool_use) → accumulate tool call
   └── message_stop → finalize
                │
7. If tool call(s) received:
   ├── Destructive Guard checks each command
   │   ├── Safe → Tool Executor runs it
   │   └── Destructive → ConfirmRequest to Chat Shell
   │       ├── Approved → Tool Executor runs it
   │       └── Rejected → tool result = "User cancelled this command"
   ├── Tool results appended to conversation
   └── GOTO step 5 (continue the turn with tool results)
                │
8. Final text response → StreamEnd to Chat Shell
                │
9. Context Manager appends assistant response to history
```

### Turn Loop

A single user turn may involve multiple API roundtrips. The LLM can call tools, receive results, call more tools, and eventually produce a final text response. The engine loops through steps 5-7 until the LLM produces a response with no tool calls (only text content).

Maximum tool call iterations per turn: **10** (configurable). If exceeded, the engine appends an error message and stops the loop.

---

## 3. System Prompt Composition

The system prompt is composed at startup and refreshed only when skills change (not in MVP — skills are static).

### Structure

```
[Base System Prompt]
  - Identity: "You are Levsha OS..."
  - Behavioral rules: be concise, use the system's formatting
  - System context: OS version, hostname, current time
  - Tool usage instructions: prefer tools over guessing

[Skill: Package Manager]
  - Package manager type (dnf)
  - Available tool descriptions
  - Response formatting guidance

[Skill: System Info]
  - Available system query tools
  - Response formatting guidance (tables, units)
```

### Base System Prompt Template

The base prompt is a static markdown file at `/usr/share/levsha/prompts/system.md` with template variables:

- `{{os_version}}` — Levsha OS version string
- `{{hostname}}` — System hostname
- `{{datetime}}` — Current date and time (refreshed each request)
- `{{skills_summary}}` — One-line summary of each active skill

Skill prompts are appended verbatim after the base prompt, separated by markdown headers.

---

## 4. Tool Calling Schema and Execution Model

### Tool Definition Format

Tools are defined in JSON Schema format per the Anthropic API specification. Each skill provides one or more tool JSON files:

```json
{
  "name": "run_command",
  "description": "Execute a shell command on the system and return its output.",
  "input_schema": {
    "type": "object",
    "properties": {
      "command": {
        "type": "string",
        "description": "The shell command to execute"
      },
      "timeout_seconds": {
        "type": "integer",
        "description": "Maximum execution time in seconds (default: 60)"
      }
    },
    "required": ["command"]
  }
}
```

### Execution Model

1. The API response contains `tool_use` content blocks with a tool `name`, `id`, and `input` object.
2. The engine resolves the tool name to a registered handler.
3. The handler executes the operation (typically a shell command via `std::process::Command`).
4. The result is formatted as a `tool_result` message with the tool's `id`, stdout content, and an `is_error` flag if the command failed.
5. The conversation continues with the tool result appended.

### Core Tools (MVP)

| Tool | Source | Description |
|------|--------|-------------|
| `run_command` | Built-in | Execute any shell command. The universal fallback. |
| `pkg_install` | Package Manager skill | Install packages via dnf. |
| `pkg_remove` | Package Manager skill | Remove packages via dnf. |
| `pkg_search` | Package Manager skill | Search for packages. |
| `pkg_update` | Package Manager skill | Update all packages. |
| `pkg_list` | Package Manager skill | List installed packages. |
| `sys_disk` | System Info skill | Report disk usage. |
| `sys_memory` | System Info skill | Report memory usage. |
| `sys_cpu` | System Info skill | Report CPU info and load. |
| `sys_uptime` | System Info skill | Report system uptime. |
| `sys_network` | System Info skill | Report network status. |
| `sys_processes` | System Info skill | List running processes. |

---

## 5. Destructive Command Classification

### Pattern Matching

The Destructive Guard operates on the resolved shell command string (not the user's natural language). It applies a set of regex patterns:

```
# Filesystem destruction
rm\s+(-[a-zA-Z]*r[a-zA-Z]*|--recursive)\s
rm\s+(-[a-zA-Z]*f[a-zA-Z]*)\s+/
mkfs\.
dd\s+.*if=
wipefs\s

# Disk/partition modification
fdisk\s
parted\s
gdisk\s

# Service disruption
systemctl\s+(stop|disable|mask)\s+(sshd|systemd-.*|levsha-.*|NetworkManager)
reboot
shutdown
poweroff
init\s+[06]

# Package removal (bulk)
dnf\s+(-y\s+)?remove\s
```

### Classification Tiers

| Tier | Action | Examples |
|------|--------|---------|
| **Safe** | Execute immediately | `ls`, `cat`, `df`, `free`, `dnf search`, `ps` |
| **Destructive** | Block, request user confirmation | `rm -rf`, `mkfs.ext4`, `dd if=/dev/zero`, `reboot` |

MVP uses a binary classification (safe/destructive). More granular risk tiers (warning, caution, critical) are deferred to Phase 2.

### Confirmation Flow

```
Engine detects destructive command
       │
       ▼
Engine sends ConfirmRequest to Chat Shell:
  {
    command: "rm -rf /tmp/old-data",
    reason: "This will permanently delete files",
    risk: "destructive"
  }
       │
       ▼
Chat Shell renders visually distinct prompt:
  ┌─────────────────────────────────────────┐
  │  ⚠ This command will permanently delete │
  │    files. Proceed?                       │
  │                                          │
  │  rm -rf /tmp/old-data                   │
  │                                          │
  │  [ Confirm ]    [ Cancel ]              │
  └─────────────────────────────────────────┘
       │
       ▼
User responds → ConfirmResponse to Engine
       │
       ├── Approved → execute command, return result to LLM
       └── Rejected → return "User cancelled this command" to LLM
```

---

## 6. Context Window Management Strategy

### Token Budget

```
Total context limit:          200,000 tokens
  - System prompt (base):       ~800 tokens (fixed)
  - Skill prompts:              ~1,200 tokens (fixed, 2 skills in MVP)
  - Response reserve:           4,096 tokens (configurable)
  - Available for history:     ~193,904 tokens
```

### Truncation Algorithm

1. Before each API request, calculate total token usage:
   `system_tokens + skill_tokens + history_tokens + response_reserve`
2. If total exceeds the model's context limit:
   a. Remove the oldest message pair (user + assistant) from history.
   b. Recalculate. Repeat until within budget.
3. If truncation occurred, prepend a system-injected user context message:
   `"[Earlier messages in this conversation were removed to fit the context window.]"`

### Token Estimation

MVP uses a conservative character-based estimate: **1 token = 4 characters**. This overestimates slightly, which is safe — it means we truncate slightly earlier rather than risk overflow. A tokenizer library (tiktoken-rs or similar) can replace this in Phase 2.

---

## 7. Streaming Response Handling

### SSE Protocol

The Anthropic API streams responses as Server-Sent Events. The engine parses the stream incrementally:

```
event: message_start
data: {"type":"message_start","message":{...}}

event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text"}}

event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

event: content_block_stop
data: {"type":"content_block_stop","index":0}

event: message_stop
data: {"type":"message_stop"}
```

### Stream Processing

1. **Text deltas** — Each `text_delta` is immediately forwarded to the Chat Shell as a `StreamChunk`. No buffering.
2. **Tool use deltas** — `input_json_delta` events are accumulated into a complete JSON object. The tool call is dispatched only after the content block is complete (`content_block_stop`).
3. **Message stop** — Signals the end of the response. If tool calls were made, the engine enters the tool execution loop. If only text was produced, `StreamEnd` is sent to the Chat Shell.
4. **Error events** — `event: error` triggers error handling per IE-05.

---

## 8. Error Recovery Design

### Retry Strategy

All retryable HTTP errors use exponential backoff:

```
Attempt 1: immediate
Attempt 2: wait 1 second
Attempt 3: wait 2 seconds
Attempt 4 (if configured): wait 4 seconds
```

If all retries are exhausted, the error is surfaced to the Chat Shell.

### Partial Stream Recovery

If the SSE connection drops mid-stream:

1. The engine marks the partial response as incomplete.
2. The partial text already streamed to Chat Shell remains visible.
3. An error message is appended: "Response was interrupted. [Retry]"
4. On retry, the full conversation (including the partial response attempt) is resent to the API.

### State Consistency

- The conversation history only commits a complete assistant response after `message_stop`.
- Failed or interrupted responses are not persisted to the SQLite database.
- Tool calls that have already executed are not re-executed on retry; their results are included in the conversation context.

---

## 9. Interface with Chat Shell

### IPC Mechanism Options

**Option A: Unix Domain Socket** (recommended for MVP)

- Engine runs as a separate systemd service.
- Chat Shell connects via Unix socket at `/run/levsha/engine.sock`.
- Messages are length-prefixed JSON.
- Allows independent restart of Chat Shell without losing engine state.

**Option B: In-Process Channel (Rust mpsc)**

- Engine is a library linked into the Chat Shell binary.
- Communication via `tokio::sync::mpsc` channels.
- Simpler deployment (single binary) but coupled lifecycle.

**Recommendation:** Option A for MVP. The separate process model allows the Chat Shell to crash and restart (via systemd) without losing conversation state held in the engine. This aligns with the reliability requirement (CS-07, IE-05).

### Message Protocol

All IPC messages are JSON with a `type` discriminator:

```json
{"type": "user_message", "content": "install nginx"}
{"type": "stream_chunk", "text": "I'll install nginx for you."}
{"type": "stream_end"}
{"type": "confirm_request", "id": "abc123", "command": "dnf install -y nginx", "reason": "Package installation"}
{"type": "confirm_response", "id": "abc123", "approved": true}
{"type": "tool_status", "tool": "pkg_install", "state": "running"}
{"type": "tool_status", "tool": "pkg_install", "state": "completed"}
{"type": "error", "message": "API connection failed", "retryable": true}
```

---

## 10. Data Flow Summary

```
                    ┌──────────┐
                    │  SQLite  │  Persistence (L2 owns the DB)
                    └────┬─────┘
                         │ read/write history
┌──────────┐        ┌────┴─────────┐        ┌──────────────────┐
│  Chat    │  IPC   │ Intelligence │  HTTPS  │  Anthropic API   │
│  Shell   │◄──────►│   Engine     │◄───────►│  (Claude)        │
│  (L3)    │        │   (L2)       │         │                  │
└──────────┘        └────┬─────────┘        └──────────────────┘
                         │
                         │ std::process::Command
                         ▼
                    ┌──────────┐
                    │  Linux   │  Base System (L1)
                    │  (root)  │
                    └──────────┘
```

The engine is the central hub. It owns the conversation database, mediates all LLM communication, and is the sole executor of system commands. The Chat Shell is a pure rendering client that sends user text and receives streamed responses.
