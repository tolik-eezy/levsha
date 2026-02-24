//! Session persistence layer (Track J).
//!
//! SQLite-backed store for session metadata. Works alongside the existing
//! History module (messages table) by adding a sessions table and a
//! session_id column to messages.

use crate::types::SessionInfo;
use rusqlite::{params, Connection};
use tracing::{debug, info};

/// SQLite-backed session store.
pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    /// Open the session store using the same database file as history.
    pub fn open(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode (may already be set by history).
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // Run migrations.
        Self::migrate(&conn)?;

        info!("Session store opened at {}", db_path);
        Ok(Self { conn })
    }

    /// Run schema migrations for session support.
    fn migrate(conn: &Connection) -> Result<(), Box<dyn std::error::Error>> {
        // Create sessions table.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                is_archived INTEGER NOT NULL DEFAULT 0
            );",
        )?;

        // Add session_id column to messages if it doesn't exist.
        // SQLite doesn't have IF NOT EXISTS for ALTER TABLE, so we check first.
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

        // Create index on session_id for messages.
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);",
        )?;

        Ok(())
    }

    /// Create a new session.
    pub fn create_session(
        &self,
        id: &str,
        name: &str,
    ) -> Result<SessionInfo, Box<dyn std::error::Error>> {
        let now = current_timestamp();

        self.conn.execute(
            "INSERT INTO sessions (id, name, created_at, updated_at, is_archived) VALUES (?1, ?2, ?3, ?4, 0)",
            params![id, name, now, now],
        )?;

        debug!("Created session: {} ({})", name, id);

        Ok(SessionInfo {
            id: id.to_string(),
            name: name.to_string(),
            updated_at: now,
            message_count: 0,
            is_archived: false,
            last_message_preview: None,
            is_active: false,
        })
    }

    /// Get a session by ID.
    pub fn get_session(
        &self,
        id: &str,
    ) -> Result<Option<SessionInfo>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.updated_at, s.is_archived,
                    (SELECT COUNT(*) FROM messages WHERE session_id = s.id) as msg_count,
                    (SELECT content FROM messages WHERE session_id = s.id AND role IN ('user', 'assistant') ORDER BY id DESC LIMIT 1) as last_msg
             FROM sessions s WHERE s.id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(SessionInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                updated_at: row.get(2)?,
                is_archived: row.get::<_, i64>(3)? != 0,
                message_count: row.get::<_, i32>(4)?,
                last_message_preview: row.get::<_, Option<String>>(5)?
                    .map(|s| truncate_preview(&s)),
                is_active: false,
            })
        });

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all non-archived sessions, most recently updated first.
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.updated_at, s.is_archived,
                    (SELECT COUNT(*) FROM messages WHERE session_id = s.id) as msg_count,
                    (SELECT content FROM messages WHERE session_id = s.id AND role IN ('user', 'assistant') ORDER BY id DESC LIMIT 1) as last_msg
             FROM sessions s
             WHERE s.is_archived = 0
             ORDER BY s.updated_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SessionInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                updated_at: row.get(2)?,
                is_archived: row.get::<_, i64>(3)? != 0,
                message_count: row.get::<_, i32>(4)?,
                last_message_preview: row.get::<_, Option<String>>(5)?
                    .map(|s| truncate_preview(&s)),
                is_active: false,
            })
        })?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    /// Rename a session.
    pub fn rename_session(
        &self,
        id: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let affected = self.conn.execute(
            "UPDATE sessions SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![name, current_timestamp(), id],
        )?;

        if affected == 0 {
            return Err(format!("Session '{}' not found", id).into());
        }

        debug!("Renamed session {} to '{}'", id, name);
        Ok(())
    }

    /// Archive a session (soft delete).
    pub fn archive_session(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let affected = self.conn.execute(
            "UPDATE sessions SET is_archived = 1, updated_at = ?1 WHERE id = ?2",
            params![current_timestamp(), id],
        )?;

        if affected == 0 {
            return Err(format!("Session '{}' not found", id).into());
        }

        debug!("Archived session {}", id);
        Ok(())
    }

    /// Delete a session and all its messages.
    pub fn delete_session(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn
            .execute("DELETE FROM messages WHERE session_id = ?1", params![id])?;
        let affected = self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id])?;

        if affected == 0 {
            return Err(format!("Session '{}' not found", id).into());
        }

        debug!("Deleted session {} and its messages", id);
        Ok(())
    }

    /// Update the activity timestamp for a session.
    pub fn update_activity(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            params![current_timestamp(), id],
        )?;
        Ok(())
    }

    /// Get a preview of the last message in a session.
    pub fn get_last_message_preview(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let result = self.conn.query_row(
            "SELECT content FROM messages WHERE session_id = ?1 AND role IN ('user', 'assistant') ORDER BY id DESC LIMIT 1",
            params![session_id],
            |row| row.get::<_, String>(0),
        );

        match result {
            Ok(content) => Ok(Some(truncate_preview(&content))),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if a default session exists, creating one if needed.
    pub fn ensure_default_session(
        &self,
        default_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Check if any session exists.
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))?;

        if count == 0 {
            let id = "default".to_string();
            self.create_session(&id, default_name)?;
            info!("Created default session");
            Ok(id)
        } else {
            // Return the most recently updated session.
            let id: String = self.conn.query_row(
                "SELECT id FROM sessions WHERE is_archived = 0 ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )?;
            Ok(id)
        }
    }
}

/// Get the current Unix timestamp.
fn current_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Truncate a string for preview display (safe for multi-byte UTF-8).
fn truncate_preview(s: &str) -> String {
    let max_chars = 100;
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => format!("{}...", &s[..idx]),
        None => s.to_string(),
    }
}
