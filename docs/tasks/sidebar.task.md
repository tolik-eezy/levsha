# Levsha OS — Module 26: Session Sidebar & Navigation Implementation Plan

## Context

Module 26 introduces a persistent left sidebar as the unified navigation hub for Levsha OS. Currently, session navigation is handled by a modal overlay (`Ctrl+Shift+S`, Module 19) and Module 20 defines a sidebar concept for concurrent session status. Module 26 upgrades this into a full ChatGPT/Claude-style sidebar with session history, settings, and power controls.

**PRD:** `docs/PRD/26-session-sidebar/design.md`

### Architecture Summary

```
┌──────────────────┬──────────────────────────────────────┐
│ Toggle [☰ / ✕]   │                                      │
├──────────────────┤                                      │
│ Session List     │           Chat View                  │
│ (scrollable)     │     (existing: chat + input +        │
│                  │      status bar + content panel)      │
├──────────────────┤                                      │
│ [+ New Session]  │                                      │
├──────────────────┤                                      │
│ ⚙ Settings       │                                      │
│ ⏻ Shut Down      │                                      │
└──────────────────┴──────────────────────────────────────┘
```

The sidebar is a new top-level widget in the window layout. The existing `GtkPaned` (chat + content panel) becomes the right child. The sidebar is the left child of a new outer horizontal container.

### Key Files (Existing)

| File | Relevance |
|------|-----------|
| `chat-shell/src/window.rs` | Main layout — sidebar inserts here as outer container |
| `chat-shell/src/session_list.rs` | Existing overlay — sidebar reuses session rendering logic |
| `chat-shell/src/keybindings.rs` | `Ctrl+B` toggle, updated shortcut routing |
| `chat-shell/src/status_bar.rs` | May lose session name segment when sidebar visible |
| `chat-shell/src/app.rs` | Application-level state |
| `engine/src/session/manager.rs` | Session CRUD, provides data for sidebar list |
| `engine/src/session/store.rs` | SQLite persistence for sessions |
| `engine/src/types.rs` | IPC message types (ShellToEngine / EngineToShell) |
| `engine/src/config.rs` | Sidebar visibility preference persistence |

---

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| Chat Shell (02) | Required | Sidebar modifies the main window layout |
| Multi-Session (19) | Required | Session list data, overlay remains for archive/delete |
| Concurrent Sessions (20) | Required | Status icons, progress bars, unread badges carry forward |
| Persistence (07) | Required | Sidebar visibility preference stored in config |
| GTK4 / libadwaita | Required | Widget toolkit for sidebar implementation |

---

## Dependency Graph

```
Stage 1 — Sidebar widget + toggle button
    │
    ├── Stage 2 — Session list rendering (reuse from overlay)
    │       │
    │       ├── Stage 3 — Bottom zone (Settings + Shut Down)
    │       │
    │       └── Stage 4 — Window layout integration
    │
    └── Stage 5 — Animations + responsive behavior
            │
            └── Stage 6 — Config persistence + testing
```

---

## Stage 1 — Sidebar Container & Toggle Button (~2 hrs, `chat-shell` agent)

**Files:** `chat-shell/src/sidebar.rs` (new), `chat-shell/src/window.rs`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| `SessionSidebar` struct | `chat-shell` | `sidebar.rs` | New widget wrapping a `gtk4::Box` (vertical). Three zones: header (toggle), session list (scrollable), bottom (settings/power). Width 260px, `$bg-surface` background, right border. |
| Toggle button | `chat-shell` | `sidebar.rs` | 36x36 `gtk4::Button` with Lucide `panel-left` / `panel-left-close` icon (18px). Positioned in sidebar header row (48px height). Click toggles sidebar visibility via `gtk4::Revealer`. |
| `SidebarState` in `AppState` | `chat-shell` | `window.rs` | Add `sidebar_visible: bool`, `sidebar_user_toggled: bool` to `AppState`. Controls toggle logic (auto-show on 2+ sessions vs manual override). |
| Revealer-based show/hide | `chat-shell` | `sidebar.rs` | Wrap sidebar content in `gtk4::Revealer` with `SlideRight` transition. Show: 250ms ease-out. Hide: 200ms ease-in. |
| CSS classes | `chat-shell` | `style.css` | Classes: `.sidebar-container`, `.sidebar-toggle-btn`, `.sidebar-toggle-btn:hover`. Apply theme tokens for background, border, hover states. |

