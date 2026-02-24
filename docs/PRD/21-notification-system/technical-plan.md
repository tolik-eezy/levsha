# 21 — Notification System: Technical Plan

**Module:** Notification System (L2 + L3)
**Language:** Rust
**Phase:** 3

---

## 1. Crate Structure

```
engine/
  src/
    notifications/
      mod.rs              # NotificationManager (bus, persistence, DND)
      bus.rs              # Notification bus (tokio::broadcast)
      types.rs            # Notification, NotificationType, Severity enums
      store.rs            # SQLite notification persistence
      sound.rs            # Sound playback via PipeWire/rodio

chat-shell/
  src/
    ui/
      notifications/
        mod.rs            # Notification UI coordinator
        toast.rs          # Toast renderer (GtkRevealer stack)
        toast_widget.rs   # Individual toast widget
        history_panel.rs  # Notification history (split-view content)
        badge.rs          # Badge counter integration
        dnd.rs            # DND indicator in status bar
    ipc/
      notification.rs     # Notification IPC message handling
```

### New Dependencies

```toml
# Added to engine/Cargo.toml
rodio = "0.19"         # Audio playback (uses PipeWire via CPAL backend)

# No new deps for chat-shell — uses existing GTK4
```

---

## 2. Notification Types and Severity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub notification_type: NotificationType,
    pub severity: Severity,
    pub title: String,
    pub body: String,
    pub timestamp: i64,
    pub session_id: Option<String>,
    pub is_read: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    TaskComplete,
    TaskError,
    AttentionNeeded,
    SkillInstalled,
    SystemAlert,
    SessionMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Notification {
    pub fn new(
        notification_type: NotificationType,
        severity: Severity,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            notification_type,
            severity,
            title: title.into(),
            body: body.into(),
            timestamp: chrono::Utc::now().timestamp(),
            session_id: None,
            is_read: false,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}
```

---

## 3. Notification Bus

```rust
use tokio::sync::broadcast;

pub struct NotificationBus {
    sender: broadcast::Sender<Notification>,
}

impl NotificationBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Emit a notification to all subscribers.
    pub fn emit(&self, notification: Notification) {
        // Ignore send errors — no subscribers is acceptable
        let _ = self.sender.send(notification);
    }

    /// Subscribe to notifications.
    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.sender.subscribe()
    }
}
```

---

## 4. SQLite Notifications Table

```rust
pub fn create_notifications_table(conn: &rusqlite::Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS notifications (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            severity TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            session_id TEXT,
            is_read INTEGER DEFAULT 0,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_notifications_timestamp
            ON notifications(timestamp DESC);

        CREATE INDEX IF NOT EXISTS idx_notifications_session
            ON notifications(session_id, is_read);

        CREATE INDEX IF NOT EXISTS idx_notifications_type
            ON notifications(type, severity);
    ")?;
    Ok(())
}
```

---

## 5. Notification Store

```rust
pub struct NotificationStore {
    db: Arc<rusqlite::Connection>,
    config: NotificationConfig,
}

impl NotificationStore {
    pub fn new(db: Arc<rusqlite::Connection>, config: NotificationConfig) -> Result<Self> {
        create_notifications_table(&db)?;
        Ok(Self { db, config })
    }

    pub fn insert(&self, notification: &Notification) -> Result<()> {
        self.db.execute(
            "INSERT INTO notifications (id, type, severity, title, body, timestamp, session_id, is_read)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                notification.id,
                serde_json::to_string(&notification.notification_type)?,
                serde_json::to_string(&notification.severity)?,
                notification.title,
                notification.body,
                notification.timestamp,
                notification.session_id,
                notification.is_read as i32,
            ],
        )?;
        Ok(())
    }

    pub fn list(
        &self,
        filter_type: Option<NotificationType>,
        filter_severity: Option<Severity>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Notification>> {
        let mut query = "SELECT * FROM notifications WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ntype) = filter_type {
            query.push_str(" AND type = ?");
            params.push(Box::new(serde_json::to_string(&ntype)?));
        }
        if let Some(sev) = filter_severity {
            query.push_str(" AND severity = ?");
            params.push(Box::new(serde_json::to_string(&sev)?));
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ? OFFSET ?");
        params.push(Box::new(limit as i64));
        params.push(Box::new(offset as i64));

        let mut stmt = self.db.prepare(&query)?;
        let notifications = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Self::row_to_notification(row),
        )?.collect::<Result<Vec<_>, _>>()?;

        Ok(notifications)
    }

    pub fn mark_read_for_session(&self, session_id: &str) -> Result<usize> {
        let changed = self.db.execute(
            "UPDATE notifications SET is_read = 1 WHERE session_id = ?1 AND is_read = 0",
            rusqlite::params![session_id],
        )?;
        Ok(changed)
    }

    pub fn mark_all_read(&self) -> Result<usize> {
        let changed = self.db.execute(
            "UPDATE notifications SET is_read = 1 WHERE is_read = 0",
            [],
        )?;
        Ok(changed)
    }

    pub fn unread_count_for_session(&self, session_id: &str) -> Result<usize> {
        let count: i64 = self.db.query_row(
            "SELECT COUNT(*) FROM notifications WHERE session_id = ?1 AND is_read = 0",
            rusqlite::params![session_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    pub fn purge_old(&self) -> Result<usize> {
        let cutoff = chrono::Utc::now().timestamp()
            - (self.config.retention_days as i64 * 86400);
        let deleted = self.db.execute(
            "DELETE FROM notifications WHERE timestamp < ?1",
            rusqlite::params![cutoff],
        )?;
        Ok(deleted)
    }

    fn row_to_notification(row: &rusqlite::Row) -> rusqlite::Result<Notification> {
        Ok(Notification {
            id: row.get(0)?,
            notification_type: serde_json::from_str(&row.get::<_, String>(1)?)
                .unwrap_or(NotificationType::SystemAlert),
            severity: serde_json::from_str(&row.get::<_, String>(2)?)
                .unwrap_or(Severity::Info),
            title: row.get(3)?,
            body: row.get(4)?,
            timestamp: row.get(5)?,
            session_id: row.get(6)?,
            is_read: row.get::<_, i32>(7)? != 0,
        })
    }
}
```

---

## 6. Notification Manager

The central coordinator that connects the bus, store, DND, and sound.

```rust
pub struct NotificationManager {
    bus: Arc<NotificationBus>,
    store: NotificationStore,
    sound_player: Option<SoundPlayer>,
    dnd_enabled: AtomicBool,
    config: NotificationConfig,
}

impl NotificationManager {
    pub fn new(
        db: Arc<rusqlite::Connection>,
        config: NotificationConfig,
    ) -> Result<Self> {
        let bus = Arc::new(NotificationBus::new(64));
        let store = NotificationStore::new(db, config.clone())?;
        let sound_player = if config.sound.enabled {
            SoundPlayer::new(&config.sound).ok()
        } else {
            None
        };

        Ok(Self {
            bus,
            store,
            sound_player,
            dnd_enabled: AtomicBool::new(config.dnd.enabled),
            config,
        })
    }

    /// Emit a notification: persist, broadcast, play sound.
    pub fn notify(&self, notification: Notification) -> Result<()> {
        // Always persist
        self.store.insert(&notification)?;

        // Always broadcast (consumers decide whether to render)
        self.bus.emit(notification.clone());

        // Play sound unless DND
        if !self.dnd_enabled.load(Ordering::Relaxed) {
            if let Some(player) = &self.sound_player {
                player.play(notification.severity);
            }
        }

        Ok(())
    }

    pub fn set_dnd(&self, enabled: bool) {
        self.dnd_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn is_dnd(&self) -> bool {
        self.dnd_enabled.load(Ordering::Relaxed)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.bus.subscribe()
    }

    pub fn store(&self) -> &NotificationStore {
        &self.store
    }
}
```

---

## 7. Sound Player

```rust
use rodio::{OutputStream, OutputStreamHandle, Decoder, Sink};
use std::fs::File;
use std::io::BufReader;

pub struct SoundPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sound_dir: PathBuf,
    volume: f32,
}

impl SoundPlayer {
    pub fn new(config: &SoundConfig) -> Result<Self> {
        let (stream, handle) = OutputStream::try_default()
            .map_err(|e| NotificationError::AudioInit(e.to_string()))?;

        Ok(Self {
            _stream: stream,
            handle,
            sound_dir: PathBuf::from("/usr/share/levsha/sounds"),
            volume: config.volume,
        })
    }

    pub fn play(&self, severity: Severity) {
        let filename = match severity {
            Severity::Info => "info.wav",
            Severity::Warning => "warning.wav",
            Severity::Error => "error.wav",
        };

        let path = self.sound_dir.join(filename);
        if let Ok(file) = File::open(&path) {
            if let Ok(source) = Decoder::new(BufReader::new(file)) {
                if let Ok(sink) = Sink::try_new(&self.handle) {
                    sink.set_volume(self.volume);
                    sink.append(source);
                    sink.detach(); // Play in background, don't block
                }
            }
        }
    }
}
```

---

## 8. Toast Renderer (Chat Shell)

```rust
pub struct ToastRenderer {
    overlay: gtk4::Overlay,
    toast_container: gtk4::Box,
    active_toasts: Vec<ToastWidget>,
    max_visible: usize,
}

pub struct ToastWidget {
    revealer: gtk4::Revealer,
    container: gtk4::Box,
    notification_id: String,
    session_id: Option<String>,
    dismiss_source_id: Option<glib::SourceId>,
}

impl ToastRenderer {
    pub fn new(max_visible: usize) -> Self {
        let overlay = gtk4::Overlay::new();

        let toast_container = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        toast_container.add_css_class("toast-container");
        toast_container.set_halign(gtk4::Align::End);
        toast_container.set_valign(gtk4::Align::Start);
        toast_container.set_margin_top(16);
        toast_container.set_margin_end(16);

        overlay.add_overlay(&toast_container);

        Self {
            overlay,
            toast_container,
            active_toasts: Vec::new(),
            max_visible,
        }
    }

    pub fn show_toast(
        &mut self,
        notification: &Notification,
        config: &ToastConfig,
        dnd: bool,
    ) {
        if dnd {
            return; // DND suppresses toasts
        }

        // Evict oldest if at max
        if self.active_toasts.len() >= self.max_visible {
            self.dismiss_oldest();
        }

        let duration = match notification.severity {
            Severity::Info => config.duration_info,
            Severity::Warning => config.duration_warning,
            Severity::Error => config.duration_error,
        };

        let widget = ToastWidget::new(notification);

        // Slide in animation
        widget.revealer.set_reveal_child(true);

        // Auto-dismiss timer (0 = manual only)
        if duration > 0 {
            let revealer = widget.revealer.clone();
            let source_id = glib::timeout_add_seconds_local(duration as u32, move || {
                revealer.set_reveal_child(false);
                glib::ControlFlow::Break
            });
            // Store source_id for pause-on-hover
        }

        self.toast_container.prepend(&widget.revealer);
        self.active_toasts.push(widget);
    }

    fn dismiss_oldest(&mut self) {
        if let Some(toast) = self.active_toasts.first() {
            toast.revealer.set_reveal_child(false);
        }
        if !self.active_toasts.is_empty() {
            // Remove after animation completes (200ms)
            let container = self.toast_container.clone();
            glib::timeout_add_local(Duration::from_millis(200), move || {
                if let Some(child) = container.last_child() {
                    container.remove(&child);
                }
                glib::ControlFlow::Break
            });
            self.active_toasts.remove(0);
        }
    }
}

impl ToastWidget {
    pub fn new(notification: &Notification) -> Self {
        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::SlideLeft);
        revealer.set_transition_duration(250);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        container.add_css_class("toast");
        container.add_css_class(&format!("toast-{}", severity_class(notification.severity)));
        container.set_width_request(300);

        // Severity stripe (left edge)
        let stripe_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        let stripe = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        stripe.add_css_class("toast-stripe");
        stripe.set_width_request(3);

        // Content
        let content = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);

        // Title row
        let title_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let icon = gtk4::Label::new(Some(severity_icon(notification.severity)));
        icon.add_css_class("toast-icon");
        let title = gtk4::Label::new(Some(&notification.title));
        title.add_css_class("toast-title");
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);
        let dismiss = gtk4::Button::new();
        dismiss.add_css_class("toast-dismiss");
        dismiss.set_icon_name("window-close-symbolic");
        title_row.append(&icon);
        title_row.append(&title);
        title_row.append(&dismiss);

        // Body
        let body = gtk4::Label::new(Some(&notification.body));
        body.add_css_class("toast-body");
        body.set_halign(gtk4::Align::Start);
        body.set_ellipsize(pango::EllipsizeMode::End);
        body.set_lines(2);

        content.append(&title_row);
        content.append(&body);

        stripe_box.append(&stripe);
        stripe_box.append(&content);
        container.append(&stripe_box);
        revealer.set_child(Some(&container));

        Self {
            revealer,
            container,
            notification_id: notification.id.clone(),
            session_id: notification.session_id.clone(),
            dismiss_source_id: None,
        }
    }
}

