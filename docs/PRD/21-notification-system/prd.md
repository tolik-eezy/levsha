# 21 — Notification System: Product Requirements

**Module:** Notification System (L2 + L3)
**Phase:** 3
**Status:** Draft

---

## 1. Overview

With concurrent sessions (Module 20), the user is often looking at one session while another completes a task in the background. The notification system provides a unified channel for alerting the user to events that happen outside their current view: background task completions, errors, attention-needed confirmations, skill installations, and system-level alerts.

Notifications are delivered as toast popups, session badges, and an optional history panel. The system is built on a typed pub/sub bus, making it extensible for future event sources.

---

## 2. Functional Requirements

### 2.1 Notification Bus (NS-01)

| Field | Value |
|-------|-------|
| **ID** | NS-01 |
| **Priority** | P0 |
| **Requirement** | A pub/sub event channel carries typed notifications from producers to consumers. |

**Details:**

- The notification bus is a `tokio::broadcast` channel with typed `Notification` messages.
- Producers: session task runners (completion, error, confirmation needed), skill system (install/remove), system events (low disk, network change).
- Consumers: toast renderer, badge counter, history persistence, sound player.
- Each notification has: id, type, severity, title, body, timestamp, source session (optional), read status.
- The bus is fire-and-forget for producers — dropped messages are acceptable under load.

**Acceptance Criteria:**

- [ ] Notification bus delivers typed messages to multiple consumers.
- [ ] Producers can emit notifications without blocking.
- [ ] Multiple consumers receive the same notification.

### 2.2 Toast Notifications (NS-02)

| Field | Value |
|-------|-------|
| **ID** | NS-02 |
| **Priority** | P0 |
| **Requirement** | Animated toast popups appear for background events, providing immediate visual feedback. |

**Details:**

- Toasts slide in from the top-right corner of the screen.
- Each toast shows: severity icon, title, body (truncated), and timestamp.
- Toasts auto-dismiss after 3 seconds (configurable).
- Clicking a toast navigates to the relevant session (if applicable).
- Hover over a toast pauses the dismiss timer.
- Maximum 3 toasts visible at a time — newer toasts push older ones up.
- Toasts respect Do Not Disturb mode (NS-07).

**Acceptance Criteria:**

- [ ] Toasts appear for background events.
- [ ] Toasts auto-dismiss after configured duration.
- [ ] Clicking a toast navigates to the source session.
- [ ] Maximum 3 toasts visible simultaneously.
- [ ] Hover pauses dismiss timer.

### 2.3 Session Badges (NS-03)

| Field | Value |
|-------|-------|
| **ID** | NS-03 |
| **Priority** | P0 |
| **Requirement** | Sessions with unread notifications show a badge count in the sidebar and session list. |

**Details:**

- When a background session produces a notification, its badge count increments.
- The badge appears on the session row in the sidebar (Module 20) and in the session list overlay (Module 19).
- Switching to a session clears its badge (marks all notifications as read for that session).
- Badge counts cap at "9+" for display purposes.

**Acceptance Criteria:**

- [ ] Badge count increments for background session notifications.
- [ ] Badge appears in sidebar and session list overlay.
- [ ] Switching to a session clears its badge.

### 2.4 Notification Persistence (NS-04)

| Field | Value |
|-------|-------|
| **ID** | NS-04 |
| **Priority** | P0 |
| **Requirement** | Notifications are stored in SQLite for history and retrieval. |

**Details:**

- All notifications are persisted to a `notifications` table.
- Notifications are retained for 30 days (configurable), then auto-purged.
- Each notification stores: id, type, severity, title, body, timestamp, session_id (nullable), is_read.
- Notifications can be queried by type, severity, session, and read status.

**Acceptance Criteria:**

- [ ] Notifications are persisted to SQLite.
- [ ] Notifications older than the retention period are auto-purged.
- [ ] Notifications can be queried by type and session.

### 2.5 Sound Alerts (NS-05)

| Field | Value |
|-------|-------|
| **ID** | NS-05 |
| **Priority** | P1 |
| **Requirement** | Notification sounds play via PipeWire for audible alerts. |

**Details:**

- A short, warm notification sound plays for each toast (unless DND is active).
- Different sounds for different severities: info (soft chime), warning (double chime), error (low tone).
- Sound files are bundled in `/usr/share/levsha/sounds/`.
- Volume follows system volume settings.
- Sounds can be disabled independently of toasts.

