# 20 — Concurrent Sessions & Background Tasks: Design Specification

**Module:** Concurrent Sessions & Background Tasks (L2 + L3)
**Phase:** 3

---

## 1. Session Sidebar Layout

The sidebar is a persistent left-side panel that replaces overlay-only session navigation. It provides continuous visibility into all sessions and their live status.

### Sidebar Layout

```
┌──────────────┬──────────────────────────────────────────────┐
│              │                                              │
│  Sessions    │                                              │
│              │              Chat View                       │
│  ● Server    │                                              │
│    Setup     │  ◆ Show me the nginx config                  │
│    ▓▓▓▓░░ 65%│                                              │
│              │  Here's the configuration file.              │
│  ◐ Package   │  I've opened it in the preview panel.        │
│    Install   │                                              │
│              │                                              │
│  ○ Coding    │                                              │
│    Project   │                                              │
│              │  ┌───────────────────────┐                   │
│  ✓ System    │  │ _                     │                   │
│    Check  1  │  └───────────────────────┘                   │
│              │                                              │
│  [+ New]     │  ▸ claude-sonnet · connected · 14:32         │
└──────────────┴──────────────────────────────────────────────┘
```

### Sidebar Dimensions

| Property | Value |
|----------|-------|
| Width | 200px (configurable) |
| Min width | 160px |
| Max width | 280px |
| Background | `$bg-surface` (#FDFBF7) |
| Border right | 1px `$border-primary` (#EBE6DC) |
| Padding | 8px 0 |

---

## 2. Sidebar Header

```
┌──────────────┐
│  Sessions    │
├──────────────┤
```

### Header Styling

| Element | Style |
|---------|-------|
| Label | IBM Plex Sans 13px, weight 600, `$text-secondary` (#6B5D4F) |
| Padding | 12px 16px 8px |
| Text transform | Uppercase |
| Letter spacing | 0.04em |

---

## 3. Session Row Design

Each session is a row in the sidebar list.

### Row Layout

```
┌──────────────┐
│ ● Server     │
│   Setup      │
│   ▓▓▓▓░░ 65%│
└──────────────┘
```

### Row Styling

| Element | Style |
|---------|-------|
| Row padding | 8px 12px |
| Row hover | `$bg-secondary` (#F5F1EA) background |
| Active row | `$bg-secondary` background, left border 3px `$accent-copper` (#C67A52) |
| Status icon | 8px, left-aligned, see status icons table |
| Session name | IBM Plex Sans 13px, weight 500, `$text-primary` (#3A3228) |
| Session name (active) | Weight 600 |
| Row gap (icon to name) | 8px |
| Row separator | None (spacing only) |
| Row corner radius | 6px |
| Row margin | 0 4px |

### Status Icons

| Icon | State | Color | Description |
|------|-------|-------|-------------|
| ○ | Idle | `$text-tertiary` (#A89B8C) | 8px hollow circle, session is ready for input |
| ◐ | Processing | `$accent-copper` (#C67A52) | 8px half-filled circle, animated pulse (1.2s) |
| ✓ | Complete (unread) | `$accent-green` (#62B37B) | Checkmark, background task finished |
| ✗ | Error | `$accent-copper` (#C67A52) | X mark, background task failed |
| ⏳ | Queued | `$accent-gold` (#D4A853) | Hourglass, waiting for resource |

### Status Icon Transitions

| Transition | Animation |
|------------|-----------|
| Idle -> Processing | Crossfade 200ms |
| Processing -> Complete | Crossfade 200ms, brief green pulse (300ms) |
| Processing -> Error | Crossfade 200ms, brief copper pulse (300ms) |
| Any -> Idle | Crossfade 200ms |

---

## 4. Progress Indicator

When a session is processing, a progress bar appears below the session name.

### Progress Bar Styling

| Element | Style |
|---------|-------|
| Track | `$bg-tertiary` (#EBE6DC), 3px height, full row width |
| Fill | `$accent-copper` (#C67A52), 3px height |
| Corner radius | 1.5px |
| Margin top | 4px |
| Animation | Fill width transitions smoothly (150ms ease-out) |
| Indeterminate | Sliding gradient left-to-right, 1.5s cycle |

For indeterminate progress (most LLM calls), the bar shows a sliding highlight animation rather than a percentage fill.

---

## 5. Cancel Button

Processing sessions show a cancel button on hover.

### Cancel Button Layout

```
┌──────────────┐
│ ◐ Package  ✕ │
│   Install    │
│   ▓▓▓▓▓▓▓▓  │
└──────────────┘
```

### Cancel Button Styling

| Element | Style |
|---------|-------|
| Icon | Lucide `x`, 12px |
| Color | `$text-tertiary` (#A89B8C) |
| Hover color | `$accent-copper` (#C67A52) |
| Position | Right-aligned in the session name row |
| Visibility | Shown on row hover for processing/queued sessions only |
| Click target | 24x24px |

---

## 6. New Session Button

At the bottom of the sidebar, a button to create a new session.

### New Session Button Styling

| Element | Style |
|---------|-------|
| Label | "+ New" |
| Font | IBM Plex Sans 13px, weight 500 |
| Color | `$accent-copper` (#C67A52) |
| Hover color | `$accent-copper-hover` (#B56B45) |
| Hover background | `$accent-copper-subtle` (#FBF2ED) |
| Padding | 8px 16px |
| Corner radius | 6px |
| Margin | 8px 12px |
| Position | Pinned to bottom of sidebar |

---

## 7. Sidebar Toggle Behavior

| Condition | Behavior |
|-----------|----------|
| 1 session, user has not toggled | Sidebar hidden |
| 2+ sessions created | Sidebar auto-shows (slide in 250ms) |
| Ctrl+B pressed | Toggle sidebar visibility |
| User manually hides | Stays hidden until Ctrl+B, even with 2+ sessions |
| Session deleted, 1 remains | Sidebar auto-hides (slide out 200ms) |

### Sidebar Animation

| Animation | Duration | Easing |
|-----------|----------|--------|
| Slide in (show) | 250ms | ease-out |
| Slide out (hide) | 200ms | ease-in |
| Chat view resize | Matches sidebar animation | Synchronized |

---

## 8. Session Switching While Background Task Runs

When the user switches from a processing session:

| Step | Duration | Effect |
|------|----------|--------|
| Sidebar: active indicator moves | Instant | Active row highlight changes |
| Chat: current session fades out | 150ms | Opacity 1->0, slight slide left |
| Chat: new session fades in | 200ms | Opacity 0->1, slight slide right |
| Status bar: session name updates | Instant | Crossfade text |
| Background session: status icon | Instant | Remains ◐ (processing) in sidebar |

The background session's progress bar continues to animate in the sidebar, giving the user visual feedback that the task is still running.

---

## 9. Unread Badge

When a background session completes a task, its status icon changes to ✓ and an unread count badge appears.

### Badge Styling

| Element | Style |
|---------|-------|
| Shape | Circle, 16px diameter |
| Background | `$accent-copper` (#C67A52) |
| Text | IBM Plex Sans 10px, weight 600, `$text-inverse` (#FAF8F4) |
| Position | Right side of session row, vertically centered |
| Max count display | "9+" for counts above 9 |
| Appear animation | Scale from 0 to 1, 200ms ease-out |

The badge clears when the user switches to that session (marking it as read).

---

## 10. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+B | Toggle session sidebar |
| Ctrl+N | Create new session |
| Ctrl+Tab | Switch to next session |
| Ctrl+Shift+Tab | Switch to previous session |
| Ctrl+Shift+S | Open session list overlay (archive/delete) |
| Ctrl+1..9 | Switch to session 1-9 by position |
| Ctrl+C | Cancel current task in active session |
| Escape | Close session list overlay (when open) |

---

## 11. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in session messages |
| Lucide `x` | Cancel button on processing sessions |
| Lucide `plus` | New session button |
| Lucide `sidebar` | Sidebar toggle hint |
| Lucide `message-square` | Session list overlay header icon |
| Lucide `archive` | Archive action (overlay) |
| Lucide `trash-2` | Delete action (overlay) |

---

## 12. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Multi-Session (19) | Extends | Sidebar supplements the session list overlay; switching animations are reused. |
| Chat Shell (02) | Modified | New sidebar widget, layout changes for sidebar + chat + optional split-view. |
| Intelligence Engine (03) | Modified | Task runner status drives sidebar state icons. |
| Notification System (21) | Consumer | Background completion triggers notifications and badges. |