fn severity_class(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn severity_icon(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "ℹ",
        Severity::Warning => "⚠",
        Severity::Error => "✗",
    }
}
```

---

## 9. Badge Counter Integration

The badge counter hooks into the notification bus and updates sidebar badges.

```rust
pub struct BadgeCounter {
    unread_counts: HashMap<String, usize>,
    store: Arc<NotificationStore>,
}

impl BadgeCounter {
    pub fn new(store: Arc<NotificationStore>) -> Self {
        Self {
            unread_counts: HashMap::new(),
            store,
        }
    }

    /// Called when a notification arrives for a session.
    pub fn increment(&mut self, session_id: &str) -> usize {
        let count = self.unread_counts.entry(session_id.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Called when the user switches to a session.
    pub fn clear(&mut self, session_id: &str) -> Result<()> {
        self.unread_counts.remove(session_id);
        self.store.mark_read_for_session(session_id)?;
        Ok(())
    }

    /// Get the unread count for a session.
    pub fn count(&self, session_id: &str) -> usize {
        self.unread_counts.get(session_id).copied().unwrap_or(0)
    }

    /// Load initial counts from the database (on startup).
    pub fn load_from_db(&mut self, session_ids: &[String]) -> Result<()> {
        for id in session_ids {
            let count = self.store.unread_count_for_session(id)?;
            if count > 0 {
                self.unread_counts.insert(id.clone(), count);
            }
        }
        Ok(())
    }
}
```

---

## 10. Notification History Panel Widget

```rust
pub struct NotificationHistoryPanel {
    container: gtk4::Box,
    header: gtk4::Box,
    filter_dropdown: gtk4::DropDown,
    list_box: gtk4::ListBox,
    scrolled: gtk4::ScrolledWindow,
    mark_all_button: gtk4::Button,
}

impl NotificationHistoryPanel {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.add_css_class("notification-history");

        // Header
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header.add_css_class("notification-history-header");

        let title = gtk4::Label::new(Some("Notifications"));
        title.add_css_class("panel-title");

        let filter_model = gtk4::StringList::new(&["All", "Tasks", "System", "Skills"]);
        let filter_dropdown = gtk4::DropDown::new(Some(filter_model), None::<gtk4::Expression>);
        filter_dropdown.add_css_class("notification-filter");

        let mark_all_button = gtk4::Button::with_label("Mark all read");
        mark_all_button.add_css_class("notification-mark-all");

        header.append(&title);
        header.append(&filter_dropdown);
        header.append(&mark_all_button);

        // Notification list
        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("notification-list");
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_child(Some(&list_box));
        scrolled.set_vexpand(true);

        container.append(&header);
        container.append(&scrolled);

        Self {
            container,
            header,
            filter_dropdown,
            list_box,
            scrolled,
            mark_all_button,
        }
    }

    pub fn load_notifications(&self, notifications: &[Notification]) {
        // Clear existing
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        for notification in notifications {
            let row = self.create_notification_row(notification);
            self.list_box.append(&row);
        }
    }

    fn create_notification_row(&self, notification: &Notification) -> gtk4::ListBoxRow {
        let row_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        row_box.set_margin_start(16);
        row_box.set_margin_end(16);
        row_box.set_margin_top(10);
        row_box.set_margin_bottom(10);

        // Top row: read indicator + severity icon + title
        let top = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let read_indicator = gtk4::Label::new(Some(
            if notification.is_read { "○" } else { "●" }
        ));
        read_indicator.add_css_class(
            if notification.is_read { "read-indicator" } else { "unread-indicator" }
        );
        let icon = gtk4::Label::new(Some(severity_icon(notification.severity)));
        let title = gtk4::Label::new(Some(&notification.title));
        title.add_css_class(if notification.is_read { "notification-title-read" } else { "notification-title" });
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);
        top.append(&read_indicator);
        top.append(&icon);
        top.append(&title);

        // Body
        let body = gtk4::Label::new(Some(&notification.body));
        body.add_css_class("notification-body");
        body.set_halign(gtk4::Align::Start);
        body.set_ellipsize(pango::EllipsizeMode::End);
        body.set_lines(2);
        body.set_margin_start(22); // Align with title (past indicator + icon)

        // Source + timestamp
        let meta = gtk4::Label::new(Some(&format!(
            "{} \u{00B7} {}",
            notification.session_id.as_deref().unwrap_or("System"),
            format_relative_time(notification.timestamp),
        )));
        meta.add_css_class("notification-meta");
        meta.set_halign(gtk4::Align::Start);
        meta.set_margin_start(22);

        row_box.append(&top);
        row_box.append(&body);
        row_box.append(&meta);

        let row = gtk4::ListBoxRow::new();
        row.set_child(Some(&row_box));
        row
    }
}
```

---

## 11. IPC Protocol Extensions

```rust
// Engine -> Chat Shell
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotificationIpcMessage {
    #[serde(rename = "notification_new")]
    New {
        notification: Notification,
    },
    #[serde(rename = "notification_badge_update")]
    BadgeUpdate {
        session_id: String,
        unread_count: usize,
    },
    #[serde(rename = "notification_dnd_changed")]
    DndChanged {
        enabled: bool,
    },
    #[serde(rename = "notification_history")]
    History {
        notifications: Vec<Notification>,
    },
}

