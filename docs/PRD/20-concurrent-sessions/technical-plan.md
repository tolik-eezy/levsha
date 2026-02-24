# 20 — Concurrent Sessions & Background Tasks: Technical Plan

**Module:** Concurrent Sessions & Background Tasks (L2 + L3)
**Language:** Rust
**Phase:** 3

---

## 1. Crate Structure

```
engine/
  src/
    sessions/
      mod.rs              # SessionManager — extended with concurrency
      store.rs            # SQLite session persistence (from Module 19)
      state.rs            # Per-session runtime state (from Module 19)
      runner.rs           # NEW: SessionTaskRunner (tokio::task per session)
      queue.rs            # NEW: Per-session task queue (VecDeque)
      state_machine.rs    # NEW: SessionState enum and transitions
    concurrency/
      mod.rs              # NEW: Resource limiter, coordination
      limiter.rs          # NEW: Semaphore-based LLM request limiter

chat-shell/
  src/
    ui/
      session_sidebar/
        mod.rs            # NEW: SessionSidebar widget
        row.rs            # NEW: SessionRow with status icon, progress, cancel
        progress_bar.rs   # NEW: Indeterminate/determinate progress bar
      session_list.rs     # Existing overlay (from Module 19)
      session_bar.rs      # Status bar session name (from Module 19)
    state/
      session.rs          # Modified: track sidebar visibility, unread counts
```

### New Dependencies

```toml
# Added to engine/Cargo.toml
tokio-util = "0.7"    # CancellationToken
```

---

## 2. Session State Machine

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    /// Session is ready for input, no active task.
    Idle,
    /// Session is executing a task (foreground or background).
    Processing,
    /// A background task requires user confirmation (destructive command).
    WaitingConfirmation,
    /// Session was processing when the user switched away.
    /// Functionally identical to Processing, tracked for UI.
    Background,
}

impl SessionState {
    pub fn can_accept_input(&self) -> bool {
        // All states can queue input, but only Idle processes immediately
        true
    }

    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Processing | Self::Background | Self::WaitingConfirmation)
    }

    pub fn transition(&self, event: SessionEvent) -> Option<SessionState> {
        match (self, event) {
            (SessionState::Idle, SessionEvent::TaskSubmitted) => Some(SessionState::Processing),
            (SessionState::Processing, SessionEvent::UserSwitchedAway) => Some(SessionState::Background),
            (SessionState::Background, SessionEvent::UserSwitchedBack) => Some(SessionState::Processing),
            (SessionState::Processing, SessionEvent::TaskComplete) => Some(SessionState::Idle),
            (SessionState::Background, SessionEvent::TaskComplete) => Some(SessionState::Idle),
            (SessionState::Processing, SessionEvent::ConfirmationRequired) => Some(SessionState::WaitingConfirmation),
            (SessionState::Background, SessionEvent::ConfirmationRequired) => Some(SessionState::WaitingConfirmation),
            (SessionState::WaitingConfirmation, SessionEvent::ConfirmationGiven) => Some(SessionState::Processing),
            (SessionState::WaitingConfirmation, SessionEvent::TaskCancelled) => Some(SessionState::Idle),
            (SessionState::Processing, SessionEvent::TaskCancelled) => Some(SessionState::Idle),
            (SessionState::Background, SessionEvent::TaskCancelled) => Some(SessionState::Idle),
            _ => None, // Invalid transition
        }
    }
}

#[derive(Debug, Clone)]
pub enum SessionEvent {
    TaskSubmitted,
    TaskComplete,
    TaskCancelled,
    ConfirmationRequired,
    ConfirmationGiven,
    UserSwitchedAway,
    UserSwitchedBack,
}
```

---

## 3. Session Task Runner

Each session gets a dedicated async task that owns its execution loop.

```rust
pub struct SessionTaskRunner {
    session_id: String,
    task_tx: mpsc::Sender<TaskItem>,
    cancel_token: CancellationToken,
    join_handle: JoinHandle<()>,
}

pub struct TaskItem {
    pub id: String,
    pub messages: Vec<Message>,
    pub cancel_token: CancellationToken,
}

