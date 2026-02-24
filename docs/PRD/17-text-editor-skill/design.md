# 17 — Text Editor Skill: Design Specification

**Module:** Text Editor Skill
**Phase:** 2

---

## 1. Interaction Design

The text editor renders the file being edited in the split-view panel while the user gives editing instructions in the chat. The chat + panel combination creates a two-pane editing experience.

### Editor Layout

```
┌──────────────────────────┬───────────────────────────────┐
│       Chat View          │       Content Panel            │
│                          │                                │
│  User: edit config.toml  │  /etc/levsha/config.toml       │
│                          │  ───────────────────────────── │
│  Levsha: I've opened     │   1 │ [api]                    │
│  the file. What would    │   2 │ key = "sk-ant-•••"       │
│  you like to change?     │   3 │ model = "claude-sonnet"  │
│                          │   4 │ base_url = "https://..." │
│  User: change the        │   5 │ timeout_seconds = 30     │
│  timeout to 60           │   6 │ max_retries = 3          │
│                          │   7 │                          │
│  Levsha: Done. I've      │   8 │ [context]                │
│  changed timeout_seconds │   9 │ max_tokens = 200000      │
│  from 30 to 60.          │  10 │ response_reserve = 4096  │
│                          │                                │
│                          │  ● Modified (unsaved)          │
│  > _                     │  [Save] [Undo] [Close]         │
│                          │                                │
│  ▸ claude-sonnet ▸ conn  │                                │
└──────────────────────────┴───────────────────────────────┘
```

---

## 2. Panel Header

```
┌─ /etc/levsha/config.toml ─── toml ───── ● Modified ── [×] ┐
```

### Header Elements

| Element | Style |
|---------|-------|
| File path | IBM Plex Mono 13px, weight 500, `$text-primary` (#3A3228) |
| Language label | IBM Plex Sans 11px, weight 500, `$text-tertiary` (#8C7E6E) |
| Modified indicator (●) | `$accent-gold` (#D4A853), 8px circle, appears when unsaved changes exist |
| "Modified" text | IBM Plex Sans 11px, `$accent-gold` |
| Close button | Lucide `x`, 24×24, `$text-tertiary` |

---

## 3. Editor Panel Content

### Line Display

| Element | Style |
|---------|-------|
| Line number column | 48px width, right-aligned |
| Line numbers | IBM Plex Mono 13px, `$text-tertiary` (#8C7E6E) |
| Line number separator | 1px `$border-primary` (#EBE6DC), right edge of number column |
| Code content | IBM Plex Mono 13px, `$text-primary` (#3A3228) |
| Code background | `$bg-code` (#F7F4EE) |
| Line height | 20px (1.5 line-height) |
| Left padding (content) | 12px after line numbers |

### Modified Line Highlighting

When a line is modified by an edit operation, it briefly highlights to draw attention.

| Element | Style |
|---------|-------|
| Modified line background | `$accent-gold` at 15% opacity, fades over 2s |
| Inserted line background | `$accent-green` at 15% opacity, fades over 2s |
| Deleted line | Removed from display, line numbers renumber |
| Current cursor line | `$bg-secondary` (#F5F1EA) background |

---

## 4. Editor Footer (Action Bar)

The editor panel has a footer with action buttons.

```
│  ● Modified (unsaved)                                      │
│  [Save]  [Undo]  [Redo]  [Diff]  [Close]                  │
```

### Footer Styling

| Element | Style |
|---------|-------|
| Footer background | `$bg-secondary` (#F5F1EA) |
| Footer height | 40px |
| Footer padding | 8px 12px |
| Status text | IBM Plex Sans 12px, `$text-secondary` |
| Modified indicator | `$accent-gold` (#D4A853) dot + "Modified (unsaved)" |
| Saved indicator | `$accent-green` (#62B37B) dot + "Saved" |
| Button style | IBM Plex Sans 12px, weight 500, `$bg-surface` bg, 1px `$border-primary`, cornerRadius 4 |
| Button hover | `$accent-copper` (#C67A52) text |
| Save button (when modified) | `$accent-copper` bg, white text |

---

## 5. Edit Operation Feedback

When the LLM performs an edit, feedback appears both in the chat and in the editor panel.

### In Chat

```
Levsha: Done. Changed line 5:
        timeout_seconds = 30 → timeout_seconds = 60
```

### In Panel

Line 5 briefly highlights with `$accent-gold` background, then fades back to normal.

### Multi-Line Edit

```
Levsha: Done. Replaced lines 8-10:
        - [context]
        - max_tokens = 200000
        + [context]
        + max_tokens = 150000
        + chars_per_token = 3
```

### Undo Feedback

```
Levsha: Undone. Reverted the last change
        (timeout_seconds = 60 → 30)
```

---

## 6. Unsaved Changes Warning

When the user tries to close the editor with unsaved changes:

```
  ┌─ ⚠ Unsaved Changes ────────────────────────────┐
  │                                                   │
  │  /etc/levsha/config.toml has unsaved changes.    │
  │                                                   │
  │  ┌────────┐  ┌─────────────────┐  ┌──────────┐ │
  │  │  Save  │  │  Discard & Close │  │  Cancel  │ │
  │  └────────┘  └─────────────────┘  └──────────┘ │
  │                                                   │
  └───────────────────────────────────────────────────┘
```

### Warning Styling

| Element | Style |
|---------|-------|
| Container | `$danger-bg` (#FFF5F0), 1px `$accent-copper` border |
| Warning icon | Lucide `alert-triangle`, `$accent-gold` |
| Text | IBM Plex Sans 14px, `$text-primary` |
| Save button | `$accent-copper` bg, white text |
| Discard button | `$bg-secondary` bg, `$accent-copper` text |
| Cancel button | `$bg-secondary` bg, `$text-primary` text |

---

## 7. Diff View (Unsaved Changes)

When the user asks to see changes or uses the "Diff" button:

```
┌─ Changes: config.toml ──────────────────────────────────┐
│                                                           │
│  @@ -5,1 +5,1 @@                                        │
│  -  timeout_seconds = 30                                  │
│  +  timeout_seconds = 60                                  │
│                                                           │
│  1 change, 1 line modified                                │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

Uses the same diff styling as the split-view module (12).

---

## 8. Keyboard Shortcuts

| Shortcut | Action | Context |
|----------|--------|---------|
| Ctrl+S | Save file | Editor panel open |
| Ctrl+Z | Undo last edit | Editor panel open |
| Ctrl+Shift+Z | Redo | Editor panel open |
| Ctrl+W | Close editor | Editor panel open |
| Escape | Close editor (prompts if unsaved) | Editor panel open |

---

## 9. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in edit confirmation messages |
| Lucide `file-text` | Editor panel header icon |
| Lucide `save` | Save action |
| Lucide `undo` | Undo action |
| Lucide `redo` | Redo action |
| Lucide `git-branch` | Diff action |
| Lucide `x` | Close action |
| Lucide `alert-triangle` | Unsaved changes warning |
