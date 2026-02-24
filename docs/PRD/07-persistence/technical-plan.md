# 07 — Persistence: Technical Plan

**Module:** Conversation Persistence (SQLite)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document details the implementation plan for the Levsha OS persistence layer — a SQLite-backed conversation store that provides crash-safe, WAL-enabled storage for all chat messages. The implementation lives in the `engine/` crate and exposes a Rust API consumed by both the Intelligence Engine (L2) and the Chat Shell (L3).

---

## 2. Library Choice

**`rusqlite`** (with `bundled` feature) is the chosen SQLite binding for Rust.

| Criterion | rusqlite | sqlx |
|-----------|----------|------|
| SQLite support | Native, mature | Supported but async-focused |
| Bundled SQLite | Yes (`bundled` feature) | Yes |
| Async required | No (synchronous API) | Yes |
| Compile time | Fast | Slow (proc macros, compile-time checks) |
| Complexity | Minimal | Higher (connection pool, runtime) |

Rationale: The persistence layer is not I/O-bound on database operations. Synchronous `rusqlite` is simpler, compiles faster, and avoids pulling in a full async runtime just for SQLite access.

**Cargo dependency:**

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

---

## 3. Module Structure

```
engine/
  src/
    persistence/
      mod.rs            # Public API: HistoryStore
      schema.rs         # Schema creation and migrations
      models.rs         # Rust types: Message, Conversation, Role
      queries.rs        # SQL query functions
    lib.rs              # Re-exports persistence module
```

---

## 4. Schema Migrations

Migrations are embedded in the binary and run at startup.

```rust
// schema.rs
const MIGRATIONS: &[(i32, &str)] = &[
    (1, "
        CREATE TABLE conversations (
            id          INTEGER PRIMARY KEY,
            created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
            updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
            title       TEXT,
            is_active   INTEGER NOT NULL DEFAULT 1
        );
        CREATE TABLE messages (
            id              INTEGER PRIMARY KEY,
            conversation_id INTEGER NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
            role            TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool_call', 'tool_result')),
            content         TEXT NOT NULL,
            metadata        TEXT,
            created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now')),
            is_complete     INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
        CREATE INDEX idx_messages_created_at ON messages(created_at);
        CREATE TABLE schema_version (
            version     INTEGER PRIMARY KEY,
            applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f', 'now'))
        );
    "),
];
```

On startup, `HistoryStore::open()` checks the highest applied version and runs any unapplied migrations in a transaction.

---

## 5. Rust Data Model

```rust
// models.rs
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
    ToolCall,
    ToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: Role,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    pub is_complete: bool,
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub title: Option<String>,
    pub is_active: bool,
}
```

---

## 6. Public API

```rust
// mod.rs
pub struct HistoryStore { /* rusqlite::Connection */ }

impl HistoryStore {
    /// Open or create the database at the given path. Runs migrations.
    pub fn open(path: &Path) -> Result<Self>;

    /// Get or create the active conversation.
    pub fn active_conversation(&self) -> Result<Conversation>;

    /// Insert a new message. Returns the message ID.
    pub fn insert_message(&self, conversation_id: i64, role: Role, content: &str, metadata: Option<&serde_json::Value>) -> Result<i64>;

    /// Update message content (used during streaming).
    pub fn update_message_content(&self, message_id: i64, content: &str) -> Result<()>;

    /// Mark a message as complete (streaming finished).
    pub fn complete_message(&self, message_id: i64) -> Result<()>;

    /// Load all messages for a conversation (Chat Shell scrollback).
    pub fn load_all_messages(&self, conversation_id: i64) -> Result<Vec<Message>>;

    /// Load the last N messages (Intelligence Engine context).
    pub fn load_recent_messages(&self, conversation_id: i64, limit: usize) -> Result<Vec<Message>>;

    /// Delete all messages and reset conversation (clear history).
    pub fn clear_history(&self, conversation_id: i64) -> Result<()>;
}
```

---

## 7. Integration with Intelligence Engine (L2)

The Intelligence Engine owns the `HistoryStore` instance and is the sole writer.

**Message lifecycle in the engine:**

1. User input received from Chat Shell via IPC.
2. Engine calls `insert_message(role=User, content=input)`.
3. Engine constructs the API request: system prompt + skill prompts + `load_recent_messages(limit=N)`.
4. Engine sends request to Claude API.
5. If the response includes tool calls:
   a. `insert_message(role=ToolCall, content=description, metadata=tool_args)`.
   b. Execute the tool.
   c. `insert_message(role=ToolResult, content=output, metadata=exit_code)`.
6. Engine inserts the assistant message: `insert_message(role=Assistant, content="", is_complete=false)`.
7. As tokens stream in, engine batches updates: `update_message_content(id, accumulated_content)` every 200ms.
8. On stream end: `complete_message(id)`.

**Context window construction:**

