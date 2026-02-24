# 26 — Session Sidebar & Navigation: Design Specification

**Module:** Session Sidebar & Navigation (L3)
**Phase:** 3

---

## 1. Overview

A persistent left sidebar that serves as the unified navigation hub for Levsha OS. Replaces the overlay-only session list from Module 19 with an always-accessible panel containing session history, settings, and power controls — similar to the sidebar in ChatGPT or Claude.

The sidebar has two logical zones:
- **Top zone** — Session list (current session + history)
- **Bottom zone** — Settings and Power/Shutdown

A toggle button in the top-left corner shows and hides the sidebar.

---

## 2. Full Layout

```
┌─ ☰ ─────────────┬──────────────────────────────────────────────┐
│                  │                                              │
│  ● Server Setup  │                                              │
│    2 min ago     │              Chat View                       │
│                  │                                              │
│  ○ Coding Proj   │  ◆ Show me the nginx config                  │
│    1 hour ago    │                                              │
│                  │  Here's the configuration file.              │
│  ○ System Check  │  I've opened it in the preview panel.        │
│    3 hours ago   │                                              │
│                  │                                              │
│  ○ Package Mgmt  │                                              │
│    Yesterday     │                                              │
│                  │                                              │
│                  │                                              │
│                  │                                              │
│  ──────────────  │  ┌───────────────────────┐                   │
│  [+ New Session] │  │ _                     │                   │
│                  │  └───────────────────────┘                   │
│  ⚙ Settings      │                                              │
│  ⏻ Shut Down     │  ▸ claude-sonnet · connected · 14:32         │
└──────────────────┴──────────────────────────────────────────────┘
```

---

## 3. Toggle Button

A small icon button pinned to the top-left corner of the screen. Visible at all times — even when the sidebar is hidden.

### Toggle Button Layout

```
When sidebar is hidden:            When sidebar is visible:

┌──────────────────────────┐       ┌────────────┬─────────────────┐
│ ☰                        │       │ ✕           │                 │
│                          │       │             │                 │
│       Chat View          │       │  Sidebar    │   Chat View     │
│                          │       │             │                 │
```

### Toggle Button Styling