// Chat Shell -> Engine
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotificationControlMessage {
    #[serde(rename = "notification_mark_read")]
    MarkRead { session_id: String },
    #[serde(rename = "notification_mark_all_read")]
    MarkAllRead,
    #[serde(rename = "notification_set_dnd")]
    SetDnd { enabled: bool },
    #[serde(rename = "notification_request_history")]
    RequestHistory {
        filter_type: Option<NotificationType>,
        filter_severity: Option<Severity>,
        limit: usize,
        offset: usize,
    },
}
```

---

## 12. Implementation Stages

**Stage 1 -- Notification Types & Bus (1 day)**
1. Define `Notification`, `NotificationType`, `Severity` types.
2. Implement `NotificationBus` with `tokio::broadcast`.
3. Unit tests for bus emit/subscribe, message delivery.

**Stage 2 -- Persistence (1 day)**
1. Create `notifications` SQLite table with migration.
2. Implement `NotificationStore` (insert, list, mark read, purge).
3. Unit tests for all CRUD operations and filtering.

**Stage 3 -- Notification Manager (1 day)**
1. Implement `NotificationManager` (coordinator).
2. Wire bus + store + DND.
3. Integration with session task runner (emit on complete/error).
4. Purge job on startup.

**Stage 4 -- Toast Renderer (2-3 days)**
1. `ToastWidget` with severity stripe, icon, title, body, dismiss.
2. `ToastRenderer` with stacking, auto-dismiss, hover-pause.
3. Slide-in/out animations via `GtkRevealer`.
4. DND suppression.
5. Click-to-navigate (switch session).

**Stage 5 -- Badge Counter (1 day)**
1. `BadgeCounter` with increment/clear/load.
2. Wire to sidebar `SessionRow` badge display (Module 20).
3. Wire to session list overlay badge display (Module 19).
4. Clear on session switch.

**Stage 6 -- Sound Playback (1 day)**
1. `SoundPlayer` with rodio/PipeWire backend.
2. Severity-specific sound files.
3. Volume control, DND muting.
4. Graceful fallback if audio init fails.

**Stage 7 -- Notification History Panel (1-2 days)**
1. `NotificationHistoryPanel` widget.
2. Filter dropdown (All, Tasks, System, Skills).
3. Mark all read button.
4. Wire to split-view content panel (Module 12).
5. `Ctrl+Shift+N` keyboard shortcut.

**Stage 8 -- DND & IPC (1 day)**
1. DND toggle in status bar.
2. DND toggle via chat command.
3. IPC protocol extensions for notification messages.
4. End-to-end integration testing.

---

## 13. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `types.rs` | Notification construction, serialization, defaults |
| `bus.rs` | Emit to multiple subscribers, dropped subscriber handling |
| `store.rs` | Insert, list with filters, mark read, purge old, unread counts |
| `sound.rs` | Sound file path resolution, graceful failure on missing files |
| `badge.rs` | Increment, clear, count, load from DB |

### Integration Tests

| Test | Method |
|------|--------|
| End-to-end notification | Emit from engine, verify toast appears, history persisted |
| Badge lifecycle | Background task completes, badge increments, switch session, badge clears |
| DND mode | Enable DND, emit notification, verify no toast, verify badge still updates |
| Toast stacking | Emit 4 notifications rapidly, verify max 3 visible, oldest dismissed |
| Toast dismiss | Hover toast, verify timer paused, click dismiss, verify removed |
| History panel | Open history, verify notifications listed, apply filter, verify filtered |
| Sound playback | Emit notification, verify sound played (integration with audio subsystem) |
| Purge | Insert old notifications, run purge, verify removed |
| Persistence | Emit notifications, restart engine, verify history survives |

### Cross-Module Tests

| Test | Description |
|------|-------------|
| Notification + concurrent sessions | Background task in session A completes while viewing B, verify toast + badge |
| Notification + split-view | Open notification history, verify renders in content panel (Module 12) |
| Notification + session sidebar | Badge appears on sidebar session row (Module 20) |
| Notification + session overlay | Badge appears on overlay session row (Module 19) |
| DND + status bar | Toggle DND, verify bell-off icon appears in status bar |
