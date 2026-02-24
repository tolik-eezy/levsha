# 21 — Notification System: Design Specification

**Module:** Notification System (L2 + L3)
**Phase:** 3

---

## 1. Toast Notification Layout

Toasts slide in from the top-right corner, overlaying the chat content.

### Toast Position

```
┌──────────────┬──────────────────────────────────────────────┐
│              │                          ┌────────────────┐  │
│  Sessions    │                          │ ✓ Task Done    │  │
│              │                          │ Package Install │  │
│  ● Server    │                          │ completed.     │  │
│    Setup     │                          │        just now│  │
│              │                          └────────────────┘  │
│  ○ Coding    │                                              │
│    Project   │              Chat View                       │
│              │                                              │
│              │                                              │
│              │                                              │
└──────────────┴──────────────────────────────────────────────┘
```

### Toast Dimensions

| Property | Value |
|----------|-------|
| Width | 300px |
| Min height | 60px |
| Max height | 120px |
| Position | Top-right, 16px from edges |
| Corner radius | 8px |
| Shadow | 0 4px 12px rgba(58, 50, 40, 0.15) |
| Z-index | Above all content (overlay layer) |

---

## 2. Toast Styling

### Toast Component Hierarchy

```
Toast
  +-- SeverityStripe (left edge, 3px wide)
  +-- ContentArea
  |     +-- IconRow
  |     |     +-- SeverityIcon (16px)
  |     |     +-- Title (bold)
  |     |     +-- DismissButton (×)
  |     +-- Body (truncated, 2 lines max)
  |     +-- Timestamp (right-aligned)
  +-- ProgressBar (auto-dismiss countdown, bottom edge)
```

### Toast Base Styling

