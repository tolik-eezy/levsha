# 02 — Chat Shell: Design Specification

**Module:** L3 — Chat Shell
**Scope:** Phase 1 (MVP)

---

## UI Layout

The Chat Shell occupies the full screen. Three regions stack vertically:

```
+--------------------------------------------------------------+
|                                                              |
|   [system] Welcome. I'm your operating system.               |
|            Everything you need -- just ask.                   |
|                                                              |
|   [system] I can manage your packages and tell you about     |
|            your system. More skills are coming soon.          |
|                                                              |
|                                                              |
|   [user]                          how's the system doing? >  |
|                                                              |
|   [system] +-- System Status -------------------------+      |
|            |  CPU      2 vCPUs (x86_64) -- 12% load   |      |
|            |  Memory   847 MB / 2048 MB (41%)          |      |
|            |  Disk     2.1 GB / 8.0 GB used (26%)      |      |
|            |  Uptime   1 hour, 23 minutes               |      |
|            +-------------------------------------------+      |
|                                                              |
|   [system] Everything looks healthy.                          |
|                                                              |
+--------------------------------------------------------------+
|  > _                                                         |
+--------------------------------------------------------------+
|  * claude-sonnet   * connected   * 14:32                     |
+--------------------------------------------------------------+
```

| Region | Height | Behavior |
|--------|--------|----------|
| Message area | Flexible (fills remaining space) | Scrollable, auto-scrolls on new content |
| Input field | Dynamic (1 line to ~40% of screen) | Grows with content, internally scrollable at max |
| Status bar | Fixed (~24-28px) | Always visible, never scrolls |

---

## Component Hierarchy

```
ChatShellApp
  +-- ChatView
  |     +-- MessageList (scrollable container)
  |     |     +-- MessageBubble (repeated)
  |     |     |     +-- MessageHeader (role icon/label, optional timestamp)
  |     |     |     +-- MessageBody
  |     |     |           +-- TextBlock
  |     |     |           +-- CodeBlock (syntax-highlighted)
  |     |     |           +-- TableBlock
  |     |     |           +-- ProgressIndicator
  |     |     |           +-- ErrorBlock
  |     |     +-- StreamingIndicator (during active generation)
  |     +-- ScrollToBottomButton (appears when scrolled up during streaming)
  +-- InputField
  |     +-- TextArea (multi-line, auto-expanding)
  |     +-- SendHint (subtle "Enter to send" / "Shift+Enter for newline")
  +-- StatusBar
  |     +-- BackendIndicator (e.g., "claude-sonnet")
  |     +-- ConnectionStatus ("connected" / "disconnected")
  |     +-- Clock (HH:MM)
  +-- SearchOverlay (hidden by default, activated by Ctrl+F)
        +-- SearchInput
        +-- MatchCount
        +-- NavigationButtons (prev/next)
```

---

## Typography System

Two font families, two purposes:

| Role | Font | Fallback | Usage |
|------|------|----------|-------|
| Proportional | IBM Plex Sans | system-ui, -apple-system, sans-serif | Conversation text, UI labels, status bar |
| Monospace | IBM Plex Mono | JetBrains Mono, ui-monospace, monospace | Code blocks, inline code, command output |

**Scale:**

| Element | Size | Weight | Line Height |
|---------|------|--------|-------------|
| Message body | 15-16px | Regular (400) | 1.55 |
| Code blocks | 13-14px | Regular (400) | 1.5 |
| Status bar | 12px | Medium (500) | 1.0 |
| Message role label | 12px | SemiBold (600) | 1.0 |
| Welcome heading | 20px | SemiBold (600) | 1.4 |
| Input field text | 15-16px | Regular (400) | 1.55 |

---

## Color Palette (Light Theme)

See [theme.design.md](../theme.design.md) for the complete color palette and component styles.

The Chat Shell uses a warm, light theme exclusively. Key tokens referenced in this document:

