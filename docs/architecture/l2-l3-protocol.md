# L2↔L3 Protocol — Chat Shell ↔ Intelligence Engine

**Version:** 0.1.0
**Status:** Phase 1 (MVP)

---

## 1. Transport

In MVP, the engine is embedded as a Rust library inside the `levsha-chat` binary. Communication uses `tokio::mpsc` channels — no serialization, no network hop.

```
Chat Shell (L3)                    Engine (L2)
     |                                  |
     |-- ShellToEngine (mpsc) --------> |
     |                                  |
     | <------- EngineToShell (mpsc) -- |
```

Channel capacities:
- Shell → Engine: 64 messages (user input is infrequent)
- Engine → Shell: 256 messages (streaming tokens are high frequency)

---

## 2. Message Types

### 2.1 Shell → Engine (L3 → L2)

| Message | Payload | When |
|---------|---------|------|
| `UserMessage` | `content: String` | User presses Enter to send a message |
| `ConfirmResponse` | `request_id: String, approved: bool` | User approves or rejects a destructive command confirmation |
| `CancelStream` | (none) | User presses Ctrl+C during streaming |

### 2.2 Engine → Shell (L2 → L3)

| Message | Payload | When |
|---------|---------|------|
| `StreamChunk` | `content: String` | Each token from the LLM streaming response |
| `StreamEnd` | (none) | LLM response is complete |
| `ConfirmRequest` | `request_id: String, command: String, description: String` | Engine detected a destructive command and needs user approval |
| `ToolStatus` | `tool_name: String, status: Started\|Completed{success}, description: String` | Tool execution lifecycle events |
| `Error` | `message: String, retryable: bool` | API or system error |
| `ConnectionStatus` | `connected: bool, backend: String` | API connection state changed |
| `HistoryMessage` | `role: MessageRole, content: String, timestamp: i64` | Replaying history on startup |

---

## 3. Message Flow Diagrams

### 3.1 Normal Conversation

```
Shell                          Engine                          Claude API
  |                              |                                |
  |-- UserMessage("hi") ------> |                                |
  |                              |-- POST /messages ------------> |
  |                              | <-- SSE: content_block_delta - |
  | <-- StreamChunk("Hello") -- |                                |
  | <-- StreamChunk(" there")-- |                                |
  |                              | <-- SSE: message_stop -------- |
  | <-- StreamEnd ------------- |                                |
```

### 3.2 Tool Calling

```
Shell                          Engine                          Claude API
  |                              |                                |
  |-- UserMessage("uptime?") -> |                                |
  |                              |-- POST /messages ------------> |
  |                              | <-- tool_use: uptime --------- |
  | <-- ToolStatus(Started) --- |                                |
  |                              |-- exec: `uptime` (L1) ------> |
  |                              | <-- stdout: "up 3 days" ------ |
  | <-- ToolStatus(Completed) - |                                |
  |                              |-- POST /messages (tool_result)>|
  |                              | <-- SSE: text response ------- |
  | <-- StreamChunk(...) ------ |                                |
  | <-- StreamEnd ------------- |                                |
```

### 3.3 Destructive Command Confirmation

```
Shell                          Engine                          Claude API
  |                              |                                |
  |-- UserMessage("rm -rf /tmp")|                                |
  |                              |-- POST /messages ------------> |
  |                              | <-- tool_use: rm -rf /tmp ---- |
  |                              |  (risk classifier: DESTRUCTIVE)|
  | <-- ConfirmRequest -------- |                                |
  |                              |                                |
  |  [user sees confirmation]   |                                |
  |                              |                                |
  |-- ConfirmResponse(yes) ---> |                                |
  |                              |-- exec: `rm -rf /tmp` ------> |
  | <-- ToolStatus(Completed) - |                                |
  |                              |-- POST /messages (result) ---> |
  | <-- StreamChunk(...) ------ |                                |
  | <-- StreamEnd ------------- |                                |
```

### 3.4 Cancellation

```
Shell                          Engine                          Claude API
  |                              |                                |
  |-- UserMessage("explain..") >|                                |
  |                              |-- POST /messages ------------> |
  | <-- StreamChunk(...) ------ | <-- SSE: tokens --------------- |
  | <-- StreamChunk(...) ------ |                                |
  |-- CancelStream -----------> |                                |
  |                              |-- abort HTTP request --------> |
  | <-- StreamEnd ------------- |                                |
  |  (partial response kept)    |                                |
```

### 3.5 Error with Retry

```
Shell                          Engine                          Claude API
  |                              |                                |
  |-- UserMessage("hello") ---> |                                |
  |                              |-- POST /messages ------------> |
  |                              | <-- HTTP 500 ----------------- |
  |                              |-- retry (1s) ----------------> |
  |                              | <-- HTTP 500 ----------------- |
  |                              |-- retry (2s) ----------------> |
  |                              | <-- HTTP 500 ----------------- |
  | <-- Error(retryable=true) - |                                |
  | <-- ConnectionStatus(false) |                                |
```

---

## 4. Rust Types

All types are defined in `engine/src/types.rs`:

- `ShellToEngine` — enum with variants: `UserMessage`, `ConfirmResponse`, `CancelStream`
- `EngineToShell` — enum with variants: `StreamChunk`, `StreamEnd`, `ConfirmRequest`, `ToolStatus`, `Error`, `ConnectionStatus`, `HistoryMessage`
- `ToolExecutionStatus` — `Started` | `Completed { success: bool }`
- `MessageRole` — `User` | `Assistant` | `System` | `ToolCall` | `ToolResult`

Channel creation: `levsha_engine::protocol::create_channels()` returns `(ProtocolShellSide, ProtocolEngineSide)`.

---

## 5. Startup Sequence

1. Chat Shell creates channels via `create_channels()`.
2. Chat Shell spawns the engine on a tokio task, passing it `ProtocolEngineSide`.
3. Engine loads config, connects to API, loads skills.
4. Engine sends `ConnectionStatus { connected: true, backend: "claude-sonnet" }`.
5. Engine queries SQLite for history:
   - If empty: sends `HistoryMessage` with welcome message, stores it in DB.
   - If non-empty: sends all `HistoryMessage`s in chronological order.
6. Chat Shell renders history messages and enables input.
7. Engine enters its main loop, awaiting `ShellToEngine` messages.

---

## 6. Invariants

- Engine never blocks the shell. All engine work is async.
- Shell never blocks on engine. Channel sends are non-blocking (buffered).
- Every user message results in either a `StreamEnd` or an `Error`.
- `ConfirmRequest` must be answered with exactly one `ConfirmResponse`.
- `ToolStatus(Started)` is always followed by `ToolStatus(Completed)`.
- History messages are only sent during startup, before the first `UserMessage`.
