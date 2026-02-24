# 19 — Multi-Session Support: Technical Plan

**Module:** Multi-Session Support (L2 + L3)
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

```
engine/
  src/
    sessions/
      mod.rs              # Session manager (CRUD, switching, auto-naming)
      store.rs            # SQLite session persistence
      state.rs            # Per-session runtime state (context, editor, project)
    context/
      history.rs          # Modified: session-scoped history queries

chat-shell/
  src/
    ui/
      session_list.rs     # Session list overlay widget
      session_bar.rs      # Status bar session name segment
    state/
      session.rs          # Modified: multi-session state tracking
```

---

## 2. Database Schema

```rust
pub fn create_sessions_table(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            name TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            message_count INTEGER DEFAULT 0,
            is_archived INTEGER DEFAULT 0,
            metadata TEXT
        );

        -- Ensure existing messages have a session_id
        -- The messages table already has session_id from Phase 1
        CREATE INDEX IF NOT EXISTS idx_messages_session
            ON messages(session_id, timestamp);

        CREATE INDEX IF NOT EXISTS idx_sessions_updated
            ON sessions(is_archived, updated_at DESC);
    ")?;
    Ok(())
}
```

---

## 3. Session Manager (Engine)

```rust
pub struct SessionManager {
    db: Arc<rusqlite::Connection>,
    active_session_id: String,
    session_states: HashMap<String, SessionState>,
    config: SessionConfig,
}

#[derive(Clone)]
pub struct Session {
    pub id: String,
    pub name: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i32,
    pub is_archived: bool,
    pub metadata: SessionMetadata,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SessionMetadata {
    pub open_file: Option<String>,          // Text/code editor open file path
    pub project_root: Option<String>,       // Code editor project directory
    pub split_view_content: Option<String>, // Serialized content panel state
}

/// Runtime state for a session (not persisted — rebuilt on switch)
pub struct SessionState {
    pub conversation_history: Vec<Message>,
    pub context_tokens: usize,
    pub edit_buffer: Option<EditBuffer>,
}

#[derive(Deserialize)]
pub struct SessionConfig {
    pub max_active: usize,     // default: 20
    pub auto_name: bool,       // default: true
    pub default_name: String,  // default: "New Session"
}

impl SessionManager {
    pub fn new(db: Arc<rusqlite::Connection>, config: SessionConfig) -> Result<Self> {
        create_sessions_table(&db)?;

        // Load or create default session
        let active = match Self::load_most_recent(&db)? {
            Some(session) => session.id,
            None => Self::create_session_internal(&db, &config.default_name)?,
        };

        Ok(Self {
            db,
            active_session_id: active,
            session_states: HashMap::new(),
            config,
        })
    }

    pub fn create_session(&mut self, name: Option<&str>) -> Result<Session> {
        let session_name = name.unwrap_or(&self.config.default_name);
        let id = Self::create_session_internal(&self.db, session_name)?;
        let session = self.load_session(&id)?;

        // Switch to the new session
        self.switch_to(&id)?;

        Ok(session)
    }

    pub fn switch_to(&mut self, session_id: &str) -> Result<Session> {
        // Save current session state
        self.save_current_state()?;

        // Load new session
        let session = self.load_session(session_id)?;

        // Load or create runtime state
        if !self.session_states.contains_key(session_id) {
            let history = self.load_history(session_id)?;
            self.session_states.insert(session_id.to_string(), SessionState {
                conversation_history: history,
                context_tokens: 0,
                edit_buffer: None,
            });
        }

        self.active_session_id = session_id.to_string();

        // Update session activity timestamp
        self.db.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().timestamp(), session_id],
        )?;

        Ok(session)
    }

    pub fn list_sessions(&self, include_archived: bool) -> Result<Vec<Session>> {
        let query = if include_archived {
            "SELECT * FROM sessions ORDER BY updated_at DESC"
        } else {
            "SELECT * FROM sessions WHERE is_archived = 0 ORDER BY updated_at DESC"
        };

        let mut stmt = self.db.prepare(query)?;
        let sessions = stmt.query_map([], |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                message_count: row.get(4)?,
                is_archived: row.get::<_, i32>(5)? != 0,
                metadata: row.get::<_, String>(6)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    pub fn rename_session(&self, session_id: &str, name: &str) -> Result<()> {
        self.db.execute(
            "UPDATE sessions SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, session_id],
        )?;
        Ok(())
    }

    pub fn archive_session(&self, session_id: &str) -> Result<()> {
        if session_id == self.active_session_id {
            return Err(SessionError::CannotArchiveActive);
        }
        self.db.execute(
            "UPDATE sessions SET is_archived = 1 WHERE id = ?1",
            rusqlite::params![session_id],
        )?;
        // Clean up runtime state
        self.session_states.remove(session_id);
        Ok(())
    }

    pub fn restore_session(&self, session_id: &str) -> Result<()> {
        self.db.execute(
            "UPDATE sessions SET is_archived = 0 WHERE id = ?1",
            rusqlite::params![session_id],
        )?;
        Ok(())
    }

    pub fn delete_session(&mut self, session_id: &str) -> Result<()> {
        if session_id == self.active_session_id {
            return Err(SessionError::CannotDeleteActive);
        }
        // Delete messages first
        self.db.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            rusqlite::params![session_id],
        )?;
        self.db.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_id],
        )?;
        self.session_states.remove(session_id);
        Ok(())
    }

    pub fn active_session(&self) -> Result<&str> {
        Ok(&self.active_session_id)
    }

    pub fn active_state(&self) -> Result<&SessionState> {
        self.session_states.get(&self.active_session_id)
            .ok_or(SessionError::NoActiveSession)
    }

    pub fn active_state_mut(&mut self) -> Result<&mut SessionState> {
        let id = self.active_session_id.clone();
        self.session_states.get_mut(&id)
            .ok_or(SessionError::NoActiveSession)
    }

    fn create_session_internal(db: &rusqlite::Connection, name: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        db.execute(
            "INSERT INTO sessions (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id, name, now, now],
        )?;
        Ok(id)
    }

    fn save_current_state(&self) -> Result<()> {
        if let Some(state) = self.session_states.get(&self.active_session_id) {
            let metadata = SessionMetadata {
                open_file: state.edit_buffer.as_ref().map(|b| b.file_path.to_string_lossy().to_string()),
                project_root: None, // populated by code editor
                split_view_content: None, // serialized panel state
            };
            self.db.execute(
                "UPDATE sessions SET metadata = ?1 WHERE id = ?2",
                rusqlite::params![serde_json::to_string(&metadata)?, self.active_session_id],
            )?;
        }
        Ok(())
    }

    fn load_history(&self, session_id: &str) -> Result<Vec<Message>> {
        let mut stmt = self.db.prepare(
            "SELECT role, content, metadata FROM messages WHERE session_id = ?1 ORDER BY timestamp"
        )?;
        let messages = stmt.query_map(rusqlite::params![session_id], |row| {
            Ok(Message {
                role: row.get(0)?,
                content: vec![ContentBlock::Text { text: row.get(1)? }],
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    fn load_most_recent(db: &rusqlite::Connection) -> Result<Option<Session>> {
        let mut stmt = db.prepare(
            "SELECT * FROM sessions WHERE is_archived = 0 ORDER BY updated_at DESC LIMIT 1"
        )?;
        let session = stmt.query_row([], |row| {
            Ok(Session {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                message_count: row.get(4)?,
                is_archived: false,
                metadata: SessionMetadata::default(),
            })
        }).optional()?;
        Ok(session)
    }
}
```