pub enum TaskResult {
    Complete {
        session_id: String,
        task_id: String,
        response: Vec<Message>,
    },
    Error {
        session_id: String,
        task_id: String,
        error: String,
    },
    Cancelled {
        session_id: String,
        task_id: String,
    },
    NeedsConfirmation {
        session_id: String,
        task_id: String,
        command: String,
        confirmation_prompt: String,
    },
}

impl SessionTaskRunner {
    pub fn spawn(
        session_id: String,
        engine: Arc<Engine>,
        resource_limiter: Arc<ResourceLimiter>,
        result_tx: mpsc::Sender<TaskResult>,
    ) -> Self {
        let (task_tx, mut task_rx) = mpsc::channel::<TaskItem>(16);
        let cancel_token = CancellationToken::new();
        let session_id_clone = session_id.clone();

        let join_handle = tokio::spawn(async move {
            while let Some(task) = task_rx.recv().await {
                // Acquire resource permit before making LLM calls
                let _permit = resource_limiter.acquire().await;

                let result = tokio::select! {
                    res = engine.process_task(&session_id_clone, &task) => {
                        match res {
                            Ok(response) => TaskResult::Complete {
                                session_id: session_id_clone.clone(),
                                task_id: task.id.clone(),
                                response,
                            },
                            Err(e) => TaskResult::Error {
                                session_id: session_id_clone.clone(),
                                task_id: task.id.clone(),
                                error: e.to_string(),
                            },
                        }
                    }
                    _ = task.cancel_token.cancelled() => {
                        TaskResult::Cancelled {
                            session_id: session_id_clone.clone(),
                            task_id: task.id.clone(),
                        }
                    }
                };

                let _ = result_tx.send(result).await;
            }
        });

        Self {
            session_id,
            task_tx,
            cancel_token,
            join_handle,
        }
    }

    pub async fn submit(&self, task: TaskItem) -> Result<()> {
        self.task_tx.send(task).await
            .map_err(|_| SessionError::RunnerClosed)
    }

    pub fn cancel_current(&self) {
        self.cancel_token.cancel();
    }

    pub async fn shutdown(self) {
        drop(self.task_tx);
        let _ = self.join_handle.await;
    }
}
```

---

## 4. Task Queue

```rust
pub struct TaskQueue {
    queue: VecDeque<TaskItem>,
    max_size: usize,
}

impl TaskQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn enqueue(&mut self, task: TaskItem) -> Result<()> {
        if self.queue.len() >= self.max_size {
            return Err(SessionError::QueueFull);
        }
        self.queue.push_back(task);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<TaskItem> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}
```

---

## 5. Resource Limiter

```rust
pub struct ResourceLimiter {
    cloud_semaphore: Semaphore,
    local_semaphore: Semaphore,
}

pub struct ResourcePermit {
    _permit: SemaphorePermit<'static>,
}

impl ResourceLimiter {
    pub fn new(max_cloud: usize, max_local: usize) -> Self {
        Self {
            cloud_semaphore: Semaphore::new(max_cloud),
            local_semaphore: Semaphore::new(max_local),
        }
    }

    pub async fn acquire_cloud(&self) -> ResourcePermit {
        let permit = self.cloud_semaphore.acquire().await
            .expect("semaphore closed");
        ResourcePermit { _permit: permit }
    }

    pub async fn acquire_local(&self) -> ResourcePermit {
        let permit = self.local_semaphore.acquire().await
            .expect("semaphore closed");
        ResourcePermit { _permit: permit }
    }

    pub fn cloud_available(&self) -> usize {
        self.cloud_semaphore.available_permits()
    }

    pub fn local_available(&self) -> usize {
        self.local_semaphore.available_permits()
    }
}
```

---

## 6. Session Manager Extensions

The existing `SessionManager` from Module 19 is extended with concurrency support.

```rust
pub struct SessionManager {
    // From Module 19
    db: Arc<rusqlite::Connection>,
    active_session_id: String,
    session_states: HashMap<String, SessionRuntimeState>,
    config: SessionConfig,

