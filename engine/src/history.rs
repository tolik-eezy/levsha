//! SQLite persistence layer with WAL mode.
//!
//! Schema: messages table with id, role, content, tool_name, tool_args,
//! exit_code, timestamp, completion_status. Auto-creates tables on first run.
//! DB path: /var/lib/levsha/history.db (configurable).

use crate::types::MessageRole;
use rusqlite::{params, Connection};
use std::path::Path;
use tracing::{debug, info};

/// A stored conversation message retrieved from the database.
#[derive(Debug, Clone)]
pub struct StoredMessage {
    pub id: i64,
    pub role: MessageRole,
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_args: Option<String>,
    pub exit_code: Option<i32>,
    pub timestamp: i64,
    pub completion_status: Option<String>,
}

/// SQLite-backed conversation history.
pub struct History {
    conn: Connection,
}

impl History {
    /// Open (or create) the database at the given path. Enables WAL mode and
    /// creates the messages table if it does not exist.
    pub fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure parent directory exists.
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Enable WAL mode for crash safety and concurrent reads.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        // Create the messages table.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                tool_name TEXT,
                tool_args TEXT,
                exit_code INTEGER,
                timestamp INTEGER NOT NULL,
                completion_status TEXT
            );",
        )?;

        // Ensure session_id column exists (Track J migration).
        // This runs early so that insert_message_for_session() can always use the column.
        let has_session_id: bool = conn
            .prepare("SELECT COUNT(*) FROM pragma_table_info('messages') WHERE name='session_id'")?
            .query_row([], |row| row.get::<_, i64>(0))
            .map(|c| c > 0)
            .unwrap_or(false);

        if !has_session_id {
            conn.execute_batch(
                "ALTER TABLE messages ADD COLUMN session_id TEXT DEFAULT 'default';",
            )?;
            info!("Added session_id column to messages table");
        }

        info!("History database opened at {}", path);
        Ok(Self { conn })
    }

    /// Insert a new message into the database.
    pub fn insert_message(
        &self,
        role: &MessageRole,
        content: &str,
        tool_name: Option<&str>,
        tool_args: Option<&str>,
        exit_code: Option<i32>,
        completion_status: Option<&str>,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        self.insert_message_for_session(
            role,
            content,
            tool_name,
            tool_args,
            exit_code,
            completion_status,
            "default",
        )
    }

    /// Insert a new message into the database for a specific session.
    pub fn insert_message_for_session(
        &self,
        role: &MessageRole,
        content: &str,
        tool_name: Option<&str>,
        tool_args: Option<&str>,
        exit_code: Option<i32>,
        completion_status: Option<&str>,
        session_id: &str,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let role_str = role_to_str(role);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO messages (role, content, tool_name, tool_args, exit_code, timestamp, completion_status, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                role_str,
                content,
                tool_name,
                tool_args,
                exit_code,
                timestamp,
                completion_status,
                session_id,
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        debug!(
            "Inserted message id={} role={} len={} session={}",
            id,
            role_str,
            content.len(),
            session_id
        );
        Ok(id)
    }

    /// Retrieve all messages in chronological order.
    pub fn get_all_messages(&self) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, tool_name, tool_args, exit_code, timestamp, completion_status
             FROM messages ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(StoredMessage {
                id: row.get(0)?,
                role: str_to_role(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                tool_name: row.get(3)?,
                tool_args: row.get(4)?,
                exit_code: row.get(5)?,
                timestamp: row.get(6)?,
                completion_status: row.get(7)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    /// Retrieve the N most recent messages in chronological order.
    pub fn get_recent_messages(
        &self,
        n: usize,
    ) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, tool_name, tool_args, exit_code, timestamp, completion_status
             FROM messages ORDER BY id DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![n as i64], |row| {
            Ok(StoredMessage {
                id: row.get(0)?,
                role: str_to_role(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                tool_name: row.get(3)?,
                tool_args: row.get(4)?,
                exit_code: row.get(5)?,
                timestamp: row.get(6)?,
                completion_status: row.get(7)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        // Reverse to get chronological order (oldest first).
        messages.reverse();
        Ok(messages)
    }

    /// Check if the history is empty (all sessions).
    pub fn is_empty(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))?;
        Ok(count == 0)
    }

    /// Check if a specific session has no messages.
    pub fn is_session_empty(&self, session_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    /// Retrieve all messages for a specific session in chronological order.
    pub fn get_messages_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, tool_name, tool_args, exit_code, timestamp, completion_status
             FROM messages WHERE session_id = ?1 ORDER BY id ASC",
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            Ok(StoredMessage {
                id: row.get(0)?,
                role: str_to_role(&row.get::<_, String>(1)?),
                content: row.get(2)?,
                tool_name: row.get(3)?,
                tool_args: row.get(4)?,
                exit_code: row.get(5)?,
                timestamp: row.get(6)?,
                completion_status: row.get(7)?,
            })
        })?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    /// Retrieve all messages for a specific session (alias for get_messages_for_session).
    pub fn get_all_messages_for_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<StoredMessage>, Box<dyn std::error::Error>> {
        self.get_messages_for_session(session_id)
    }

    /// Clear all messages for a specific session.
    pub fn clear_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )?;
        info!("Cleared messages for session {}", session_id);
        Ok(())
    }

    /// Clear all messages and vacuum the database.
    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute("DELETE FROM messages", [])?;
        self.conn.execute_batch("VACUUM")?;
        info!("History cleared and vacuumed");
        Ok(())
    }
}