---

## 4. Auto-Naming

```rust
impl SessionManager {
    pub async fn auto_name_session(
        &self,
        session_id: &str,
        first_messages: &[Message],
        backend: &dyn LlmBackend,
    ) -> Result<String> {
        // Only auto-name if still default name
        let session = self.load_session(session_id)?;
        if session.name.as_deref() != Some(&self.config.default_name) {
            return Ok(session.name.unwrap_or_default());
        }

        // Build a minimal request for naming
        let context: String = first_messages.iter()
            .take(3)
            .flat_map(|m| m.content.iter())
            .filter_map(|c| match c {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        let request = MessageRequest {
            model: "claude-haiku-4-5-20251001".into(), // cheapest model
            max_tokens: 10,
            system: "Generate a 2-4 word title for this conversation. Reply with ONLY the title.".into(),
            messages: vec![Message {
                role: "user".into(),
                content: vec![ContentBlock::Text {
                    text: format!("Title this conversation:\n{}", &context[..context.len().min(500)]),
                }],
            }],
            tools: vec![],
            stream: false,
        };

        let name = backend.send_message(&request).await?
            .collect_text().await?
            .trim()
            .to_string();

        if !name.is_empty() && name.len() < 50 {
            self.rename_session(session_id, &name)?;
            Ok(name)
        } else {
            Ok(self.config.default_name.clone())
        }
    }
}
```