    // New in Module 20
    task_runners: HashMap<String, SessionTaskRunner>,
    task_queues: HashMap<String, TaskQueue>,
    session_state_machine: HashMap<String, SessionState>,
    resource_limiter: Arc<ResourceLimiter>,
    result_tx: mpsc::Sender<TaskResult>,
    result_rx: mpsc::Receiver<TaskResult>,
}

impl SessionManager {
    /// Submit a task to a session. If the session is busy, the task is queued.
    pub async fn submit_task(&mut self, session_id: &str, messages: Vec<Message>) -> Result<()> {
        let task = TaskItem {
            id: uuid::Uuid::new_v4().to_string(),
            messages,
            cancel_token: CancellationToken::new(),
        };

        let state = self.session_state_machine
            .get(session_id)
            .copied()
            .unwrap_or(SessionState::Idle);

        if state.is_busy() {
            // Session is busy — queue the task
            let queue = self.task_queues
                .entry(session_id.to_string())
                .or_insert_with(|| TaskQueue::new(self.config.concurrency.task_queue_max));
            queue.enqueue(task)?;
            self.notify_ui(session_id, UiEvent::QueueUpdated(queue.len()));
        } else {
            // Session is idle — start immediately
            self.start_task(session_id, task).await?;
        }

        Ok(())
    }

    async fn start_task(&mut self, session_id: &str, task: TaskItem) -> Result<()> {
        // Lazily create the task runner
        let runner = self.task_runners
            .entry(session_id.to_string())
            .or_insert_with(|| {
                SessionTaskRunner::spawn(
                    session_id.to_string(),
                    self.engine.clone(),
                    self.resource_limiter.clone(),
                    self.result_tx.clone(),
                )
            });

        runner.submit(task).await?;

        // Transition state
        self.transition_state(session_id, SessionEvent::TaskSubmitted);

        Ok(())
    }

    /// Cancel the current task in a session.
    pub fn cancel_task(&mut self, session_id: &str) -> Result<()> {
        if let Some(runner) = self.task_runners.get(session_id) {
            runner.cancel_current();
        }
        self.transition_state(session_id, SessionEvent::TaskCancelled);
        Ok(())
    }

    /// Handle user switching away from a session.
    pub fn on_session_deactivated(&mut self, session_id: &str) {
        self.transition_state(session_id, SessionEvent::UserSwitchedAway);
    }

    /// Handle user switching back to a session.
    pub fn on_session_activated(&mut self, session_id: &str) {
        self.transition_state(session_id, SessionEvent::UserSwitchedBack);
    }

    fn transition_state(&mut self, session_id: &str, event: SessionEvent) {
        let current = self.session_state_machine
            .get(session_id)
            .copied()
            .unwrap_or(SessionState::Idle);

        if let Some(new_state) = current.transition(event) {
            self.session_state_machine.insert(session_id.to_string(), new_state);
            self.notify_ui(session_id, UiEvent::StateChanged(new_state));
        }
    }

    /// Process results from task runners (called in main loop).
    pub async fn poll_results(&mut self) -> Option<TaskResult> {
        self.result_rx.recv().await
    }
}
```

---

## 7. IPC Protocol Extensions

```rust
// Engine -> Chat Shell
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionStatusMessage {
    #[serde(rename = "session_state_changed")]
    StateChanged {
        session_id: String,
        state: SessionState,
    },
    #[serde(rename = "task_progress")]
    TaskProgress {
        session_id: String,
        task_id: String,
        progress: TaskProgressInfo,
    },
    #[serde(rename = "task_complete")]
    TaskComplete {
        session_id: String,
        task_id: String,
        response: Vec<Message>,
    },
    #[serde(rename = "task_error")]
    TaskError {
        session_id: String,
        task_id: String,
        error: String,
    },
    #[serde(rename = "task_cancelled")]
    TaskCancelled {
        session_id: String,
        task_id: String,
    },
    #[serde(rename = "queue_updated")]
    QueueUpdated {
        session_id: String,
        queued_count: usize,
    },
}

#[derive(Serialize, Deserialize)]
pub struct TaskProgressInfo {
    pub percent: Option<f64>,       // None = indeterminate
    pub step_label: Option<String>,
}