**Acceptance Criteria:**

- [ ] Notification sounds play for each severity level.
- [ ] Sounds are disabled in DND mode.
- [ ] Sounds can be independently disabled in configuration.

### 2.6 Notification History Panel (NS-06)

| Field | Value |
|-------|-------|
| **ID** | NS-06 |
| **Priority** | P1 |
| **Requirement** | A history panel displays all past notifications with filtering. |

**Details:**

- Opened with `Ctrl+Shift+N`.
- Renders in the split-view content panel (Module 12).
- Shows a scrollable list of notifications: icon, title, body, timestamp, source session.
- Unread notifications are visually distinct (bold, dot indicator).
- Filter by type (all, tasks, system, skills) and severity (all, info, warning, error).
- "Mark all as read" button.
- Opening the history panel marks visible notifications as read.

**Acceptance Criteria:**

- [ ] History panel opens with Ctrl+Shift+N.
- [ ] All past notifications are displayed.
- [ ] Notifications can be filtered by type and severity.
- [ ] Unread notifications are visually distinct.

### 2.7 Do Not Disturb Mode (NS-07)

| Field | Value |
|-------|-------|
| **ID** | NS-07 |
| **Priority** | P1 |
| **Requirement** | A Do Not Disturb mode suppresses toasts and sounds while still recording notifications. |

**Details:**

- Toggle via "do not disturb" / "dnd on" / "dnd off" in chat, or clicking the bell icon in the status bar.
- When DND is active: toasts are suppressed, sounds are muted, badges still update.
- DND indicator: bell icon with slash in the status bar.
- Notifications are still persisted and badges still increment — only the intrusive presentation is suppressed.

**Acceptance Criteria:**

- [ ] DND mode suppresses toasts and sounds.
- [ ] Badges still update in DND mode.
- [ ] DND is toggleable via chat and status bar.
- [ ] DND indicator is visible in the status bar.

---

## 3. Notification Types

| Type | Source | Example |
|------|--------|---------|
| `TaskComplete` | Session task runner | "Package Install: Completed successfully" |
| `TaskError` | Session task runner | "Build: Failed with 3 errors" |
| `AttentionNeeded` | Session task runner | "Server Setup: Confirmation required for rm -rf" |
| `SkillInstalled` | Skill system | "Installed skill: docker-manager" |
| `SystemAlert` | System monitor | "Disk usage at 90%" |
| `SessionMessage` | Session | "Coding Project: 2 new messages" |

### Severity Levels

| Severity | Usage | Toast Duration | Sound |
|----------|-------|---------------|-------|
| `info` | Normal completions, installs | 3s | Soft chime |
| `warning` | Attention needed, resource warnings | 5s | Double chime |
| `error` | Task failures, system errors | 8s (manual dismiss) | Low tone |

---

## 4. Configuration

New config entries in `/etc/levsha/config.toml`:

```toml
[notifications]
enabled = true
retention_days = 30            # Days to keep notifications
max_stored = 1000              # Maximum stored notifications

[notifications.toast]
enabled = true
duration_info = 3              # Seconds for info toasts
duration_warning = 5           # Seconds for warning toasts
duration_error = 0             # 0 = manual dismiss only
max_visible = 3                # Maximum simultaneous toasts

[notifications.sound]
enabled = true
volume = 0.7                   # 0.0 to 1.0, relative to system volume

[notifications.dnd]
enabled = false                # DND off by default
```

---

## 5. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Concurrent Sessions (20) | Producer | Background task completion and errors produce notifications. |
| Chat Shell (02) | Modified | Toast renderer, badge display, DND indicator, history panel. |
| Intelligence Engine (03) | Modified | Notification bus, notification persistence, event emission. |
| Persistence (07) | Modified | New notifications table. |
| Split-View (12) | Consumer | Notification history renders in the split-view content panel. |
| Skill Repository (11) | Producer | Skill install/remove events produce notifications. |

---

## 6. Out of Scope

- Push notifications to external devices (phone, watch).
- Email notifications.
- Webhook integrations.
- Per-notification action buttons (e.g., "Retry" on error toasts).
- Notification grouping or bundling.
- Custom notification sounds (user-uploaded).
- Notification priority levels (beyond severity).
