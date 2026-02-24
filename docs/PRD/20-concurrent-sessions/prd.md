# 20 — Concurrent Sessions & Background Tasks: Product Requirements

**Module:** Concurrent Sessions & Background Tasks (L2 + L3)
**Phase:** 3
**Status:** Draft

---

## 1. Overview

Phase 2 introduced multi-session support (Module 19), but sessions are not concurrent — only one session is active at a time, and switching away from a session interrupts any in-progress task. Phase 3 removes this limitation. Sessions can now execute tasks in parallel: when a user switches away from a session mid-task, the task continues in the background. A persistent session sidebar replaces the overlay-only navigation, giving users continuous visibility into all sessions and their live status.

This is the foundation for a multi-threaded workflow — the user can ask one session to install packages, switch to another to edit a config file, and return to find the installation complete. Each session has its own async task runner, and tasks within a session execute sequentially while tasks across sessions run in parallel.

---

## 2. Functional Requirements

### 2.1 Background Task Execution (CS-01)

| Field | Value |
|-------|-------|
| **ID** | CS-01 |
| **Priority** | P0 |
| **Requirement** | Tasks continue executing when the user switches away from a session. |

**Details:**

- When the user switches from session A to session B while session A is processing a request, session A's task continues in the background.
- The user is not interrupted — session B is immediately interactive.
- When the user switches back to session A, they see the completed (or still-in-progress) result.
- Background tasks have full access to the engine: LLM API calls, tool execution, skill dispatch.
- If a background task requires user confirmation (destructive command), the session enters a `waiting_confirmation` state and the user is notified (see Module 21).

**Acceptance Criteria:**

- [ ] Switching away from a processing session does not cancel the task.
- [ ] Background tasks complete and their results are available when the user returns.
- [ ] Confirmation-required tasks pause and notify the user.

### 2.2 Session Task Runner (CS-02)

| Field | Value |
|-------|-------|
| **ID** | CS-02 |
| **Priority** | P0 |
| **Requirement** | Each session has its own async task runner that manages task execution independently. |

**Details:**

- Each session spawns a dedicated `tokio::task` that owns the session's execution loop.
- The task runner receives work items from a channel and executes them sequentially.
- The task runner communicates results back to the main engine via a result channel.
- Task runners are created when a session first receives a task, not at session creation time (lazy initialization).
- Task runners are dropped when a session is archived or deleted.

**Acceptance Criteria:**

- [ ] Each session has an independent task runner.
- [ ] Task runners are lazily initialized.
- [ ] Task runners are cleaned up on session archive/delete.

### 2.3 Task Queue (CS-03)

| Field | Value |
|-------|-------|
| **ID** | CS-03 |
| **Priority** | P0 |
| **Requirement** | Tasks execute sequentially within a session and in parallel across sessions. |

**Details:**

- Each session maintains a FIFO task queue.
- If the user sends a message to a session that is already processing, the new message is queued.
- Queued tasks execute in order after the current task completes.
- Tasks across different sessions execute concurrently, subject to resource limits (CS-06).
- The session sidebar shows queued task count.

**Acceptance Criteria:**

- [ ] Tasks within a session execute in order.
- [ ] Tasks across sessions execute in parallel.
- [ ] Queued tasks are visible in the sidebar.

### 2.4 Task Cancellation (CS-04)

| Field | Value |
|-------|-------|
| **ID** | CS-04 |
| **Priority** | P0 |
| **Requirement** | Users can cancel the current task in any session. |

**Details:**

- `Ctrl+C` cancels the current task in the active (foreground) session.
- "Cancel" or "stop" in chat cancels the current task in the active session.
- The session sidebar shows a cancel button on processing sessions — clicking it cancels that session's task regardless of which session is in the foreground.
- Cancellation is cooperative: the task runner checks a `CancellationToken` at safe points.
- Cancelled tasks are logged with a cancellation message in the session history.
- If a task has queued followers, cancellation only affects the current task — queued tasks proceed.

**Acceptance Criteria:**

- [ ] Ctrl+C cancels the active session's current task.
- [ ] Sidebar cancel button works for any session.
- [ ] Cancellation is logged in session history.
- [ ] Queued tasks are not affected by cancellation of the current task.