**Deliverable:** Sidebar widget renders and toggles on/off. No session data yet — just the container and toggle button.

---

## Stage 2 — Session List Rendering (~2 hrs, `chat-shell` agent)

**Files:** `chat-shell/src/sidebar.rs`, `chat-shell/src/session_list.rs`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Extract shared session row logic | `chat-shell` | `session_list.rs` | Refactor `SessionListOverlay` to extract session row creation into a shared `build_session_row()` function. Reused by both overlay and sidebar. |
| Sidebar session list | `chat-shell` | `sidebar.rs` | Scrollable `gtk4::ListBox` in the middle zone. Populated from `Vec<SessionInfo>`. Active session highlighted with left copper border + `$bg-secondary` background. |
| Session row layout | `chat-shell` | `sidebar.rs` | Each row: status icon (8px circle) + session name (14px, truncated) + relative timestamp (12px, `$text-tertiary`). 10px 16px padding. 6px corner radius. |
| Status icon integration | `chat-shell` | `sidebar.rs` | Reuse Module 20 status icons: ○ idle, ◐ processing, ✓ complete, ✗ error, ⏳ queued. Pulse animation on processing (1.2s cycle). |
| Click-to-switch | `chat-shell` | `sidebar.rs` | Clicking a session row sends `ShellToEngine::SessionSwitch`. Active row highlight updates immediately. |
| Context menu | `chat-shell` | `sidebar.rs` | Right-click on session row shows popover menu: Rename (`pencil`), Archive (`archive`), Delete (`trash-2`). Delete uses `$danger-text` styling. |
| Unread badges | `chat-shell` | `sidebar.rs` | Carry forward Module 20 unread badge: 16px copper circle with white count text. Right-aligned in row. Clears on session focus. |

**Deliverable:** Session list renders in sidebar with all session data, status icons, click-to-switch, and context menu.

---

## Stage 3 — Bottom Zone (~1.5 hrs, `chat-shell` agent)

**Files:** `chat-shell/src/sidebar.rs`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| New Session button | `chat-shell` | `sidebar.rs` | Pinned button between session list and bottom zone. "+ New Session" label with Lucide `plus` icon. Copper text, `$accent-copper-subtle` hover background. Sends `ShellToEngine::SessionCreate`. |
| Bottom zone container | `chat-shell` | `sidebar.rs` | `gtk4::Box` (vertical) pinned to bottom. Top border `$border-primary`. Contains Settings and Shut Down rows. |
| Settings row | `chat-shell` | `sidebar.rs` | Lucide `settings` icon (16px) + "Settings" label. `$text-secondary` default, `$text-primary` on hover, `$bg-secondary` hover background. Click action: TBD (placeholder — opens settings panel or chat command). |
| Shut Down row | `chat-shell` | `sidebar.rs` | Lucide `power` icon (16px) + "Shut Down" label. `$text-secondary` default. Hover: `$danger-bg` background, `$danger-text` color. |
| Shut Down confirmation | `chat-shell` | `sidebar.rs` | Clicking "Shut Down" replaces the power row with inline confirmation card: "Shut down Levsha OS?" with [Shut Down] (copper) and [Cancel] (secondary) buttons. 200ms expand animation. |
| Shut Down action | `chat-shell` | `sidebar.rs` | [Shut Down] confirmed → send system command (e.g., `systemctl poweroff`). [Cancel] → collapse confirmation card (150ms). |
| CSS classes | `chat-shell` | `style.css` | Classes: `.sidebar-new-session-btn`, `.sidebar-bottom-zone`, `.sidebar-settings-row`, `.sidebar-power-row`, `.sidebar-power-confirm`. |

**Deliverable:** Bottom zone with New Session, Settings, and Shut Down (with confirmation) fully functional.

---

## Stage 4 — Window Layout Integration (~2 hrs, `chat-shell` agent)