// Chat Shell -> Engine
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionControlMessage {
    #[serde(rename = "task_cancel")]
    CancelTask { session_id: String },
    #[serde(rename = "confirmation_response")]
    ConfirmationResponse {
        session_id: String,
        task_id: String,
        confirmed: bool,
    },
}
```

---

## 8. Session Sidebar Widget (Chat Shell)

```rust
pub struct SessionSidebar {
    container: gtk4::Box,
    revealer: gtk4::Revealer,
    list_box: gtk4::ListBox,
    new_button: gtk4::Button,
    session_rows: HashMap<String, SessionRow>,
    is_user_hidden: Cell<bool>,
}

pub struct SessionRow {
    row: gtk4::ListBoxRow,
    row_box: gtk4::Box,
    status_icon: gtk4::Label,
    name_label: gtk4::Label,
    progress_bar: gtk4::ProgressBar,
    cancel_button: gtk4::Button,
    badge: gtk4::Label,
    session_id: String,
}

impl SessionSidebar {
    pub fn new() -> Self {
        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::SlideRight);
        revealer.set_transition_duration(250);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.add_css_class("session-sidebar");
        container.set_width_request(200);

        // Header
        let header = gtk4::Label::new(Some("SESSIONS"));
        header.add_css_class("sidebar-header");
        header.set_halign(gtk4::Align::Start);
        header.set_margin_start(16);
        header.set_margin_top(12);
        header.set_margin_bottom(8);
        container.append(&header);

        // Session list
        let list_box = gtk4::ListBox::new();
        list_box.add_css_class("sidebar-list");
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_child(Some(&list_box));
        scrolled.set_vexpand(true);
        container.append(&scrolled);

        // New session button
        let new_button = gtk4::Button::with_label("+ New");
        new_button.add_css_class("sidebar-new-button");
        new_button.set_margin_start(12);
        new_button.set_margin_end(12);
        new_button.set_margin_top(8);
        new_button.set_margin_bottom(8);
        container.append(&new_button);

        revealer.set_child(Some(&container));

        Self {
            container,
            revealer,
            list_box,
            new_button,
            session_rows: HashMap::new(),
            is_user_hidden: Cell::new(false),
        }
    }

    pub fn toggle(&self) {
        let visible = self.revealer.reveals_child();
        self.revealer.set_reveal_child(!visible);
        self.is_user_hidden.set(visible);
    }

    pub fn auto_show(&self, session_count: usize, threshold: usize) {
        if self.is_user_hidden.get() {
            return; // User manually hid — respect their choice
        }
        self.revealer.set_reveal_child(session_count >= threshold);
    }

    pub fn update_session_state(&self, session_id: &str, state: SessionState) {
        if let Some(row) = self.session_rows.get(session_id) {
            let (icon, css_class) = match state {
                SessionState::Idle => ("○", "status-idle"),
                SessionState::Processing | SessionState::Background => ("◐", "status-processing"),
                SessionState::WaitingConfirmation => ("⏳", "status-waiting"),
            };
            row.status_icon.set_label(icon);
            row.status_icon.set_css_classes(&[css_class]);
            row.progress_bar.set_visible(state.is_busy());
            row.cancel_button.set_visible(state.is_busy());
        }
    }

    pub fn update_progress(&self, session_id: &str, progress: &TaskProgressInfo) {
        if let Some(row) = self.session_rows.get(session_id) {
            match progress.percent {
                Some(pct) => {
                    row.progress_bar.set_fraction(pct / 100.0);
                }
                None => {
                    row.progress_bar.pulse();
                }
            }
        }
    }

    pub fn set_badge(&self, session_id: &str, count: usize) {
        if let Some(row) = self.session_rows.get(session_id) {
            if count == 0 {
                row.badge.set_visible(false);
            } else {
                let text = if count > 9 { "9+".to_string() } else { count.to_string() };
                row.badge.set_label(&text);
                row.badge.set_visible(true);
            }
        }
    }
}
```

---

## 9. Main Layout Integration

The Chat Shell layout changes to accommodate the sidebar.

```rust
pub struct MainLayout {
    outer_box: gtk4::Box,          // Horizontal: sidebar + content
    sidebar: SessionSidebar,
    content_area: gtk4::Box,       // Vertical: chat + input + status bar
    split_view: SplitViewContainer, // From Module 12
}