---

## 5. Context Manager Integration

The existing context manager is updated to be session-aware.

```rust
impl ContextManager {
    /// Load conversation history for the active session
    pub fn load_session_context(&mut self, session_id: &str) -> Result<()> {
        self.history = self.session_mgr.load_history(session_id)?;
        self.recalculate_tokens();
        Ok(())
    }

    /// Append a message to the active session
    pub fn append_message(&mut self, msg: Message) -> Result<()> {
        let session_id = self.session_mgr.active_session()?;
        self.db.execute(
            "INSERT INTO messages (role, content, timestamp, session_id) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                msg.role,
                msg.text_content(),
                chrono::Utc::now().timestamp(),
                session_id,
            ],
        )?;
        self.history.push(msg);

        // Update session message count
        self.db.execute(
            "UPDATE sessions SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![chrono::Utc::now().timestamp(), session_id],
        )?;

        Ok(())
    }
}
```

---

## 6. IPC Protocol Extensions

```rust
// Chat Shell -> Engine
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionMessage {
    #[serde(rename = "session_create")]
    Create { name: Option<String> },
    #[serde(rename = "session_switch")]
    Switch { session_id: String },
    #[serde(rename = "session_list")]
    List { include_archived: bool },
    #[serde(rename = "session_rename")]
    Rename { session_id: String, name: String },
    #[serde(rename = "session_archive")]
    Archive { session_id: String },
    #[serde(rename = "session_restore")]
    Restore { session_id: String },
    #[serde(rename = "session_delete")]
    Delete { session_id: String },
}

// Engine -> Chat Shell
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionResponse {
    #[serde(rename = "session_switched")]
    Switched {
        session: SessionInfo,
        history: Vec<Message>,   // Full conversation for the session
        has_content_panel: bool,  // Whether to open split-view
    },
    #[serde(rename = "session_list")]
    List { sessions: Vec<SessionInfo> },
    #[serde(rename = "session_created")]
    Created { session: SessionInfo },
    #[serde(rename = "session_renamed")]
    Renamed { session_id: String, name: String },
    #[serde(rename = "session_deleted")]
    Deleted { session_id: String },
}

#[derive(Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub updated_at: i64,
    pub message_count: i32,
    pub is_archived: bool,
    pub last_message_preview: Option<String>,
    pub is_active: bool,
}
```

---

## 7. Session List Overlay (Chat Shell)