### 2.5 Session Sidebar (CS-05)

| Field | Value |
|-------|-------|
| **ID** | CS-05 |
| **Priority** | P0 |
| **Requirement** | A narrow sidebar panel shows all sessions with live status indicators, replacing overlay-only navigation. |

**Details:**

- The sidebar is a persistent left-side panel (200px wide) showing all active sessions.
- Each session row displays: name, status icon, and (when processing) a progress indicator.
- Status icons reflect the session's current state: idle, processing, complete (unread), error, queued.
- Clicking a session switches to it.
- The sidebar is toggled with `Ctrl+B`.
- When only one session exists, the sidebar is hidden by default. It auto-shows when a second session is created.
- The session list overlay (`Ctrl+Shift+S` from Module 19) remains available for archive/delete operations.

**Acceptance Criteria:**

- [ ] Sidebar displays all active sessions with status icons.
- [ ] Clicking a session in the sidebar switches to it.
- [ ] Sidebar toggles with Ctrl+B.
- [ ] Sidebar auto-shows when 2+ sessions exist.
- [ ] Status icons update in real-time.

### 2.6 Resource Limiting (CS-06)

| Field | Value |
|-------|-------|
| **ID** | CS-06 |
| **Priority** | P1 |
| **Requirement** | Concurrent LLM requests are capped to prevent API rate limiting and local resource exhaustion. |

**Details:**

- Maximum concurrent cloud LLM requests: 3 (configurable).
- Maximum concurrent local LLM requests: 1 (local model is single-threaded).
- When the limit is reached, new requests are queued at the resource level (distinct from the per-session task queue).
- The resource queue is FIFO — first-requested, first-served across sessions.
- Resource limits are configured in `/etc/levsha/config.toml`.

**Acceptance Criteria:**

- [ ] No more than 3 concurrent cloud LLM requests.
- [ ] No more than 1 concurrent local LLM request.
- [ ] Excess requests are queued and executed in order.
- [ ] Limits are configurable.

### 2.7 Session State Machine (CS-07)

| Field | Value |
|-------|-------|
| **ID** | CS-07 |
| **Priority** | P1 |
| **Requirement** | Each session has a well-defined state machine that governs its behavior and UI representation. |

**Details:**

- States: `Idle`, `Processing`, `WaitingConfirmation`, `Background`.
- `Idle` — session is ready for input, no active task.
- `Processing` — session is executing a task (foreground or background).
- `WaitingConfirmation` — a background task hit a destructive command and needs user approval.
- `Background` — legacy alias; a session transitions from `Processing` to `Background` when the user switches away. Functionally identical to `Processing` but tracked for UI purposes.
- State transitions trigger IPC messages to the Chat Shell for UI updates.

**Acceptance Criteria:**

- [ ] Sessions transition between states correctly.
- [ ] State changes are reflected in the sidebar UI.
- [ ] WaitingConfirmation state pauses task execution.

---

## 3. Configuration

New config entries in `/etc/levsha/config.toml`:

```toml
[sessions.concurrency]
max_cloud_requests = 3       # Maximum concurrent cloud LLM API requests
max_local_requests = 1       # Maximum concurrent local LLM requests
task_queue_max = 10          # Maximum queued tasks per session

[sessions.sidebar]
default_width = 200          # Sidebar width in pixels
auto_show_threshold = 2      # Show sidebar when this many sessions exist
```

---

## 4. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Multi-Session (19) | Extends | Builds on session infrastructure — adds concurrency to existing session model. |
| Chat Shell (02) | Modified | Session sidebar widget, status icons, cancel button, layout changes. |
| Intelligence Engine (03) | Modified | Per-session task runners, resource limiter, task queue, state machine. |
| Notification System (21) | Consumer | Background task completion and error events produce notifications. |

---

## 5. Out of Scope

- Session pinning (keeping a session always visible in a split view).
- Priority scheduling (all sessions have equal priority).
- Distributed execution (tasks run on the local machine only).
- Cross-session task dependencies.
- Task retry logic (failed tasks are reported, not retried).
- Concurrent tasks within a single session.
