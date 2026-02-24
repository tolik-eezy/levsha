# 07 — Persistence: Design

> **Visual styling follows [theme.design.md](../theme.design.md).**

**Module:** Conversation Persistence (SQLite)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

The persistence layer is a SQLite database that stores the complete conversation history for Levsha OS. It is the single source of truth for all messages exchanged between the user and the system. Both the Intelligence Engine (L2) and the Chat Shell (L3) interact with this database — L2 as the primary writer and context reader, L3 as the display reader.

---

## 2. Database Location

```
/var/lib/levsha/history.db        # Main database file
/var/lib/levsha/history.db-wal    # Write-ahead log (auto-managed)
/var/lib/levsha/history.db-shm    # Shared memory file (auto-managed)
```

- Directory owned by `levsha:levsha`, mode `0700`.
- Database file mode `0600`.
- The directory is created by the systemd service unit on first boot.

---

## 3. SQLite Configuration

```sql
PRAGMA journal_mode = WAL;          -- Write-ahead logging for crash safety
PRAGMA synchronous = NORMAL;        -- Sync WAL to disk at checkpoints (good durability/performance tradeoff)
PRAGMA wal_autocheckpoint = 1000;   -- Checkpoint every 1000 pages
PRAGMA foreign_keys = ON;           -- Enforce referential integrity
PRAGMA busy_timeout = 5000;         -- Wait up to 5s for locks
```

WAL mode ensures:
- Writes do not block reads (Chat Shell can scroll while Engine writes).
- A crash mid-write only loses the uncommitted transaction, never corrupts committed data.
- `synchronous = NORMAL` provides durability at WAL commit boundaries without the overhead of `FULL`.

---

## 4. Database Schema

### 4.1 Conversations Table

Tracks the single conversation session. Designed for future multi-session support but contains only one row in MVP.

```sql
CREATE TABLE conversations (
    id          INTEGER PRIMARY KEY,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    title       TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1
);
```

### 4.2 Messages Table

Stores every message in the conversation.

```sql
CREATE TABLE messages (
    id              INTEGER PRIMARY KEY,
    conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role            TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool_call', 'tool_result')),
    content         TEXT NOT NULL,
    metadata        TEXT,          -- JSON blob for role-specific data
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
    is_complete     INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_messages_created_at ON messages(created_at);
```

### 4.3 Schema Version Table

Tracks schema migrations for forward compatibility.

```sql
CREATE TABLE schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
);
```

---

## 5. Data Model

### 5.1 Message Roles and Metadata

| Role | Description | Metadata (JSON) |
|------|-------------|-----------------|
| `user` | User-typed message | `null` |
| `assistant` | LLM response text | `{"model": "claude-sonnet-4-20250514"}` |
| `system` | System-generated message (welcome, error, status) | `{"type": "welcome" \| "error" \| "status"}` |
| `tool_call` | Tool invocation by the LLM | `{"tool": "execute_command", "args": {"command": "df -h"}}` |
| `tool_result` | Output from a tool execution | `{"tool": "execute_command", "exit_code": 0}` |

### 5.2 Streaming and Completeness

- When the Intelligence Engine begins streaming a response, it inserts a message with `is_complete = 0`.
- As tokens arrive, the message `content` is updated in place.
- When streaming finishes, `is_complete` is set to `1`.
- If the process crashes mid-stream, the partial message remains with `is_complete = 0`. On restart, the engine can detect this and either discard or retain the partial content.

---

## 6. Operations

### 6.1 Write Path (Intelligence Engine)

1. **User message received:** Insert row with `role = 'user'`.
2. **API call made:** If tool calls are returned, insert `tool_call` row, execute tool, insert `tool_result` row.
3. **Streaming begins:** Insert row with `role = 'assistant'`, `is_complete = 0`.
4. **Tokens arrive:** Periodically update the assistant message's `content` (batched, not per-token).
5. **Streaming ends:** Set `is_complete = 1`, update `conversations.updated_at`.

Content updates during streaming are batched (every 200ms or 50 tokens, whichever comes first) to avoid excessive write I/O.

### 6.2 Read Path (Chat Shell — History Display)

On startup, the Chat Shell loads all messages for the active conversation:

```sql
SELECT id, role, content, metadata, created_at, is_complete
FROM messages
WHERE conversation_id = ?
ORDER BY id ASC;
```

For conversations with very large history, the Chat Shell can paginate by loading the most recent N messages first and loading older messages on scroll-up.

### 6.3 Read Path (Intelligence Engine — Context Window)

The engine loads the most recent messages to fit within the LLM context budget:

```sql
SELECT id, role, content, metadata
FROM messages
WHERE conversation_id = ?
ORDER BY id DESC
LIMIT ?;
```

The result is reversed to chronological order before inclusion in the API request. The engine estimates token count from character count (approximately 4 characters per token) and adjusts the limit to stay within budget.

### 6.4 Clear History

```sql
DELETE FROM messages WHERE conversation_id = ?;
UPDATE conversations SET updated_at = strftime('%Y-%m-%dT%H:%M:%f', 'now');
VACUUM;
```

After clearing, the engine inserts a new `system` welcome message, resetting the conversation to its first-boot state.

---

## 7. Concurrency Model

- **Single writer:** The Intelligence Engine is the only process that writes to the database.
- **Multiple readers:** Both the Chat Shell and the Intelligence Engine read from the database.
- WAL mode allows concurrent reads during writes without blocking.
- The Chat Shell connects in read-only mode (`SQLITE_OPEN_READONLY`) as an additional safety measure.

---

## 8. Error Handling

| Scenario | Behavior |
|----------|----------|
| Database file missing on startup | Engine creates the database and runs schema migrations. |
| Database corrupted | Engine logs error, renames corrupt file to `history.db.corrupt`, creates fresh database. |
| Disk full | Engine catches the write error, displays "storage full" message in chat via L3. |
| WAL file grows too large | `wal_autocheckpoint` handles this. If manual intervention is needed, engine runs `PRAGMA wal_checkpoint(TRUNCATE)`. |
| Crash during streaming | On restart, messages with `is_complete = 0` are detected. Partial content is retained but marked in the UI. |

---

## 9. Diagram: Write Flow

```
User types message
       |
       v
+------------------+
| Intelligence     |  1. INSERT user message
|    Engine        |  2. Send to Claude API
|                  |  3. INSERT tool_call (if any)
|                  |  4. Execute tool
|                  |  5. INSERT tool_result
|                  |  6. INSERT assistant message (is_complete=0)
|                  |  7. UPDATE content as tokens stream
|                  |  8. SET is_complete=1
+--------+---------+
         |
         v
   +----------+
   |  SQLite  |  WAL ensures durability
   | history  |
   |   .db    |
   +----------+
         |
         v (read)
+------------------+
|   Chat Shell     |  Reads messages for display
+------------------+
```
