# 07 — Persistence: Product Requirements

**Module:** Conversation Persistence (SQLite)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

Persistence in Levsha OS ensures that the conversation history — the user's primary interface with the system — survives reboots, crashes, and power failures. Since the chat is the computer, losing conversation history is equivalent to losing all open documents and running applications in a traditional OS.

The persistence layer uses SQLite with write-ahead logging (WAL) to store every message exchanged between the user and the system. There is no separate save action; all messages are persisted immediately as they are produced.

---

## 2. Scope

### In Scope (MVP)

- Full conversation history stored in a local SQLite database
- Write-ahead logging (WAL) for crash safety — no data loss on power failure
- Automatic persistence of all message types (user, assistant, system, tool calls, tool results)
- History retrieval for display in the Chat Shell (scrollback)
- History retrieval for the Intelligence Engine (context window construction)
- User-initiated history clear via chat command ("clear history", "start fresh")

### Out of Scope (MVP)

- Multiple conversation sessions (single session only)
- Conversation search/filter (Ctrl+F search is handled by Chat Shell, not persistence)
- Export/import of conversation history
- Cloud backup or sync
- Conversation branching or forking
- Automatic history pruning or archival

---

## 3. Functional Requirements

### 3.1 Storage

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| PS-01 | All conversation messages are stored in a local SQLite database. | P0 | PM-01 |
| PS-02 | The database uses WAL (Write-Ahead Logging) mode. Writes do not block reads. | P0 | 8.4 |
| PS-03 | Every message is persisted to the WAL before being acknowledged as stored. No data loss on unexpected shutdown or VM crash. | P0 | 8.4 |
| PS-04 | The database is stored at a fixed, well-known filesystem path (`/var/lib/levsha/history.db`). | P0 | — |
| PS-05 | The database and its WAL/SHM files are owned by the `levsha` user. | P0 | — |

### 3.2 Message Types

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| PS-10 | User messages are stored with full text content and timestamp. | P0 | PM-01 |
| PS-11 | Assistant messages are stored with full text content, timestamp, and completion status (streaming complete vs. interrupted). | P0 | PM-01 |
| PS-12 | Tool call messages are stored with the tool name, arguments, and invocation timestamp. | P0 | PM-01 |
| PS-13 | Tool result messages are stored with output content, exit code, and completion timestamp. | P0 | PM-01 |
| PS-14 | System messages (errors, status updates, welcome message) are stored with type and timestamp. | P0 | PM-01 |

### 3.3 Retrieval

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| PS-20 | The Chat Shell can retrieve the full conversation history for scrollback display on startup. | P0 | PM-01 |
| PS-21 | The Intelligence Engine can retrieve the N most recent messages for context window construction. | P0 | IE-04 |
| PS-22 | Messages are retrieved in chronological order. | P0 | — |
| PS-23 | History retrieval for Chat Shell display completes within 100ms for up to 10,000 messages. | P0 | — |

### 3.4 Clear History

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| PS-30 | User can clear all conversation history via chat ("clear history", "start fresh"). | P0 | PM-02 |
| PS-31 | Clearing history requires explicit confirmation via the destructive command confirmation flow. | P0 | IE-03 |
| PS-32 | After clearing, the Chat Shell displays a fresh welcome message as if on first boot. | P0 | PM-02 |
| PS-33 | Clear is a soft delete — the database is vacuumed, but no recovery mechanism is needed in MVP. | P0 | — |

---

## 4. Non-Functional Requirements

| Requirement | Target |
|-------------|--------|
| Write latency (single message) | < 5 ms |
| Read latency (last 100 messages) | < 20 ms |
| Full history load (10,000 messages) | < 100 ms |
| Database size (10,000 messages) | < 50 MB |
| Data durability | No data loss on power failure (WAL) |
| Concurrent access | Single writer (engine), multiple readers (shell, engine) |

---

## 5. Acceptance Criteria

1. **Reboot persistence:** User sends 10 messages, VM is rebooted, all 10 messages appear in the Chat Shell on next boot.
2. **Crash safety:** User sends a message, VM is force-killed (`kill -9` on QEMU), database is intact on restart with no corruption.
3. **Power failure simulation:** Pulling VM power during a streaming response preserves all messages up to the last committed write.
4. **Clear history:** User says "clear history", confirms, Chat Shell resets to welcome message with empty history.
5. **Context loading:** Intelligence Engine loads the last 50 messages from the database and includes them in the API request.
6. **Performance:** Writing 1,000 messages sequentially completes within 5 seconds total.
7. **Scrollback:** Chat Shell renders the full conversation history on startup without visual lag.

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| L2 (Intelligence Engine) | Bidirectional | Engine writes messages to the database and reads history for context window construction. |
| L3 (Chat Shell) | Consumer | Chat Shell reads history for display on startup and scrollback. |
| L1 (Base System) | Infrastructure | Filesystem and user permissions for database storage. |

---

## 7. Traceability

| PRD Requirement | Persistence Requirement |
|-----------------|------------------------|
| PM-01 (history survives reboot) | PS-01, PS-02, PS-03, PS-10 through PS-14 |
| PM-02 (clear history via chat) | PS-30, PS-31, PS-32, PS-33 |
| IE-04 (context includes history) | PS-21 |
| 8.4 (WAL, no data loss) | PS-02, PS-03 |
| 8.4 (crash recovery) | PS-03 |