```rust
pub struct SessionListOverlay {
    container: gtk4::Box,
    revealer: gtk4::Revealer,
    list_box: gtk4::ListBox,
    new_button: gtk4::Button,
    backdrop: gtk4::Box,
}

impl SessionListOverlay {
    pub fn new() -> Self {
        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::Crossfade);
        revealer.set_transition_duration(200);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.add_css_class("session-overlay");
        container.set_halign(gtk4::Align::Center);
        container.set_valign(gtk4::Align::Center);
        container.set_width_request(480);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let title = gtk4::Label::new(Some("Sessions"));
        title.add_css_class("session-overlay-title");
        let new_button = gtk4::Button::with_label("+ New");
        new_button.add_css_class("session-new-button");
        header.append(&title);
        header.append(&new_button);

        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("session-list");

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_child(Some(&list_box));
        scrolled.set_max_content_height(400);

        container.append(&header);
        container.append(&scrolled);
        revealer.set_child(Some(&container));

        let backdrop = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        backdrop.add_css_class("session-backdrop");

        Self {
            container,
            revealer,
            list_box,
            new_button,
            backdrop,
        }
    }

    pub fn show(&self) {
        self.backdrop.set_visible(true);
        self.revealer.set_reveal_child(true);
    }

    pub fn hide(&self) {
        self.revealer.set_reveal_child(false);
        self.backdrop.set_visible(false);
    }

    pub fn update_sessions(&self, sessions: &[SessionInfo]) {
        // Clear existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        for session in sessions {
            let row = self.create_session_row(session);
            self.list_box.append(&row);
        }
    }

    fn create_session_row(&self, session: &SessionInfo) -> gtk4::ListBoxRow {
        let row_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        row_box.set_margin_start(16);
        row_box.set_margin_end(16);
        row_box.set_margin_top(8);
        row_box.set_margin_bottom(8);

        let top = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let indicator = gtk4::Label::new(Some(if session.is_active { "●" } else { "○" }));
        indicator.add_css_class(if session.is_active { "session-active" } else { "session-inactive" });
        let name = gtk4::Label::new(Some(&session.name));
        name.add_css_class("session-name");
        let time = gtk4::Label::new(Some(&format_relative_time(session.updated_at)));
        time.add_css_class("session-time");
        time.set_hexpand(true);
        time.set_halign(gtk4::Align::End);
        top.append(&indicator);
        top.append(&name);
        top.append(&time);

        row_box.append(&top);

        if let Some(preview) = &session.last_message_preview {
            let preview_label = gtk4::Label::new(Some(preview));
            preview_label.add_css_class("session-preview");
            preview_label.set_ellipsize(pango::EllipsizeMode::End);
            preview_label.set_max_width_chars(50);
            row_box.append(&preview_label);
        }

        let row = gtk4::ListBoxRow::new();
        row.set_child(Some(&row_box));
        row
    }
}
```

---

## 8. Implementation Stages

**Stage 1 — Database & Session Manager (1 day)**
1. Create sessions table with migration.
2. Implement `SessionManager` (create, switch, list, rename, archive, delete).
3. Link messages to sessions.
4. Unit tests for all CRUD operations.

**Stage 2 — Context Integration (0.5 day)**
1. Update `ContextManager` to be session-aware.
2. Load/save conversation history per session.
3. Session-scoped message append.

**Stage 3 — IPC Protocol (0.5 day)**
1. Add session message types to IPC protocol.
2. Engine-side session message handlers.
3. Wire session commands to the session manager.

**Stage 4 — Session List Overlay (1-2 days)**
1. `SessionListOverlay` widget with session rows.
2. Backdrop dimming effect.
3. New session button.
4. Click-to-switch behavior.
5. Keyboard shortcuts (Ctrl+N, Ctrl+Tab, Ctrl+Shift+S).

**Stage 5 — Status Bar Integration (0.5 day)**
1. Session name in status bar.
2. Click to open session list.
3. Session switch animation.

**Stage 6 — Auto-Naming (0.5 day)**
1. Trigger auto-naming after first assistant response.
2. Lightweight LLM request for title generation.
3. Name update in database and UI.

**Stage 7 — Session-Scoped State (1 day)**
1. Save/restore editor state per session.
2. Save/restore split-view panel state per session.
3. Save/restore code editor project context per session.

---

## 9. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `store.rs` | Session CRUD, listing, filtering archived |
| `state.rs` | State save/restore, metadata serialization |
| `sessions/mod.rs` | Switch logic, active session tracking |

### Integration Tests

| Test | Method |
|------|--------|
| Create session | Create, verify in database, verify active |
| Switch session | Create two sessions, switch, verify history changes |
| List sessions | Create multiple, list, verify ordering by updated_at |
| Archive/restore | Archive session, verify hidden from list, restore, verify visible |
| Delete session | Delete, verify messages removed, verify from list |
| Session persistence | Create session, restart engine, verify session survives |
| History isolation | Send messages in session A, switch to B, verify B has no A messages |
| Auto-naming | Create session, send message, verify auto-generated name |
| Keyboard shortcuts | Ctrl+N creates, Ctrl+Tab switches, Ctrl+Shift+S opens list |
| Session-scoped editor | Open file in session A, switch to B, verify file not open |

### Cross-Module Tests

| Test | Description |
|------|-------------|
| Session + context window | Switch session, verify API call uses correct history |
| Session + split-view | Open file in session A, switch to B (panel closes), switch back (panel restores) |
| Session + text editor | Edit file in session A, switch to B, switch back, verify edits preserved |
