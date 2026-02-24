# Intelligence Engine (L2) — Product Requirements

**Module:** 03-intelligence-engine
**Layer:** L2 — Intelligence Engine
**Phase:** 1 (MVP)

---

## 1. Overview

The Intelligence Engine is the orchestration layer between the Chat Shell (L3) and the Base System (L1). It replaces the traditional shell (bash) and window manager: it interprets user intent from natural language, dispatches system calls through skills, manages conversation context, and composes responses streamed back to the GUI.

In MVP, the engine uses the **Anthropic Claude API** exclusively with a **hardcoded API key**. There is no local model, no model selection, and no user-configurable key. The engine must be ready to accept input immediately after the Chat Shell appears on screen.

---

## 2. Functional Requirements

### 2.1 Cloud API Connection (IE-01)

| Field | Value |
|-------|-------|
| **ID** | IE-01 |
| **Priority** | P0 |
| **Requirement** | Connects to Anthropic Claude API using a hardcoded API key on boot. Ready to accept input immediately after Chat Shell appears. |

**Details:**

- The API key is read from the system configuration file at startup (`/etc/levsha/config.toml`).
- The engine establishes connectivity to the Anthropic API during system initialization, before the Chat Shell signals readiness.
- If the API is unreachable at boot, the engine enters a retry loop and notifies the Chat Shell to display a connection error.
- Model identifier is hardcoded (e.g., `claude-sonnet-4-20250514`). No model selection in MVP.

**Acceptance Criteria:**

- [ ] Engine reads API key from config file at startup.
- [ ] First user message receives a response within 1 second of first token (on reasonable connection).
- [ ] Engine is ready before or simultaneously with Chat Shell readiness.

### 2.2 Tool / Function Calling (IE-02)

| Field | Value |
|-------|-------|
| **ID** | IE-02 |
| **Priority** | P0 |
| **Requirement** | Supports tool/function calling. LLM executes real system commands with full access and returns real output. |

**Details:**

- Tools are defined as JSON schemas in skill manifests. The engine loads all active skill tool definitions at startup and includes them in every API request.
- When the LLM returns a tool call, the engine executes the corresponding system command (shell command, binary invocation, or internal function).
- Tool execution has **full system access** — no sandboxing, no permission model.
- Tool output (stdout, stderr, exit code) is captured and sent back to the LLM as tool results.
- The engine supports multiple sequential tool calls within a single turn (the LLM may call several tools before composing a final text response).

**Acceptance Criteria:**

- [ ] Engine parses tool call responses from the Claude API.
- [ ] Tool definitions from all active skills are included in API requests.
- [ ] System commands execute with full root access.
- [ ] Stdout, stderr, and exit code are captured and returned as tool results.
- [ ] Multi-step tool calling works (LLM calls tool A, gets result, calls tool B, gets result, then responds).

### 2.3 Destructive Command Confirmation (IE-03)

| Field | Value |
|-------|-------|
| **ID** | IE-03 |
| **Priority** | P0 |
| **Requirement** | Destructive commands require explicit user confirmation displayed as a visually distinct prompt in the chat. |

**Details:**

- The engine maintains a pattern list of destructive commands (e.g., `rm -rf`, `mkfs`, `dd`, `fdisk`, `wipefs`, package removal, systemctl operations on critical services).
- Before executing any command matching a destructive pattern, the engine pauses execution and sends a confirmation request to the Chat Shell.
- The Chat Shell renders a visually distinct confirmation prompt (not a regular message).
- The user must explicitly approve or reject. Rejection cancels the command and informs the LLM.
- The classification operates on the actual command string, not the user's natural language input.

**Acceptance Criteria:**

- [ ] Destructive patterns cover: `rm -rf`, `rm -r /`, `mkfs.*`, `dd if=`, `fdisk`, `wipefs`, `parted`, `systemctl stop/disable` on core services, bulk package removal.
- [ ] Confirmation request is sent to Chat Shell before execution.
- [ ] User approval triggers execution; rejection cancels and informs the LLM.
- [ ] Non-destructive commands execute without confirmation.

### 2.4 Context Window Management (IE-04)

| Field | Value |
|-------|-------|
| **ID** | IE-04 |
| **Priority** | P0 |
| **Requirement** | Context window includes: system prompt, active skill prompts, and recent conversation history (rolling window). |

**Details:**

The context sent to the API on each request is composed of three parts, in order:

1. **System Prompt** — Base system identity and behavioral instructions. Always present, never truncated.
2. **Active Skill Prompts** — Prompt fragments from all loaded skills, appended to the system prompt. Always present, never truncated.
3. **Conversation History** — A rolling window of recent messages (user + assistant + tool calls/results). Truncated from the oldest messages when the total context approaches the model's context limit.