| Element | Style |
|---------|-------|
| Background | `$bg-surface` (#FDFBF7) |
| Border | 1px `$border-primary` (#EBE6DC) |
| Padding | 12px |
| Gap (icon to content) | 8px |
| Title font | IBM Plex Sans 13px, weight 600, `$text-primary` (#3A3228) |
| Body font | IBM Plex Sans 12px, weight 400, `$text-secondary` (#6B5D4F) |
| Timestamp font | IBM Plex Sans 11px, weight 400, `$text-tertiary` (#A89B8C) |
| Dismiss button | Lucide `x`, 14px, `$text-tertiary`, hover: `$text-primary` |

### Severity-Specific Styling

| Severity | Stripe Color | Icon | Icon Color |
|----------|-------------|------|------------|
| info | `$info` (#8A7F72) | Lucide `info` | `$info` (#8A7F72) |
| warning | `$warning` (#D4A853) | Lucide `alert-triangle` | `$accent-gold` (#D4A853) |
| error | `$error` (#C67A52) | Lucide `alert-circle` | `$accent-copper` (#C67A52) |

### Auto-Dismiss Progress Bar

| Element | Style |
|---------|-------|
| Track | Transparent |
| Fill | Severity color at 30% opacity |
| Height | 2px |
| Position | Bottom edge of toast |
| Animation | Width shrinks from 100% to 0% over dismiss duration |
| Pause on hover | Fill freezes, resumes on mouse leave |

---

## 3. Toast Stacking

Multiple toasts stack vertically from the top-right corner.

```
┌────────────────┐
│ ✗ Build Failed │  <- newest (top)
│ 3 errors in... │
└────────────────┘
       ↕ 8px gap
┌────────────────┐
│ ✓ Packages     │  <- older
│ Installed 5... │
└────────────────┘
       ↕ 8px gap
┌────────────────┐
│ ⚠ Disk Usage   │  <- oldest (bottom)
│ 90% full       │
└────────────────┘
```

### Stacking Rules

| Rule | Value |
|------|-------|
| Max visible toasts | 3 |
| Gap between toasts | 8px |
| Stack direction | Top to bottom (newest on top) |
| Overflow behavior | Oldest toast is dismissed early to make room |
| Entry animation | Slide in from right + fade in, 250ms ease-out |
| Exit animation | Slide out to right + fade out, 200ms ease-in |
| Restack animation | Remaining toasts slide up, 150ms ease-out |

---

## 4. Toast Interaction

| Action | Behavior |
|--------|----------|
| Hover | Pauses auto-dismiss timer, shows dismiss button |
| Click body | Navigate to source session (if applicable), dismiss toast |
| Click dismiss (x) | Dismiss immediately |
| Mouse leave | Resumes auto-dismiss timer from remaining time |

---

## 5. Session Badge Design

Badges appear on session rows in both the sidebar (Module 20) and the session list overlay (Module 19).

### Badge in Sidebar

```
┌──────────────┐
│ ○ Coding     │
│   Project  2 │  <- badge with count
└──────────────┘
```

### Badge Styling

| Element | Style |
|---------|-------|
| Shape | Circle (single digit) or pill (multi-digit) |
| Min size | 16px diameter |
| Background | `$accent-copper` (#C67A52) |
| Text | IBM Plex Sans 10px, weight 600, `$text-inverse` (#FAF8F4) |
| Text alignment | Centered |
| Position | Right side of session row, vertically centered |
| Padding (pill) | 0 4px |
| Max display | "9+" for counts above 9 |
| Appear animation | Scale from 0 to 1, 200ms ease-out with slight bounce |
| Disappear animation | Scale from 1 to 0, 150ms ease-in |
| Increment animation | Brief scale pulse (1.0 -> 1.2 -> 1.0), 200ms |

### Badge in Session List Overlay

Same styling as sidebar badge, positioned at the right edge of the session row.

---

## 6. Notification History Panel

Opened with `Ctrl+Shift+N`, renders in the split-view content panel.

### History Panel Layout

```
┌─ Notifications ──────── [All ▾] [Mark all read] ── [×] ┐
│                                                          │
│  ● ✓ Package Install completed                          │
│    "Installed 5 packages successfully."                  │
│    Server Setup · just now                               │
│                                                          │
│  ● ✗ Build failed with 3 errors                         │
│    "error[E0308]: mismatched types in src/main.rs"       │
│    Coding Project · 5 min ago                            │
│                                                          │
│  ○ ⚠ Disk usage at 90%                                  │
│    "Consider cleaning up /var/log."                      │
│    System · 1 hour ago                                   │
│                                                          │
│  ○ ✓ Skill installed: docker-manager                    │
│    "Docker management skill is now available."           │
│    System · 3 hours ago                                  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### History Panel Header Styling

| Element | Style |
|---------|-------|
| Background | `$bg-secondary` (#F5F1EA) |
| Height | 36px |
| Padding | 0 12px |
| Title font | IBM Plex Mono 13px, weight 500, `$text-primary` (#3A3228) |
| Filter dropdown | IBM Plex Sans 12px, `$text-secondary`, `$bg-surface` background |
| Mark all read button | IBM Plex Sans 12px, weight 500, `$accent-copper` (#C67A52) |
| Close button | 24x24, Lucide `x`, `$text-tertiary`, hover: `$text-primary` |

### History Notification Row Styling

| Element | Style |
|---------|-------|
| Unread indicator (●) | 6px filled circle, `$accent-copper` (#C67A52) |
| Read indicator (○) | 6px hollow circle, `$text-tertiary` (#A89B8C) |
| Severity icon | 14px, severity color (see toast styling) |
| Title | IBM Plex Sans 13px, weight 500 (unread: 600), `$text-primary` (#3A3228) |
| Body | IBM Plex Sans 12px, `$text-secondary` (#6B5D4F), max 2 lines, truncated |
| Source session | IBM Plex Sans 11px, weight 500, `$accent-copper` (#C67A52) |
| Timestamp | IBM Plex Sans 11px, `$text-tertiary` (#A89B8C), after dot separator |
| Row padding | 10px 16px |
| Row hover | `$bg-secondary` (#F5F1EA) background |
| Row separator | 1px `$border-primary` (#EBE6DC) |

### Filter Options

| Filter | Shows |
|--------|-------|
| All | All notification types |
| Tasks | TaskComplete, TaskError, AttentionNeeded |
| System | SystemAlert |
| Skills | SkillInstalled |

---

## 7. Do Not Disturb Indicator

DND status is shown in the status bar.

### Status Bar with DND

```
▸ claude-sonnet · connected · Server Setup · 🔕 · 14:32
                                              ↑ DND indicator
```

### DND Indicator Styling

| Element | Style |
|---------|-------|
| Icon | Lucide `bell-off`, 14px |
| Color (DND off) | Not shown |
| Color (DND on) | `$text-secondary` (#6B5D4F) |
| Click action | Toggle DND mode |
| Tooltip | "Do Not Disturb: On" / "Do Not Disturb: Off" |

### DND Toggle Animation

| Step | Duration | Effect |
|------|----------|--------|
| Icon crossfade | 200ms | `bell` -> `bell-off` or reverse |
| Active toasts | 200ms | Existing toasts fade out simultaneously |

---

## 8. Sound Configuration

| Sound | File | Duration | Description |
|-------|------|----------|-------------|
| Info chime | `info.wav` | ~200ms | Soft, warm single tone |
| Warning chime | `warning.wav` | ~350ms | Two ascending warm tones |
| Error tone | `error.wav` | ~400ms | Low, warm single tone |

### Sound Behavior

| Condition | Behavior |
|-----------|----------|
| DND active | No sound |
| Sound disabled in config | No sound |
| Multiple simultaneous | Only play for the newest notification |
| System muted | Respect system volume (no override) |

---

## 9. Animation Specifications

| Animation | Duration | Easing | Trigger |
|-----------|----------|--------|---------|
| Toast slide in | 250ms | ease-out | Notification received |
| Toast slide out | 200ms | ease-in | Auto-dismiss or manual dismiss |
| Toast restack | 150ms | ease-out | Toast dismissed, remaining shift |
| Badge appear | 200ms | ease-out (bounce) | First unread notification |
| Badge increment pulse | 200ms | ease-in-out | Additional notification |
| Badge disappear | 150ms | ease-in | Session switched to (read) |
| History panel open | 250ms | ease-out | Ctrl+Shift+N or split-view open |
| DND icon toggle | 200ms | ease-out | DND state change |

---

## 10. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+Shift+N | Open/close notification history panel |
| Escape | Dismiss topmost toast (when toasts visible) |

---

## 11. Assets Used

| Asset | Usage |
|-------|-------|
| Lucide `info` | Info severity icon |
| Lucide `alert-triangle` | Warning severity icon |
| Lucide `alert-circle` | Error severity icon |
| Lucide `check-circle` | Task complete icon |
| Lucide `x-circle` | Task error icon |
| Lucide `bell` | Notification indicator (normal) |
| Lucide `bell-off` | DND indicator |
| Lucide `x` | Toast dismiss, history panel close |
| Lucide `filter` | History panel filter |
| `info.wav` | Info notification sound |
| `warning.wav` | Warning notification sound |
| `error.wav` | Error notification sound |

---

## 12. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Concurrent Sessions (20) | Producer | Background task events trigger notifications and badges. |
| Chat Shell (02) | Modified | Toast overlay layer, badge rendering, DND indicator. |
| Split-View (12) | Consumer | Notification history renders in the content panel. |
| Multi-Session (19) | Modified | Badges added to session list overlay rows. |