```rust
let recent = store.load_recent_messages(conv_id, 100)?;
let mut token_budget = MAX_CONTEXT_TOKENS - system_prompt_tokens - skill_prompt_tokens;
let mut context_messages = Vec::new();
for msg in recent.iter().rev() {
    let estimated_tokens = msg.content.len() / 4;
    if token_budget < estimated_tokens { break; }
    token_budget -= estimated_tokens;
    context_messages.push(msg);
}
context_messages.reverse();
```

---

## 8. Integration with Chat Shell (L3)

The Chat Shell connects to the database in **read-only mode** for displaying conversation history on startup.

**Startup flow:**

1. Chat Shell opens a read-only connection to `history.db`.
2. Calls `load_all_messages()` for the active conversation.
3. Renders all messages in the scrollable chat view.
4. For new messages during the session, the Chat Shell receives them via IPC from the engine (not by polling the database).

The database read on startup is a one-time operation. During the session, the Chat Shell receives new messages through the IPC channel with the engine, not through database polling.

---

## 9. Backup and Recovery

### Automatic (MVP)

- WAL mode provides automatic crash recovery. SQLite replays uncommitted WAL entries on next open.
- No additional backup mechanism in MVP. The database is a single file that can be manually copied.

### Corruption Recovery

```rust
impl HistoryStore {
    fn recover_from_corruption(path: &Path) -> Result<Self> {
        let corrupt_path = path.with_extension("db.corrupt");
        std::fs::rename(path, &corrupt_path)?;
        // Log warning about corruption and backup
        Self::open(path) // Creates fresh database
    }
}
```

On startup, if `rusqlite::Connection::open()` fails with a corruption error, the engine renames the corrupt database and creates a fresh one. The user sees a system message explaining that history was lost due to corruption.

---

## 10. Testing

### Unit Tests

| Test | Description |
|------|-------------|
| `test_create_database` | `HistoryStore::open()` creates a new database with correct schema. |
| `test_migration_idempotent` | Running `open()` twice on the same database does not fail or duplicate tables. |
| `test_insert_and_load` | Insert messages of all roles, load them back, verify content and order. |
| `test_streaming_update` | Insert incomplete message, update content multiple times, mark complete. |
| `test_clear_history` | Insert messages, clear, verify empty. Verify new welcome message can be inserted. |
| `test_recent_messages_limit` | Insert 200 messages, load recent 50, verify correct subset and order. |
| `test_wal_mode_enabled` | After open, verify `PRAGMA journal_mode` returns `wal`. |
| `test_concurrent_read_write` | Write from one thread, read from another, no blocking or errors. |

### Integration Tests

| Test | Description |
|------|-------------|
| `test_reboot_persistence` | Write messages, close database, reopen, verify messages survive. |
| `test_crash_during_stream` | Insert incomplete message, do not call `complete_message`, reopen database, verify partial message exists with `is_complete = 0`. |
| `test_corruption_recovery` | Truncate database file to simulate corruption, open store, verify fresh database created and corrupt file backed up. |
| `test_context_window_construction` | Insert conversation with tool calls and results, load recent messages, verify correct format for API request. |

### Performance Tests

| Test | Description |
|------|-------------|
| `test_bulk_insert_performance` | Insert 10,000 messages, verify total time < 10 seconds. |
| `test_load_all_performance` | Load 10,000 messages, verify total time < 100ms. |
| `test_load_recent_performance` | Load last 100 of 10,000 messages, verify total time < 20ms. |

### Cross-Module Integration Tests

These tests verify the persistence layer is correctly wired between Engine and Chat Shell. See `00-system-architecture/integration-checks.md` for full details.

| Check | Seam | What it verifies |
|-------|------|------------------|
| IC-40 | L2 -> SQLite -> L3 | Engine writes message to SQLite, Chat Shell reads it on restart |
| IC-41 | L2 -> SQLite | Partial streaming message survives Engine crash (is_complete=false) |
| IC-42 | SQLite -> reboot | Full conversation history survives VM reboot |
| IC-43 | SQLite -> L2 -> API | Engine loads recent history from SQLite, builds valid context window within token limits |
| IC-44 | L3 -> L2 -> SQLite | Clear history propagates from Chat Shell through Engine to SQLite |

---

## 11. Implementation Order

1. **Schema and migrations** — `schema.rs` with embedded SQL, migration runner.
2. **Data model** — `models.rs` with Rust types and `From` impls for rusqlite rows.
3. **Core CRUD** — `queries.rs` with insert, update, load, clear operations.
4. **HistoryStore API** — `mod.rs` wrapping everything with a clean public interface.
5. **Unit tests** — All unit tests passing.
6. **Engine integration** — Wire `HistoryStore` into the Intelligence Engine message loop.
7. **Chat Shell integration** — Read-only connection for startup history load.
8. **Integration tests** — Cross-component tests with engine and shell.
9. **Performance validation** — Benchmark tests confirming targets are met.