**Files:** `chat-shell/src/window.rs`, `chat-shell/src/keybindings.rs`, `chat-shell/src/status_bar.rs`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Outer layout restructure | `chat-shell` | `window.rs` | Current layout: `Paned(ChatView, ContentPanel)`. New layout: `Box(Horizontal) [ Sidebar, Paned(ChatView, ContentPanel) ]`. Sidebar is a direct child of the outer box, not inside the Paned. |
| Sidebar auto-show logic | `chat-shell` | `window.rs` | On `EngineToShell::SessionListResponse`: if 2+ sessions and user hasn't manually hidden → show sidebar. If 1 session and user hasn't manually shown → hide sidebar. |
| Toggle button outside sidebar | `chat-shell` | `window.rs` | When sidebar is hidden, toggle button must remain visible as an overlay on the chat view (top-left corner). Use `gtk4::Overlay` to layer toggle button on top of the chat area. |
| `Ctrl+B` binding | `chat-shell` | `keybindings.rs` | Add `Ctrl+B` handler: toggle sidebar visibility. Set `sidebar_user_toggled = true` to prevent auto-show/hide overriding user preference. Pass `SessionSidebar` reference to `keybindings::setup()`. |
| `Ctrl+N` sidebar behavior | `chat-shell` | `keybindings.rs` | `Ctrl+N` (new session) also opens sidebar if hidden, unless user previously manually hid it. |
| Status bar session name | `chat-shell` | `status_bar.rs` | When sidebar is visible, hide session name from status bar (redundant). When sidebar is hidden, show session name as before (Module 19). |
| Sidebar ↔ session list overlay | `chat-shell` | `window.rs` | `Ctrl+Shift+S` still opens the overlay (for archive/delete actions). Overlay sits above sidebar + chat. Sidebar and overlay can coexist. |
| Focus management | `chat-shell` | `window.rs` | After sidebar toggle, return focus to input bar. Clicking a session in sidebar → switch session → focus input bar. |

**Deliverable:** Sidebar integrated into the main window layout. Toggle, auto-show, keyboard shortcuts, and focus management all work.

---

## Stage 5 — Animations & Responsive Behavior (~1.5 hrs, `chat-shell` agent)

**Files:** `chat-shell/src/sidebar.rs`, `chat-shell/src/window.rs`, `style.css`

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Slide animation | `chat-shell` | `sidebar.rs` | Revealer transition: `SlideRight` direction, 250ms show / 200ms hide. Chat view width adjusts synchronously (flex layout handles this). |
| Active row animation | `chat-shell` | `sidebar.rs` | When switching sessions, the copper left border slides to the new row (200ms ease-out). Row background crossfades. |
| Context menu animation | `chat-shell` | `sidebar.rs` | Popover appears with scale 0.95→1 + fade in (150ms). Dismiss: fade out (100ms). |
| Shut Down expand/collapse | `chat-shell` | `sidebar.rs` | Confirmation card uses `gtk4::Revealer` with `SlideDown` transition. Expand: 200ms ease-out. Collapse: 150ms ease-in. |
| Responsive overlay mode | `chat-shell` | `window.rs` | When window width < 800px: sidebar becomes absolutely positioned (overlay), with semi-transparent backdrop (`$bg-primary` at 60% opacity). Clicking backdrop closes sidebar. |
| Responsive detection | `chat-shell` | `window.rs` | Connect to `notify::default-width` or `size-allocate` signal on the window. Track whether sidebar should push or overlay based on width threshold. |
| CSS transitions | `chat-shell` | `style.css` | Row hover: 100ms background transition. Toggle button icon crossfade: 150ms. Button hover states: 100ms. |

**Deliverable:** All animations match the design spec. Responsive overlay mode works below 800px.

---

## Stage 6 — Config Persistence & Testing (~1.5 hrs, `engine` + `chat-shell` agents)