- Token counting uses a conservative estimate (4 characters per token) to stay within limits.
- A configurable reserve (default: 4096 tokens) is maintained for the model's response.
- When truncation occurs, a system message is prepended to the history indicating that older messages were removed.

**Acceptance Criteria:**

- [ ] System prompt and skill prompts are always included, never truncated.
- [ ] Conversation history is a rolling window, oldest messages dropped first.
- [ ] Context never exceeds the model's context limit.
- [ ] Token counting is conservative and avoids context overflow errors.
- [ ] Truncation notification is included when messages are dropped.

### 2.5 Error Handling (IE-05)

| Field | Value |
|-------|-------|
| **ID** | IE-05 |
| **Priority** | P0 |
| **Requirement** | Graceful error handling: API failures, timeouts, and rate limits are surfaced as friendly messages in the chat with retry options. |

**Details:**

The engine handles the following error classes:

| Error | Behavior |
|-------|----------|
| Network unreachable | Retry with exponential backoff (3 attempts, 1s/2s/4s). Then surface error to Chat Shell with manual retry option. |
| HTTP 401 (auth failure) | Surface immediately: "API key is invalid or expired." No retry. |
| HTTP 429 (rate limit) | Parse `Retry-After` header. Wait and auto-retry. If wait exceeds 30s, surface to user with estimated wait time. |
| HTTP 500/502/503 (server error) | Retry with exponential backoff (3 attempts). Then surface error. |
| Request timeout (30s default) | Cancel request, surface timeout message, offer retry. |
| Malformed API response | Log the response body, surface a generic error, offer retry. |
| Tool execution failure | Return the error (stderr + exit code) to the LLM as a tool result. Let the LLM decide how to communicate it to the user. |

**Acceptance Criteria:**

- [ ] All error classes listed above are handled without panicking.
- [ ] User sees a friendly error message (not raw HTTP errors or stack traces).
- [ ] Retry is offered for recoverable errors.
- [ ] Rate limit handling respects `Retry-After` headers.
- [ ] Tool execution errors are returned to the LLM, not swallowed.

---

## 3. Non-Functional Requirements

| Requirement | Target |
|-------------|--------|
| First-token latency | < 1 second (reasonable internet connection) |
| Streaming support | Token-by-token delivery to Chat Shell |
| Memory overhead | Engine process < 50 MB RSS at idle |
| Startup time | Engine ready within 2 seconds of process launch |
| Concurrent requests | Single request at a time (single session, MVP) |

---

## 4. Interfaces

### 4.1 Chat Shell Interface (L3 -> L2)

The Chat Shell communicates with the engine via an IPC mechanism (Unix domain socket or in-process channel). Messages:

| Direction | Message | Payload |
|-----------|---------|---------|
| L3 -> L2 | `UserMessage` | Text content from user input |
| L2 -> L3 | `StreamChunk` | Partial text token from LLM response |
| L2 -> L3 | `StreamEnd` | End of response stream |
| L2 -> L3 | `ConfirmRequest` | Destructive command details, awaiting user approval |
| L3 -> L2 | `ConfirmResponse` | User's approval or rejection |
| L2 -> L3 | `ToolStatus` | Tool execution started/completed (for progress indicators) |
| L2 -> L3 | `Error` | Error message with optional retry action |

### 4.2 Base System Interface (L2 -> L1)

Tool execution runs system commands via `std::process::Command` with:

- Full root privileges (the engine runs as root in MVP).
- Captured stdout, stderr, and exit code.
- Configurable timeout per command (default: 60 seconds).

### 4.3 Skill System Interface

- Skills are loaded from `/usr/share/levsha/skills/` at startup.
- Each skill provides a prompt fragment (markdown) and tool definitions (JSON schema).
- The engine merges all active skill prompts into the system prompt and all tool definitions into the API request.

---

## 5. Configuration

Configuration file: `/etc/levsha/config.toml`

```toml
[api]
key = "sk-ant-..."          # Hardcoded API key (MVP)
model = "claude-sonnet-4-20250514"  # Model identifier
base_url = "https://api.anthropic.com"
timeout_seconds = 30
max_retries = 3

[context]
max_tokens = 200000          # Model context limit
response_reserve = 4096      # Reserved tokens for response
chars_per_token = 4          # Conservative token estimate

[execution]
command_timeout_seconds = 60 # Per-command execution timeout

[skills]
path = "/usr/share/levsha/skills"
```

---

## 6. Out of Scope (MVP)

- Local LLM support
- Model selection or switching
- User-configurable API key
- Multi-session / concurrent conversations
- Skill install/remove at runtime
- Conversation summarization for context compression
- Prompt caching or optimization
- Telemetry or usage tracking