/// Convert a MessageRole to its database string representation.
fn role_to_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::System => "system",
        MessageRole::ToolCall => "tool_call",
        MessageRole::ToolResult => "tool_result",
    }
}

/// Convert a database string to a MessageRole.
fn str_to_role(s: &str) -> MessageRole {
    match s {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        "tool_call" => MessageRole::ToolCall,
        "tool_result" => MessageRole::ToolResult,
        _ => MessageRole::System,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_memory_db() -> History {
        History::open(":memory:").expect("Failed to open in-memory database")
    }

    #[test]
    fn new_db_is_empty() {
        let h = open_memory_db();
        assert!(h.is_empty().unwrap());
    }

    #[test]
    fn insert_makes_db_non_empty() {
        let h = open_memory_db();
        h.insert_message(&MessageRole::User, "hello", None, None, None, None)
            .unwrap();
        assert!(!h.is_empty().unwrap());
    }

    #[test]
    fn insert_and_get_all() {
        let h = open_memory_db();
        h.insert_message(&MessageRole::User, "msg1", None, None, None, None)
            .unwrap();
        h.insert_message(&MessageRole::Assistant, "msg2", None, None, None, Some("complete"))
            .unwrap();

        let msgs = h.get_all_messages().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, MessageRole::User);
        assert_eq!(msgs[0].content, "msg1");
        assert_eq!(msgs[1].role, MessageRole::Assistant);
        assert_eq!(msgs[1].content, "msg2");
        assert_eq!(msgs[1].completion_status.as_deref(), Some("complete"));
    }

    #[test]
    fn insert_with_tool_fields() {
        let h = open_memory_db();
        h.insert_message(
            &MessageRole::ToolCall,
            "tool_use_123",
            Some("run_command"),
            Some(r#"{"command":"ls"}"#),
            None,
            None,
        )
        .unwrap();

        let msgs = h.get_all_messages().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].tool_name.as_deref(), Some("run_command"));
        assert_eq!(msgs[0].tool_args.as_deref(), Some(r#"{"command":"ls"}"#));
    }

    #[test]
    fn insert_with_exit_code() {
        let h = open_memory_db();
        h.insert_message(
            &MessageRole::ToolResult,
            "output",
            Some("tool_use_123"),
            None,
            Some(0),
            None,
        )
        .unwrap();

        let msgs = h.get_all_messages().unwrap();
        assert_eq!(msgs[0].exit_code, Some(0));
    }

    #[test]
    fn get_recent_messages_respects_limit() {
        let h = open_memory_db();
        for i in 0..5 {
            h.insert_message(
                &MessageRole::User,
                &format!("msg{}", i),
                None,
                None,
                None,
                None,
            )
            .unwrap();
        }

        let recent = h.get_recent_messages(3).unwrap();
        assert_eq!(recent.len(), 3);
        // Should be the last 3 in chronological order
        assert_eq!(recent[0].content, "msg2");
        assert_eq!(recent[1].content, "msg3");
        assert_eq!(recent[2].content, "msg4");
    }

    #[test]
    fn get_recent_messages_returns_all_when_limit_exceeds() {
        let h = open_memory_db();
        h.insert_message(&MessageRole::User, "only", None, None, None, None)
            .unwrap();

        let recent = h.get_recent_messages(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "only");
    }

    #[test]
    fn clear_removes_all_messages() {
        let h = open_memory_db();
        h.insert_message(&MessageRole::User, "hello", None, None, None, None)
            .unwrap();
        assert!(!h.is_empty().unwrap());

        h.clear().unwrap();
        assert!(h.is_empty().unwrap());
        assert_eq!(h.get_all_messages().unwrap().len(), 0);
    }

    #[test]
    fn messages_are_chronological() {
        let h = open_memory_db();
        h.insert_message(&MessageRole::User, "first", None, None, None, None)
            .unwrap();
        h.insert_message(&MessageRole::Assistant, "second", None, None, None, None)
            .unwrap();
        h.insert_message(&MessageRole::User, "third", None, None, None, None)
            .unwrap();

        let msgs = h.get_all_messages().unwrap();
        assert!(msgs[0].id < msgs[1].id);
        assert!(msgs[1].id < msgs[2].id);
    }

    #[test]
    fn role_roundtrip() {
        let roles = [
            MessageRole::User,
            MessageRole::Assistant,
            MessageRole::System,
            MessageRole::ToolCall,
            MessageRole::ToolResult,
        ];
        for role in &roles {
            let s = role_to_str(role);
            let back = str_to_role(s);
            assert_eq!(*role, back);
        }
    }

    #[test]
    fn unknown_role_string_defaults_to_system() {
        assert_eq!(str_to_role("unknown_role"), MessageRole::System);
    }

    #[test]
    fn open_with_tempfile() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let h = History::open(db_path.to_str().unwrap()).unwrap();
        h.insert_message(&MessageRole::User, "test", None, None, None, None)
            .unwrap();
        assert!(!h.is_empty().unwrap());
    }

    #[test]
    fn is_session_empty_checks_per_session() {
        let h = open_memory_db();
        assert!(h.is_session_empty("session-a").unwrap());
        assert!(h.is_session_empty("session-b").unwrap());

        h.insert_message_for_session(
            &MessageRole::User, "hello", None, None, None, None, "session-a",
        )
        .unwrap();

        assert!(!h.is_session_empty("session-a").unwrap());
        assert!(h.is_session_empty("session-b").unwrap());
    }

    #[test]
    fn get_messages_for_session_isolates_sessions() {
        let h = open_memory_db();

        h.insert_message_for_session(
            &MessageRole::User, "msg-a1", None, None, None, None, "sess-a",
        )
        .unwrap();
        h.insert_message_for_session(
            &MessageRole::Assistant, "msg-a2", None, None, None, None, "sess-a",
        )
        .unwrap();
        h.insert_message_for_session(
            &MessageRole::User, "msg-b1", None, None, None, None, "sess-b",
        )
        .unwrap();

        let a_msgs = h.get_messages_for_session("sess-a").unwrap();
        assert_eq!(a_msgs.len(), 2);
        assert_eq!(a_msgs[0].content, "msg-a1");
        assert_eq!(a_msgs[1].content, "msg-a2");

        let b_msgs = h.get_messages_for_session("sess-b").unwrap();
        assert_eq!(b_msgs.len(), 1);
        assert_eq!(b_msgs[0].content, "msg-b1");

        // get_all_messages still returns everything
        let all = h.get_all_messages().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn clear_session_only_removes_target() {
        let h = open_memory_db();

        h.insert_message_for_session(
            &MessageRole::User, "keep", None, None, None, None, "sess-a",
        )
        .unwrap();
        h.insert_message_for_session(
            &MessageRole::User, "remove", None, None, None, None, "sess-b",
        )
        .unwrap();

        h.clear_session("sess-b").unwrap();

        assert!(!h.is_session_empty("sess-a").unwrap());
        assert!(h.is_session_empty("sess-b").unwrap());
        assert_eq!(h.get_all_messages().unwrap().len(), 1);
    }
}
