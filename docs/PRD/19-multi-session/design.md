# 19 — Multi-Session Support: Design Specification

**Module:** Multi-Session Support (L2 + L3)
**Phase:** 2

---

## 1. Session Indicator in Status Bar

The status bar gains a session name segment.

### Status Bar Layout

```
▸ claude-sonnet · connected · Server Setup · 14:32
                               ↑ session name
```

When only one session exists, the session name is hidden to avoid clutter.

### Status Bar Session Styling

| Element | Style |
|---------|-------|
| Session name | IBM Plex Sans 12px, weight 500, `$text-secondary` (#6B5D4F) |
| Separator dot | `$text-tertiary` (#8C7E6E) |
| Clickable area | Entire session name; click opens session list overlay |

---

## 2. Session List Overlay

Activated by `Ctrl+Shift+S` or clicking the session name in the status bar.

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  ┌─ Sessions ──────────────────────────── [+ New] ─┐    │
│  │                                                   │    │
│  │  ● Server Setup               2 min ago          │    │
│  │    "I've installed nginx and configured..."       │    │
│  │                                                   │    │
│  │  ○ Coding Project             1 hour ago          │    │
│  │    "The build succeeded. 0 warnings."             │    │
│  │                                                   │    │
│  │  ○ System Exploration         3 hours ago         │    │
│  │    "Your disk usage is at 42%."                   │    │
│  │                                                   │    │
│  │  ─────────────────────────────────────────────── │    │
│  │  ○ New Session                                    │    │
│  │                                                   │    │
│  └───────────────────────────────────────────────────┘    │
│                                                          │
│  [normal chat content dimmed behind overlay]              │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Overlay Styling

| Element | Style |
|---------|-------|
| Overlay backdrop | `$bg-primary` (#FBF8F3) at 80% opacity (dims chat behind) |
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Card cornerRadius | 12 |
| Card shadow | 0 4px 16px rgba(58,50,40,0.12) |
| Card max-width | 480px, centered |
| Card max-height | 60% of screen height, scrollable |
| Header "Sessions" | IBM Plex Sans 16px, weight 600, `$text-primary` (#3A3228) |
| New button [+ New] | IBM Plex Sans 13px, weight 500, `$accent-copper` (#C67A52) |

### Session Row Styling

| Element | Style |
|---------|-------|
| Active indicator (●) | `$accent-copper` (#C67A52), 8px filled circle |
| Inactive indicator (○) | `$text-tertiary` (#8C7E6E), 8px hollow circle |
| Session name | IBM Plex Sans 14px, weight 500, `$text-primary` |
| Timestamp | IBM Plex Sans 12px, `$text-tertiary`, right-aligned |
| Message preview | IBM Plex Sans 12px, `$text-secondary`, single line, truncated with ellipsis |
| Row padding | 12px vertical, 16px horizontal |
| Row hover | `$bg-secondary` (#F5F1EA) background |
| Row separator | 1px `$border-primary` (#EBE6DC) |
| Active row | Left border 3px `$accent-copper` |
| New Session row | IBM Plex Sans 14px, `$text-tertiary`, italic |

---

## 3. Session Creation

### Via Chat

```
User: new session

Levsha: Created a new session. What would you like to work on?
```

The new session becomes immediately active. The previous session is preserved.

### Via Keyboard

`Ctrl+N` creates a new session and switches to it instantly.

### Via Overlay

Clicking "New Session" at the bottom of the session list creates and switches to a new session.

---

## 4. Session Switching Animation

| Step | Duration | Effect |
|------|----------|--------|
| Current chat fades out | 150ms | Opacity 1→0, slight slide left |
| New session chat fades in | 200ms | Opacity 0→1, slight slide right |
| Status bar session name updates | Instant | Crossfade text |
| Split-view panel | 150ms | If new session has content: open; if not: close |

---

## 5. Auto-Naming

When a session has its default name ("New Session"), the engine auto-generates a name after the first assistant response. The LLM is asked to produce a 2-4 word summary of the conversation topic.

**Implementation:** After the first complete assistant response in a new session, append an internal system message:

```
Generate a 2-4 word title for this conversation.
Reply with ONLY the title, nothing else.
```

This is done as a lightweight side-request (using the local model if available, or a single non-streaming request to cloud with max_tokens=10).

### Auto-Name Display

```
Status bar: ▸ claude-sonnet · connected · New Session · 14:32
                                              ↓ (after first response)
            ▸ claude-sonnet · connected · Nginx Setup · 14:32
```

The name change animates as a crossfade in the status bar (200ms).

---

## 6. Session Archive/Delete

### Archive

```
User: archive this session

Levsha: Archived "Server Setup". You can restore it later
        with "show archived sessions".
```

### Delete Confirmation

```
User: delete this session

Levsha:
  ┌─ ⚠ Delete Session ──────────────────────────────┐
  │                                                    │
  │  Permanently delete "System Exploration"?          │
  │                                                    │
  │  This will remove 47 messages.                     │
  │  This action cannot be undone.                     │
  │                                                    │
  │  ┌──────────┐  ┌──────────┐                       │
  │  │  Delete  │  │  Cancel  │                       │
  │  └──────────┘  └──────────┘                       │
  │                                                    │
  └────────────────────────────────────────────────────┘
```

### Delete Confirmation Styling

| Element | Style |
|---------|-------|
| Container | `$danger-bg` (#FFF5F0), 2px `$accent-copper` border, cornerRadius 8 |
| Warning icon | Lucide `alert-triangle`, `$accent-gold` (#D4A853) |
| Title | IBM Plex Sans 15px, weight 600, `$text-primary` |
| Session name | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Stats | IBM Plex Sans 13px, `$text-secondary` |
| Warning text | IBM Plex Sans 13px, `$accent-copper` |
| Delete button | `$accent-copper` bg, white text |
| Cancel button | `$bg-secondary` bg, `$text-primary` text |

---

## 7. Session Context in Chat

When switching sessions, a subtle separator appears at the top of the chat to indicate the session boundary.

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  ──── Server Setup · 47 messages · last active 2 min ── │
│                                                          │
│  Levsha: I've installed nginx and configured the         │
│  virtual host...                                         │
│                                                          │
```

### Context Banner Styling

| Element | Style |
|---------|-------|
| Banner line | 1px `$border-primary` (#EBE6DC), full width |
| Banner text | IBM Plex Sans 11px, `$text-tertiary` (#8C7E6E), centered |
| Banner background | `$bg-primary` (#FBF8F3), inline with the line |
| Session name in banner | Weight 500 |

---

## 8. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+N | Create new session |
| Ctrl+Tab | Switch to next session |
| Ctrl+Shift+Tab | Switch to previous session |
| Ctrl+Shift+S | Open session list overlay |
| Escape | Close session list overlay (when open) |
| Ctrl+1..9 | Switch to session 1-9 by position |

---

## 9. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in session messages |
| Lucide `message-square` | Session list header icon |
| Lucide `plus` | New session button |
| Lucide `archive` | Archive action |
| Lucide `trash-2` | Delete action |
| Lucide `rotate-ccw` | Restore archived session |
| Lucide `edit-3` | Rename session |
| Lucide `alert-triangle` | Delete confirmation |

---

## 10. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Session list overlay, status bar session name, switching animations. |
| Intelligence Engine (03) | Modified | Per-session context loading, session management tools. |
| Persistence (07) | Modified | Sessions table, session-scoped queries. |
| Split-View (12) | Modified | Session-scoped panel state. |
