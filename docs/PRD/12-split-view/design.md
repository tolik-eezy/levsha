# 12 — Split-View Content Rendering: Design Specification

**Module:** Split-View Content Rendering (L3)
**Phase:** 2

---

## 1. Layout Architecture

The split-view transforms the Chat Shell from a single-column layout to a two-panel layout using a GtkPaned widget.

### Closed State (Default)

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│                      Chat View                           │
│                   (full width)                           │
│                                                          │
│                                                          │
├──────────────────────────────────────────────────────────┤
│  > _                                                     │
├──────────────────────────────────────────────────────────┤
│  ▸ claude-sonnet   ▸ connected   ▸ 14:32                │
└──────────────────────────────────────────────────────────┘
```

### Open State

```
┌──────────────────────────┬───┬──────────────────────────┐
│                          │   │                           │
│       Chat View          │ ║ │     Content Panel        │
│                          │ ║ │                           │
│  Messages render here    │ ║ │  File preview, diff,     │
│  at reduced width        │ ║ │  image, or progress      │
│                          │ ║ │  renders here             │
│                          │ ║ │                           │
│                          │ ║ │                           │
├──────────────────────────┤ ║ │                           │
│  > _                     │ ║ │                           │
├──────────────────────────┴───┤                  [×]     │
│  ▸ claude-sonnet   ▸ connected   ▸ 14:32                │
└──────────────────────────────────────────────────────────┘
```

### Layout Dimensions

| Property | Value |
|----------|-------|
| Minimum panel width | 300px |
| Default split ratio | 50/50 |
| Divider width | 4px (1px visual line + grab area) |
| Divider color | `$border-primary` (#EBE6DC) |
| Divider hover color | `$accent-copper` (#C67A52) at 50% opacity |
| Content panel background | `$bg-primary` (#FBF8F3) |
| Content panel padding | 16px |
| Close button position | Top-right corner of content panel |

---

## 2. Content Panel Component Hierarchy

```
ContentPanel
  +-- PanelHeader
  |     +-- ContentTitle (file name, "Diff", "Build Progress", etc.)
  |     +-- ContentControls (zoom, view toggle, etc.)
  |     +-- CloseButton ([×])
  +-- ContentArea (scrollable)
  |     +-- FilePreview (syntax-highlighted source)
  |     |     +-- LineNumbers
  |     |     +-- SourceCode (highlighted)
  |     +-- DiffView
  |     |     +-- DiffHeader (file paths, stats)
  |     |     +-- DiffHunks (added/removed/context lines)
  |     +-- ImageView
  |     |     +-- ImageCanvas (scaled image)
  |     |     +-- ZoomControls
  |     +-- ProgressView
  |           +-- ProgressBar
  |           +-- StepList (completed, active, pending)
  |           +-- OutputLog (streaming text)
  +-- PanelFooter (optional — actions like "Approve", "Cancel")
```

---

## 3. Panel Header Design

```
┌─ /etc/levsha/config.toml ──────────────────── [×] ┐
│                                                     │
```

### Header Styling

| Element | Style |
|---------|-------|
| Background | `$bg-secondary` (#F5F1EA) |
| Height | 36px |
| Padding | 0 12px |
| Title font | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Close button | 24×24, Lucide `x`, `$text-tertiary`, hover: `$text-primary` |
| Border bottom | 1px `$border-primary` (#EBE6DC) |

---

## 4. File Preview Renderer

Syntax-highlighted file view with line numbers.

```
┌─ /etc/levsha/config.toml ──────────────────── [×] ┐
│                                                     │
│   1 │ [api]                                         │
│   2 │ key = "sk-ant-..."                            │
│   3 │ model = "claude-sonnet-4-20250514"            │
│   4 │ base_url = "https://api.anthropic.com"        │
│   5 │ timeout_seconds = 30                          │
│   6 │ max_retries = 3                               │
│   7 │                                               │
│   8 │ [context]                                     │
│   9 │ max_tokens = 200000                           │
│  10 │ response_reserve = 4096                       │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### File Preview Styling