| Element | Style |
|---------|-------|
| Icon (closed) | Lucide `panel-left`, 18px, `$text-secondary` (#6B5D4F) |
| Icon (open) | Lucide `panel-left-close`, 18px, `$text-secondary` (#6B5D4F) |
| Button size | 36x36px (meets 44px touch target with padding) |
| Hit area | 44x44px |
| Background | Transparent |
| Hover background | `$bg-tertiary` (#EBE6DC) |
| Hover icon color | `$text-primary` (#3A3228) |
| Corner radius | 8px |
| Position (sidebar hidden) | 8px from left, 8px from top, overlays chat view |
| Position (sidebar open) | 8px from left, 8px from top, inside sidebar header |
| Focus outline | 2px solid `$accent-copper` (#C67A52), 2px offset |
| z-index | Above chat content, below overlays |

---

## 4. Sidebar Container

### Sidebar Dimensions

| Property | Value |
|----------|-------|
| Width | 260px |
| Min width | 220px |
| Max width | 320px |
| Height | Full viewport height |
| Background | `$bg-surface` (#FDFBF7) |
| Border right | 1px `$border-primary` (#EBE6DC) |
| Shadow | 0 1px 4px rgba(58, 50, 40, 0.06) |
| Padding top | 0 (toggle button occupies top area) |
| Padding bottom | 0 |
| Layout | Flex column, space-between (top zone fills, bottom zone pins) |

### Sidebar Structure

```
┌──────────────────┐
│ [✕]              │  ← Toggle button row (48px height)
├──────────────────┤
│                  │
│  Session List    │  ← Top zone: scrollable session list
│  (scrollable)    │
│                  │
│                  │
├──────────────────┤
│ [+ New Session]  │  ← New session button (pinned)
├──────────────────┤
│ ⚙ Settings       │  ← Bottom zone: pinned actions
│ ⏻ Shut Down      │
└──────────────────┘
```

---

## 5. Session List (Top Zone)

The scrollable area shows all sessions, sorted by most recently active. The current session is highlighted.

### Session Row Layout

```
┌──────────────────┐
│ ● Server Setup   │  ← status icon + session name
│   2 min ago      │  ← relative timestamp
│                  │
│ ○ Coding Project │
│   1 hour ago     │
│                  │
│ ○ System Check   │
│   3 hours ago    │
└──────────────────┘
```

### Session Row Styling

| Element | Style |
|---------|-------|
| Row padding | 10px 16px |
| Row hover | `$bg-secondary` (#F5F1EA) background |
| Active row | `$bg-secondary` (#F5F1EA) background, left border 3px `$accent-copper` (#C67A52) |
| Active row (left border padding offset) | Padding-left reduced to 13px to compensate for border |
| Status icon (active) | `$accent-copper` (#C67A52), 8px filled circle (●) |
| Status icon (inactive) | `$text-tertiary` (#A89B8C), 8px hollow circle (○) |
| Session name | IBM Plex Sans 14px, weight 500, `$text-primary` (#3A3228), single line, truncated with ellipsis |
| Session name (active) | Weight 600 |
| Timestamp | IBM Plex Sans 12px, weight 400, `$text-tertiary` (#A89B8C) |
| Icon-to-name gap | 8px |
| Name-to-timestamp gap | 2px (vertical) |
| Row corner radius | 6px |
| Row margin | 0 8px |
| Row gap (between rows) | 2px |

### Session Row Context Menu

Right-clicking a session row shows a context menu:

| Action | Icon | Shortcut |
|--------|------|----------|
| Rename | Lucide `pencil` | — |
| Archive | Lucide `archive` | — |
| Delete | Lucide `trash-2` | — |

### Context Menu Styling

| Element | Style |
|---------|-------|
| Background | `$bg-surface` (#FDFBF7) |
| Border | 1px `$border-primary` (#EBE6DC) |
| Corner radius | 8px |
| Shadow | 0 4px 12px rgba(58, 50, 40, 0.10) |
| Row padding | 8px 12px |
| Row hover | `$bg-secondary` (#F5F1EA) |
| Icon | 14px, `$text-secondary` (#6B5D4F) |
| Label | IBM Plex Sans 13px, weight 400, `$text-primary` (#3A3228) |
| Delete label | IBM Plex Sans 13px, weight 400, `$danger-text` (#7A4A35) |
| Delete icon | 14px, `$danger-text` (#7A4A35) |

---

## 6. New Session Button

Pinned below the session list and above the bottom zone, separated by a divider.

### New Session Button Layout

```
├──────────────────┤
│  + New Session   │
├──────────────────┤
```

### New Session Button Styling

| Element | Style |
|---------|-------|
| Icon | Lucide `plus`, 14px |
| Label | IBM Plex Sans 13px, weight 500, `$accent-copper` (#C67A52) |
| Padding | 10px 16px |
| Hover background | `$accent-copper-subtle` (#FBF2ED) |
| Hover color | `$accent-copper-hover` (#B56B45) |
| Corner radius | 6px |
| Margin | 4px 8px |
| Border top | 1px `$border-primary` (#EBE6DC) (separates from session list) |
| Cursor | pointer |

---

## 7. Bottom Zone (Settings & Power)

Pinned to the bottom of the sidebar. Contains two action rows separated from the session list by a subtle divider.

### Bottom Zone Layout

```
├──────────────────┤
│  ⚙ Settings      │
│  ⏻ Shut Down     │
└──────────────────┘
```

### Bottom Zone Styling

| Element | Style |
|---------|-------|
| Container padding | 4px 0 8px |
| Border top | 1px `$border-primary` (#EBE6DC) |

### Settings Row Styling

| Element | Style |
|---------|-------|
| Icon | Lucide `settings`, 16px, `$text-secondary` (#6B5D4F) |
| Label | IBM Plex Sans 13px, weight 400, `$text-secondary` (#6B5D4F) |
| Padding | 10px 16px |
| Hover background | `$bg-secondary` (#F5F1EA) |
| Hover icon/label color | `$text-primary` (#3A3228) |
| Corner radius | 6px |
| Margin | 0 8px |
| Icon-to-label gap | 10px |

### Power/Shut Down Row Styling

| Element | Style |
|---------|-------|
| Icon | Lucide `power`, 16px, `$text-secondary` (#6B5D4F) |
| Label | IBM Plex Sans 13px, weight 400, `$text-secondary` (#6B5D4F) |
| Padding | 10px 16px |
| Hover background | `$danger-bg` (#FDF5F0) |
| Hover icon/label color | `$danger-text` (#7A4A35) |
| Corner radius | 6px |
| Margin | 0 8px |
| Icon-to-label gap | 10px |

### Shut Down Confirmation

Clicking "Shut Down" shows a confirmation dialog inline (not a modal overlay).

```
├──────────────────────┤
│  ⚙ Settings          │
│                      │
│  ┌────────────────┐  │
│  │ Shut down      │  │
│  │ Levsha OS?     │  │
│  │                │  │
│  │ [Shut Down]    │  │
│  │ [Cancel]       │  │
│  └────────────────┘  │
└──────────────────────┘
```

### Shut Down Confirmation Styling

| Element | Style |
|---------|-------|
| Container background | `$danger-bg` (#FDF5F0) |
| Container border | 1px `$accent-copper` (#C67A52) |
| Container corner radius | 8px |
| Container padding | 12px |
| Container margin | 4px 8px 8px |
| Title | IBM Plex Sans 14px, weight 600, `$text-primary` (#3A3228) |
| Shut Down button | `$accent-copper` (#C67A52) background, `$text-inverse` (#FAF8F4) text, 6px corner radius, 8px 16px padding |
| Cancel button | `$bg-secondary` (#F5F1EA) background, `$text-primary` (#3A3228) text, 6px corner radius, 8px 16px padding |
| Button gap | 8px |

---

## 8. Sidebar Toggle Behavior

| Condition | Behavior |
|-----------|----------|
| First boot / single session | Sidebar hidden; toggle button visible |
| User clicks toggle button | Toggle sidebar visibility |
| `Ctrl+B` pressed | Toggle sidebar visibility |
| 2+ sessions exist, user hasn't manually hidden | Sidebar auto-shows |
| User manually hides via toggle or `Ctrl+B` | Stays hidden until manually reopened (preference persisted) |
| All sessions except one deleted | Sidebar remains in its current state (no auto-hide) |
| Screen width < 800px | Sidebar overlays chat instead of pushing it |

---

## 9. Animations

### Sidebar Slide

| Animation | Duration | Easing |
|-----------|----------|--------|
| Slide in (show) | 250ms | ease-out |
| Slide out (hide) | 200ms | ease-in |
| Chat view resize | Matches sidebar | Synchronized |

The sidebar slides in from the left edge. The chat view width adjusts simultaneously — the content reflows rather than being clipped.

### Toggle Button Icon

| Animation | Duration | Easing |
|-----------|----------|--------|
| Icon crossfade (open/close) | 150ms | ease-out |

### Session Row

| Animation | Duration | Easing |
|-----------|----------|--------|
| Active highlight move | 200ms | ease-out |
| Row hover background | 100ms | ease-out |
| Context menu appear | 150ms | ease-out (scale 0.95 to 1 + fade) |
| Context menu dismiss | 100ms | ease-in |

### Shut Down Confirmation

| Animation | Duration | Easing |
|-----------|----------|--------|
| Confirmation expand | 200ms | ease-out |
| Confirmation collapse | 150ms | ease-in |

---

## 10. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+B | Toggle sidebar visibility |
| Ctrl+N | Create new session (sidebar opens if hidden) |
| Ctrl+Tab | Switch to next session |
| Ctrl+Shift+Tab | Switch to previous session |
| Ctrl+1..9 | Switch to session 1-9 by position |
| Ctrl+Shift+S | Open session list overlay (archive/delete, from Module 19) |
| Escape | Close sidebar (when open and focused) |

---

## 11. Responsive Behavior

| Viewport Width | Behavior |
|----------------|----------|
| < 800px | Sidebar overlays chat (absolute position, shadow), does not push content |
| >= 800px | Sidebar pushes chat content (flexbox layout, chat view shrinks) |
| < 800px + sidebar open | Clicking chat area closes sidebar |
| < 800px + sidebar open | Semi-transparent backdrop `$bg-primary` (#FBF8F3) at 60% opacity behind sidebar |

---

## 12. Interaction with Module 20 (Concurrent Sessions)

Module 20 defines a sidebar for concurrent session status (progress bars, status icons, cancel buttons). Module 26 extends that sidebar into a full navigation hub:

| Module 20 Feature | Module 26 Behavior |
|--------------------|--------------------|
| Status icons (○ ◐ ✓ ✗ ⏳) | Preserved — displayed as session row status icon |
| Progress bar per session | Preserved — rendered below session name in row |
| Cancel button on hover | Preserved — shown on processing sessions |
| Unread badge | Preserved — shown on right side of session row |
| Sidebar header "SESSIONS" | Removed — replaced by toggle button row |
| [+ New] button (bottom) | Moved to dedicated section between list and bottom zone |
| Sidebar auto-show on 2+ sessions | Preserved |
| `Ctrl+B` toggle | Preserved |

The bottom zone (Settings, Shut Down) is new in Module 26 and does not exist in Module 20.

---

## 13. Assets Used

| Asset | Usage |
|-------|-------|
| Lucide `panel-left` | Toggle button icon (sidebar closed) |
| Lucide `panel-left-close` | Toggle button icon (sidebar open) |
| Lucide `plus` | New session button |
| Lucide `settings` | Settings row icon |
| Lucide `power` | Shut down row icon |
| Lucide `pencil` | Rename action in context menu |
| Lucide `archive` | Archive action in context menu |
| Lucide `trash-2` | Delete action in context menu |

---

## 14. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Sidebar widget added to main layout; chat view becomes flex child that shares horizontal space with sidebar. |
| Multi-Session (19) | Extends | Session list overlay remains available via `Ctrl+Shift+S` for archive/delete; sidebar provides persistent navigation. |
| Concurrent Sessions (20) | Extends | Sidebar layout from Module 20 is upgraded with toggle button, bottom zone, and responsive overlay mode. Status icons and progress bars are preserved. |
| Persistence (07) | Consumer | Reads session list, timestamps, and names from sessions table. Sidebar visibility preference persisted. |