**Files:** `engine/src/config.rs`, `chat-shell/src/sidebar.rs`, integration tests

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Config field | `engine` | `engine/src/config.rs` | Add `sidebar_visible: Option<bool>` to config. `None` = auto (show if 2+ sessions). `Some(true/false)` = user override. |
| Config persistence | `engine` | `engine/src/config.rs` | When user toggles sidebar via `Ctrl+B` or toggle button, persist preference. New IPC message: `ShellToEngine::SidebarToggle { visible: bool }`. |
| IPC message | `engine` | `engine/src/types.rs` | Add `SidebarToggle { visible: bool }` variant to `ShellToEngine`. Engine writes to config on receipt. |
| Config read at startup | `chat-shell` | `window.rs` | On startup, read sidebar preference from config. Apply before first render to avoid flash. |
| Unit test: toggle logic | `chat-shell` | tests | Test: 1 session → hidden. 2 sessions → auto-show. User toggles off → stays off. User toggles on → stays on. Config persists. |
| Unit test: context menu | `chat-shell` | tests | Test: right-click shows menu. Rename sends `SessionRename`. Delete sends `SessionDelete` after confirmation. |
| Unit test: responsive | `chat-shell` | tests | Test: width < 800px → overlay mode. Width >= 800px → push mode. Backdrop click closes in overlay mode. |
| Integration test: sidebar + sessions | `qa` | `tests/sidebar_integration.rs` | Create 2 sessions → sidebar auto-shows → click second session → switches → status icons update → close sidebar → persists across restart. |

**Deliverable:** Sidebar preference persists. All tests pass.

---

## Execution Plan

### Parallel Tracks

```
Time →

Track A (chat-shell):  [Stage 1: container] → [Stage 2: session list] → [Stage 3: bottom zone] → [Stage 4: layout] → [Stage 5: animations]
                                                                                                         │
Track B (engine):      ──────────────────────────────────────────────────────────────────────────── [Stage 6: config + IPC]
                                                                                                         │
Track C (qa):          ──────────────────────────────────────────────────────────────────────────────── [Stage 6: tests]
```

Most work is in `chat-shell`. The engine changes are minimal (one new config field + one new IPC message).

### Team Setup

```bash
# Create worktrees
git worktree add ../Levsha.OS-wt-chat-shell main
git worktree add ../Levsha.OS-wt-engine main

# Create team
TeamCreate(team_name="levsha-sidebar")

# Spawn agents
Task(name="chat-shell", prompt="Work in ../Levsha.OS-wt-chat-shell/ ...")
Task(name="engine",     prompt="Work in ../Levsha.OS-wt-engine/ ...")
```

### Merge Order

1. Merge `engine` branch (Stage 6) — config + IPC message
2. Merge `chat-shell` branch (Stages 1-5) — sidebar widget + layout
3. Run integration tests from main worktree
4. Verify all existing tests still pass

---

## Estimated Effort

| Stage | Agent(s) | Estimated Time |
|-------|----------|----------------|
| Stage 1 — Sidebar Container + Toggle | `chat-shell` | 1.5-2 hours |
| Stage 2 — Session List | `chat-shell` | 1.5-2 hours |
| Stage 3 — Bottom Zone | `chat-shell` | 1-1.5 hours |
| Stage 4 — Window Layout Integration | `chat-shell` | 1.5-2 hours |
| Stage 5 — Animations + Responsive | `chat-shell` | 1-1.5 hours |
| Stage 6 — Config + Testing | `engine` + `qa` | 1-1.5 hours |
| **Total (sequential)** | | **~8-11 hours** |
| **Total (parallel, 2 agents)** | | **~6-8 hours** |

---

## Verification Checklist

### Sidebar Display

- [ ] Sidebar renders at 260px width with `$bg-surface` background
- [ ] Right border: 1px `$border-primary`
- [ ] Toggle button visible in top-left at all times (even when sidebar hidden)
- [ ] Toggle button icon: `panel-left` (closed) / `panel-left-close` (open)
- [ ] Toggle button hover: `$bg-tertiary` background

### Session List

- [ ] All sessions listed with name, relative timestamp, status icon
- [ ] Active session: left 3px copper border + `$bg-secondary` background + weight 600 name
- [ ] Inactive sessions: hollow circle icon, weight 500 name
- [ ] Click switches session and updates highlight
- [ ] Session name truncated with ellipsis when too long
- [ ] Scrollable when many sessions

### Status Icons (from Module 20)

- [ ] ○ idle: `$text-tertiary`
- [ ] ◐ processing: `$accent-copper`, pulse animation
- [ ] ✓ complete: `$accent-green`
- [ ] ✗ error: `$accent-copper`
- [ ] ⏳ queued: `$accent-gold`

### Context Menu

- [ ] Right-click shows popover with Rename, Archive, Delete
- [ ] Delete action uses `$danger-text` color
- [ ] Menu appears with scale+fade animation