| Element | Style |
|---------|-------|
| Code background | `$bg-code` (#F7F4EE) |
| Line number column width | 48px |
| Line number font | IBM Plex Mono 13px, `$text-tertiary` (#8C7E6E) |
| Line number separator | 1px `$border-primary`, right edge |
| Code font | IBM Plex Mono 13px, `$text-primary` |
| Code padding-left | 12px (after line numbers) |
| Line height | 1.5 (20px) |
| Syntax colors | Per `$syn-*` tokens from theme (keyword: copper, string: green, comment: tertiary, function: gold, number: copper) |

---

## 5. Diff Renderer

Side-by-side or unified diff view for self-improvement and Git operations.

### Unified View

```
┌─ Diff: chat-shell/src/ui/message.rs ──────── [×] ┐
│                                                     │
│  @@ -42,6 +42,8 @@ impl MessageWidget               │
│   42 │  fn configure_font(&self) {                  │
│   43 │      let desc = pango::FontDescription::     │
│  -44 │          from_string("IBM Plex Sans 15");    │
│  +44 │          from_string("IBM Plex Sans 15");    │
│  +45 │      desc.set_hint_style(Full);              │
│  +46 │      desc.set_subpixel(true);                │
│   47 │      self.label.set_font_description(&desc); │
│                                                     │
│  1 file changed, 3 insertions(+), 1 deletion(-)    │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Diff Styling

| Element | Style |
|---------|-------|
| Added line bg | `$accent-green` at 10% opacity |
| Added line marker (+) | `$accent-green` (#62B37B), weight 600 |
| Removed line bg | `$accent-copper` at 10% opacity |
| Removed line marker (-) | `$accent-copper` (#C67A52), weight 600 |
| Context line | Default bg, `$text-primary` |
| Hunk header (@@) | `$bg-secondary` bg, `$text-secondary` text, IBM Plex Mono 12px |
| Stats line | IBM Plex Sans 12px, `$text-tertiary` |

---

## 6. Image Renderer

```
┌─ photo.png (1920×1080) ─── [fit] [1:1] ──── [×] ┐
│                                                     │
│                                                     │
│              ┌─────────────────────┐                │
│              │                     │                │
│              │                     │                │
│              │    (image scaled    │                │
│              │     to fit panel)   │                │
│              │                     │                │
│              │                     │                │
│              └─────────────────────┘                │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Image Viewer Styling

| Element | Style |
|---------|-------|
| Checkerboard bg (for transparency) | Alternating `$bg-primary`/`$bg-secondary`, 16px squares |
| Zoom controls | Pill shape, `$bg-secondary` bg, `$text-secondary` text |
| Image info | IBM Plex Sans 12px, `$text-tertiary` |
| Fit button | Active: `$accent-copper` text |

---

## 7. Progress Renderer

For long-running tasks (builds, downloads, batch operations).

```
┌─ Build: chat-shell ──────────────────────────── [×] ┐
│                                                       │
│  ████████████████░░░░░░░░ 65%                        │
│                                                       │
│  ✓ Compiling pulldown-cmark v0.11.0                  │
│  ✓ Compiling syntect v0.5.0                          │
│  ▶ Compiling levsha-chat v0.2.0                      │
│    └─ src/ui/message.rs                              │
│  ○ Linking                                           │
│  ○ Stripping symbols                                 │
│                                                       │
│  Elapsed: 1m 23s                                     │
│  Warnings: 0  Errors: 0                              │
│                                                       │
└───────────────────────────────────────────────────────┘
```

---

## 8. Animation Specifications

| Animation | Duration | Easing | Trigger |
|-----------|----------|--------|---------|
| Panel open (slide in) | 250ms | ease-out | ContentOpen message received |
| Panel close (slide out) | 200ms | ease-in | Close button or Escape |
| Divider drag | Real-time | linear | User drags divider |
| Divider snap-to-close | 150ms | ease-out | Panel width dragged below minimum |
| Image zoom | 200ms | ease-out | Zoom button click |
| Progress bar fill | 100ms | linear | Progress update |

---

## 9. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+W | Close content panel |
| Escape | Close content panel (when panel focused) |
| Ctrl+Shift+P | Toggle content panel visibility |
| Ctrl+= | Zoom in (image/file preview) |
| Ctrl+- | Zoom out |
| Ctrl+0 | Reset zoom to fit |

---

## 10. Assets Used

| Asset | Usage |
|-------|-------|
| Lucide `x` | Close button |
| Lucide `maximize-2` | Fit to panel |
| Lucide `zoom-in` | Zoom in |
| Lucide `zoom-out` | Zoom out |
| Lucide `file-text` | File preview header icon |
| Lucide `git-branch` | Diff view header icon |
| Lucide `image` | Image view header icon |
| Lucide `activity` | Progress view header icon |