impl MainLayout {
    pub fn new(sidebar: SessionSidebar, split_view: SplitViewContainer) -> Self {
        let outer_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);

        // Sidebar on the left
        outer_box.append(&sidebar.revealer);

        // Content area (chat + optional split view) on the right
        let content_area = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        content_area.set_hexpand(true);
        content_area.append(&split_view.widget());
        outer_box.append(&content_area);

        Self {
            outer_box,
            sidebar,
            content_area,
            split_view,
        }
    }
}
```

---

## 10. Implementation Stages

**Stage 1 -- Session State Machine (1 day)**
1. Implement `SessionState` enum with transitions.
2. Integrate state tracking into `SessionManager`.
3. Unit tests for all valid and invalid transitions.

**Stage 2 -- Task Runner & Queue (2 days)**
1. Implement `SessionTaskRunner` with `tokio::task` and `CancellationToken`.
2. Implement `TaskQueue` with FIFO ordering.
3. Wire task submission through `SessionManager`.
4. Task cancellation via `CancellationToken`.
5. Unit tests for runner lifecycle, queue ordering, cancellation.

**Stage 3 -- Resource Limiter (1 day)**
1. Implement `ResourceLimiter` with `tokio::sync::Semaphore`.
2. Integrate with task runner (acquire permit before LLM call).
3. Configuration loading from `config.toml`.
4. Tests for concurrent limit enforcement.

**Stage 4 -- IPC Protocol Extensions (1 day)**
1. Add `SessionStatusMessage` and `SessionControlMessage` to IPC.
2. Engine-side handlers for cancel and confirmation response.
3. Wire state transitions to IPC messages.
4. Serialization/deserialization tests.

**Stage 5 -- Session Sidebar Widget (2-3 days)**
1. `SessionSidebar` widget with GTK4 ListBox.
2. `SessionRow` with status icon, name, progress bar, cancel button, badge.
3. Sidebar show/hide animation with `GtkRevealer`.
4. Auto-show logic (2+ sessions).
5. Click-to-switch wiring.
6. `Ctrl+B` toggle keyboard shortcut.

**Stage 6 -- Integration & Polish (1-2 days)**
1. Wire engine task results to sidebar state updates.
2. Wire sidebar cancel button to engine cancellation.
3. Badge management (set on completion, clear on switch).
4. Main layout integration (sidebar + chat + split-view).
5. End-to-end testing.

---

## 11. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `state_machine.rs` | All state transitions, invalid transition rejection |
| `runner.rs` | Task submission, completion, cancellation, shutdown |
| `queue.rs` | Enqueue, dequeue, max size enforcement, clear |
| `limiter.rs` | Permit acquisition, concurrent limit, availability count |

### Integration Tests

| Test | Method |
|------|--------|
| Background execution | Submit task in session A, switch to B, verify A completes |
| Task queuing | Submit 3 tasks to one session, verify sequential execution |
| Cancellation | Submit task, cancel, verify cancellation result |
| Resource limiting | Submit 5 cloud tasks across sessions, verify max 3 concurrent |
| State transitions | Verify state changes through full lifecycle |
| Sidebar updates | Submit task, verify sidebar shows processing state |
| Badge management | Complete background task, verify badge appears, switch to session, verify badge clears |
| Queue overflow | Submit tasks beyond max_queue, verify error |
| Runner cleanup | Archive session, verify runner is dropped |

### Cross-Module Tests

| Test | Description |
|------|-------------|
| Concurrent + split-view | Background task opens split-view content, switch away, switch back, verify content preserved |
| Concurrent + notifications | Background task completes, verify notification is emitted (Module 21) |
| Concurrent + confirmation | Background task hits destructive command, verify WaitingConfirmation state and notification |
| Sidebar + session list overlay | Sidebar visible, open overlay, perform archive, verify sidebar updates |