### Bottom Zone

- [ ] "+ New Session" button with copper text
- [ ] Settings row with `settings` icon
- [ ] Shut Down row with `power` icon
- [ ] Shut Down hover: `$danger-bg` background
- [ ] Shut Down click shows inline confirmation
- [ ] Confirmation [Shut Down] button is copper bg / white text
- [ ] [Cancel] collapses the confirmation

### Toggle Behavior

- [ ] `Ctrl+B` toggles sidebar
- [ ] 1 session: sidebar hidden by default
- [ ] 2+ sessions: sidebar auto-shows (if user hasn't manually hidden)
- [ ] User manual toggle overrides auto behavior
- [ ] Preference persists across restart

### Animations

- [ ] Slide in: 250ms ease-out
- [ ] Slide out: 200ms ease-in
- [ ] Chat view resizes synchronously
- [ ] Active row highlight moves: 200ms ease-out
- [ ] Row hover background: 100ms transition

### Responsive

- [ ] Width < 800px: sidebar overlays (absolute position, shadow)
- [ ] Width < 800px: semi-transparent backdrop behind sidebar
- [ ] Clicking backdrop closes sidebar in overlay mode
- [ ] Width >= 800px: sidebar pushes chat content (flex layout)

### Integration

- [ ] `Ctrl+Shift+S` overlay still works alongside sidebar
- [ ] Status bar session name hidden when sidebar visible
- [ ] Focus returns to input bar after session switch
- [ ] All Module 19 keyboard shortcuts still work
- [ ] All existing tests pass

---

## Risk Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| GTK4 Revealer doesn't synchronize with flex layout resize | Janky animation — chat snaps instead of smooth resize | Use `size-allocate` signal to manually animate chat width. Fallback: use CSS `transition: width 250ms` on the chat container instead of Revealer. |
| Toggle button overlay conflicts with chat content | Button covers important text | Give toggle button a subtle background wash and keep it small (36px). Consider auto-hiding after 3s of inactivity when sidebar is closed. |
| Responsive overlay mode + split-view = three-layer layout | Complex Z-ordering, confusing UX | In overlay mode (< 800px), auto-close sidebar when user opens split-view content panel. |
| Session list performance with many sessions (100+) | Slow rendering, high memory | Use `gtk4::ListBox` with lazy row creation. Only render visible rows. Paginate or virtualize if needed. |
| Sidebar + content panel too wide for small screens | No room for chat | At < 1024px: sidebar 220px (min-width). At < 800px: overlay mode. Content panel respects remaining space. |
| Shut Down button accidentally clicked | Unexpected OS shutdown | Inline confirmation required (not a system dialog). Confirmation card is visually distinct with `$danger-bg`. |

---

## Files Created / Modified

### New Files

| File | Description |
|------|-------------|
| `chat-shell/src/sidebar.rs` | Session sidebar widget: container, session list, toggle button, bottom zone |
| `tests/sidebar_integration.rs` | Integration tests for sidebar behavior |

### Modified Files

| File | Change |
|------|--------|
| `chat-shell/src/window.rs` | Outer layout restructure: `Box(Horizontal)[Sidebar, Paned]`. Sidebar state in `AppState`. Auto-show logic. Overlay toggle button. Responsive mode. |
| `chat-shell/src/session_list.rs` | Extract shared `build_session_row()` function for reuse by sidebar. |
| `chat-shell/src/keybindings.rs` | Add `Ctrl+B` handler. Pass `SessionSidebar` to `setup()`. Update `Ctrl+N` to open sidebar. |
| `chat-shell/src/status_bar.rs` | Conditionally hide session name when sidebar is visible. |
| `chat-shell/src/main.rs` | Import `sidebar` module. |
| `chat-shell/src/app.rs` | Wire sidebar to application lifecycle. |
| `chat-shell/style.css` | New CSS classes for sidebar, toggle button, session rows, bottom zone, responsive mode. |
| `engine/src/types.rs` | Add `SidebarToggle { visible: bool }` to `ShellToEngine`. |
| `engine/src/config.rs` | Add `sidebar_visible: Option<bool>` field. Handle `SidebarToggle` message. |
| `base/overlay/etc/levsha/config.toml` | Add commented-out `# sidebar_visible =` to `[sessions]` section. |