- **Backgrounds:** warm parchment `bg-primary` (#FBF8F3), deeper parchment `bg-secondary` (#F5F1EA), gentle tan `bg-tertiary` (#EBE6DC)
- **Text:** rich warm brown `text-primary` (#3A3228), warm mid-brown `text-secondary` (#6B5D4F)
- **Accent:** copper `accent-copper` (#C67A52)
- **Semantic:** success (green #62B37B), warning (gold #D4A853), error (copper #C67A52)
- **Code blocks:** warm off-white `bg-code` (#F7F4EE), muted warm syntax colors
- **Syntax highlighting:** copper, green, gold — all defined in theme.design.md

All colors, surfaces, message bubble styles, input field styles, status bar styles, code block styles, destructive command confirmation styles, and syntax highlighting tokens are defined centrally in theme.design.md. No color overrides are permitted in this module.

---

## Animation Specifications

All animations use CSS-style easing. No linear transitions except spinners.

| Animation | Duration | Easing | Trigger |
|-----------|----------|--------|---------|
| Message appear | 200ms | ease-out | New message added |
| Message fade-in opacity | 150ms | ease-out | New message added |
| Scroll momentum | Physics-based | deceleration | User scroll release |
| Typing indicator pulse | 1200ms loop | ease-in-out | Waiting for first token |
| Status change | 300ms | ease-in-out | Connection state change |
| Input field resize | 100ms | ease-out | Line count changes |
| Search overlay open | 150ms | ease-out | Ctrl+F pressed |
| Error message appear | 250ms | ease-out | Error received |
| Scroll-to-bottom button | 200ms | ease-out | User scrolls up during stream |

**Typing indicator:** Three small dots that pulse in sequence. Appears after the user sends a message and before the first token arrives.

---

## Streaming Render Pipeline

Token-by-token rendering is the core UX of the Chat Shell. The pipeline:

```
API SSE stream
    |
    v
Token arrives (engine -> chat shell via IPC)
    |
    v
Append token to current MessageBubble's raw text buffer
    |
    v
Re-parse Markdown on the raw buffer (incremental if possible)
    |
    v
Update the rendered content in the MessageBody widget
    |
    v
Recalculate MessageBubble height
    |
    v
If auto-scroll is active: scroll to bottom
    |
    v
Request frame repaint (compositor schedules at vsync)
```

**Key constraints:**
- Each token append must complete within one frame (16ms at 60fps).
- Markdown re-parse can be deferred: render plain text immediately, apply formatting on pause or completion.
- Code block syntax highlighting is applied on block completion (closing triple backticks), not per-token.
- Auto-scroll tracks the bottom of the message list. If the user has scrolled up (offset from bottom > threshold), auto-scroll disengages.
- A "scroll to bottom" button appears when auto-scroll is disengaged and new content is arriving.

**Stream states:**

| State | Indicator |
|-------|-----------|
| Idle | Input field focused, cursor blinking |
| Waiting | Typing indicator (pulsing dots) |
| Streaming | Text appearing token-by-token, cursor at end |
| Complete | Cursor removed, message finalized |
| Error | Error block displayed inline |

---

## Content Rendering: Markdown Subset

The Chat Shell renders a subset of Markdown within system messages.

| Element | Markdown Syntax | Rendering |
|---------|----------------|-----------|
| Bold | `**text**` | Bold weight |
| Italic | `*text*` | Italic style |
| Inline code | `` `code` `` | Monospace, bg-code (#F7F4EE) background, rounded corners |
| Code block | ```` ```lang ```` | Monospace, syntax-highlighted, bg-code (#F7F4EE), language label |
| Unordered list | `- item` | Bullet point with indent |
| Ordered list | `1. item` | Numbered with indent |
| Table | Pipe-delimited | Bordered table with header row |
| Headings | `## heading` | Larger/bolder text (no `#` -- only `##` through `####`) |
| Horizontal rule | `---` | Subtle line separator |

**Not supported in MVP:** Images, links (displayed as text), blockquotes, nested lists deeper than 2 levels, footnotes, HTML.

---

## Keyboard Shortcut Map

| Context | Shortcut | Action |
|---------|----------|--------|
| Global | Ctrl+F | Open search overlay |
| Global | Escape | Close search overlay / cancel current action |
| Global | Ctrl+C | Cancel streaming response |
| Global | Ctrl+L | Clear visible chat (history preserved) |
| Global | Page Up | Scroll up |
| Global | Page Down | Scroll down |
| Global | Home | Scroll to top of history |
| Global | End | Scroll to bottom |
| Input | Enter | Send message |
| Input | Shift+Enter | Insert newline |
| Input | Up arrow | Recall previous message (when input empty) |
| Input | Down arrow | Recall next message (when navigating history) |
| Input | Ctrl+A | Select all input text |
| Input | Ctrl+V | Paste from clipboard |
| Input | Ctrl+Backspace | Delete word backward |
| Search | Enter | Next match |
| Search | Shift+Enter | Previous match |
| Search | Escape | Close search |

---

## UI State Machine

```
                    +----------+
                    |  BOOTING |
                    +----+-----+
                         |
                    compositor ready
                         |
                    +----v-----+
          +-------->|   IDLE   |<---------+
          |         +----+-----+          |
          |              |                |
          |         user sends msg        |
          |              |                |
          |         +----v-----+          |
          |         | WAITING  |          |
          |         | (typing  |          |
          |         | indicator|          |
          |         +----+-----+          |
          |              |                |
          |         first token           |
          |              |                |
          |         +----v-----+          |
          |         |STREAMING |          |
          |         +----+-----+          |
          |              |                |
          |       stream complete         |
          |              |                |
          +--------------+                |
          |                               |
          |         error at any point    |
          |              |                |
          |         +----v-----+          |
          +---------|  ERROR   |----------+
           retry    +----------+   dismiss
```

**States:**

| State | Input Field | Status Bar | Message Area |
|-------|-------------|------------|--------------|
| BOOTING | Disabled | "starting..." | Empty or splash |
| IDLE | Enabled, focused | "connected" / "disconnected" | Shows history |
| WAITING | Disabled | "connected" | Typing indicator at bottom |
| STREAMING | Disabled (Ctrl+C to cancel) | "connected" | Text appearing token-by-token |
| ERROR | Enabled | "disconnected" or "error" | Error block with retry |

**Transitions:**
- IDLE -> WAITING: user presses Enter with non-empty input
- WAITING -> STREAMING: first token received from API
- WAITING -> ERROR: API timeout or connection failure
- STREAMING -> IDLE: stream complete (stop token received)
- STREAMING -> ERROR: connection lost mid-stream
- STREAMING -> IDLE: user presses Ctrl+C (cancel)
- ERROR -> WAITING: user selects retry
- ERROR -> IDLE: user dismisses error or types new message

---

## Message Layout Details

**User messages:** Right-aligned or full-width with bg-secondary (#F5F1EA) background. No avatar. Role label optional.

**System messages:** Left-aligned or full-width with bg-surface (#FDFBF7) background, 1px bg-tertiary (#EBE6DC) border. Small role label ("Levsha" or system icon).

```
+--------------------------------------------------------------+
|                                                              |
|  Levsha                                                      |
|  +--------------------------------------------------+        |
|  |  Here are the top processes by memory:            |        |
|  |                                                    |        |
|  |  +----------------------------------------------+ |        |
|  |  | PID   NAME              MEM     CPU          | |        |
|  |  | 1142  levsha-chat       312 MB  3.2%         | |        |
|  |  | 892   systemd-journald   48 MB  0.1%         | |        |
|  |  +----------------------------------------------+ |        |
|  +--------------------------------------------------+        |
|                                                              |
|                        +-----------------------------+        |
|                        |  update all packages        |        |
|                        +-----------------------------+        |
|                                                     You      |
|                                                              |
+--------------------------------------------------------------+
```

**Code block layout:**

```
+--------------------------------------------------+
|  rust                                        [^] |
|  ------------------------------------------------|
|  fn main() {                                     |
|      println!("Hello, Levsha");                  |
|  }                                               |
+--------------------------------------------------+
```

The language label sits top-left. A copy affordance `[^]` sits top-right (functional in MVP only via keyboard shortcut, not clickable).

---

## Responsive Behavior

The MVP targets a fixed set of resolutions with the following constraints:

| Resolution | Behavior |
|------------|----------|
| 1280x720 (minimum) | Message max-width: ~85% of screen. Font sizes as specified. |
| 1920x1080 (recommended) | Message max-width: ~70% of screen. Comfortable reading width. |
| Larger | Message max-width caps at ~800px to maintain readability. |

Message content never stretches edge-to-edge. Horizontal padding (48-64px on each side at 1080p) creates a centered column similar to a chat application.
