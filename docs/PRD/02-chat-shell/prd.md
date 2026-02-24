# 02 — Chat Shell: Product Requirements

**Module:** L3 — Chat Shell
**Status:** Phase 1 (MVP)
**Priority:** P0 — this is the product

---

## Overview

The Chat Shell is the only graphical surface in Levsha OS. It is a custom, full-screen Wayland application that renders a polished chat interface. There is no desktop, no window manager chrome, no taskbar. The chat occupies the entire screen.

The design mandate is non-negotiable: the Chat Shell must feel like a premium product from first launch. Visual polish, smooth animations, considered typography, and a calm color palette take precedence over feature count. It is better to ship fewer features with a finished feel than more features that look rough.

---

## Requirements

### CS-01 — Full-Screen Chat Layout

The Chat Shell presents a full-screen Wayland-native GUI with three regions:

| Region | Position | Content |
|--------|----------|---------|
| Message history | Top, scrollable | All user and system messages in chronological order |
| Input field | Bottom, fixed | Multi-line text entry area with cursor |
| Status bar | Bottom edge, fixed | Time, connection status, AI backend indicator |

No window decorations, no title bar, no system tray. The chat fills the entire display.

**Acceptance criteria:**
- Application launches as a full-screen Wayland surface with no decorations.
- Message history is scrollable via keyboard (Page Up/Down) and mouse/touchpad.
- Input field is always visible and focused on launch.
- Status bar displays current time (HH:MM), connection state (connected/disconnected), and backend name (e.g., "claude-sonnet").

---

### CS-02 — Visual Polish and Animations

The GUI must meet a high visual bar. This is not optional.

| Element | Specification |
|---------|---------------|
| Message appearance | New messages fade/slide in with a subtle animation (150-250ms ease-out) |
| Scrolling | Smooth, momentum-based scrolling at 60fps |
| Typing indicator | Animated indicator while waiting for LLM first token |
| State transitions | Connection status changes animate smoothly, no hard cuts |
| Typography | Proportional font for conversation text, monospace for code. Generous line height (1.5-1.6x). Comfortable contrast ratios (WCAG AA minimum). |
| Spacing | Consistent padding between messages (12-16px). Clear visual separation between user and system messages. |

**Acceptance criteria:**
- All animations run at 60fps with no dropped frames on the reference VM (2 vCPU, 2GB RAM).
- Message appearance animation is smooth and consistent.
- Scrolling has no visual jank or tearing.
- Typography is legible at 1280x720 and 1920x1080.
- User messages and system messages are visually distinct (alignment, color, or shape).

---

### CS-03 — Light Theme Default

The Chat Shell ships with a single light theme — warm parchment with copper accents (see `theme.design.md`). There is no dark mode in the MVP. There is no theme switcher.

| Element | Color Role |
|---------|------------|
| Background | Warm parchment (bg-primary) |
| Message bubbles (assistant) | Lightest parchment surface (bg-surface) |
| Message bubbles (user) | Slightly deeper parchment (bg-secondary) |
| Primary text | Warm dark brown — never pure black (text-primary) |
| Secondary text | Warm mid-brown (text-secondary) |
| Accent | Copper for interactive elements and highlights (accent-copper) |
| Code blocks | Warm off-white background, syntax-highlighted with copper/gold/green |
| Error states | Copper tone, warm not aggressive (error) |

**Acceptance criteria:**
- All UI elements use the defined dark palette.
- No white flashes on load or transitions.
- Contrast ratios meet WCAG AA (4.5:1 for body text, 3:1 for large text).

---

### CS-04 — Input Field and Keyboard Support

The input field supports multi-line text entry and standard keyboard shortcuts.

| Shortcut | Action |
|----------|--------|
| Enter | Send message (single-line mode) |
| Shift+Enter | New line within message |
| Ctrl+C | Cancel current streaming response |
| Ctrl+L | Clear the visible chat (history preserved in DB) |
| Up arrow | Recall previous user message (when input is empty) |
| Ctrl+A | Select all text in input field |
| Ctrl+V | Paste from clipboard |

The input field expands vertically as the user types multiple lines, up to a maximum height (roughly 40% of screen), then becomes internally scrollable.

**Acceptance criteria:**
- All listed shortcuts function as specified.
- Multi-line input renders correctly with proper line wrapping.
- Input field grows and shrinks dynamically with content.
- Cursor is visible and blinking in the input field on launch.
- Mouse/touch: clicking in the input field focuses it; clicking a message does nothing (MVP).

---

### CS-05 — Rich Content Rendering

System responses render structured content inline within the chat.

| Content Type | Rendering |
|--------------|-----------|
| Styled text | Bold, italic, inline code via Markdown subset |
| Code blocks | Fenced code blocks with syntax highlighting, language label, monospace font, distinct background |
| Tables | Bordered or lined tables with aligned columns |
| Progress indicators | Animated spinner (e.g., "Installing ffmpeg...") with completion state (checkmark or X) |
| Lists | Ordered and unordered lists with proper indentation |
| Links | Displayed as styled text (not clickable in MVP -- no browser) |

**Acceptance criteria:**
- Markdown bold, italic, and inline code render with correct styling.
- Fenced code blocks display with syntax highlighting for common languages (Python, Rust, Bash, JSON, YAML at minimum).
- Tables render with aligned columns and are readable.
- Progress indicators animate during operations and show final success/failure state.
- Content does not overflow or clip incorrectly at any reasonable window size.

---

### CS-06 — Streaming Token Display

LLM output appears token-by-token as it arrives from the API. The rendering must be smooth.

| Aspect | Specification |
|--------|---------------|
| Token arrival | Each token appends to the current message in real time |
| Re-layout | Message bubble and scroll position update without visual jumps |
| Auto-scroll | Chat auto-scrolls to follow new content while streaming; stops if user scrolls up |
| Cursor/caret | A blinking cursor or subtle indicator at the end of the streaming text |
| Completion | When streaming finishes, the cursor disappears and the message is finalized |

**Acceptance criteria:**
- Tokens appear in real time with no perceptible batching delay.
- The chat view auto-scrolls to keep the latest text visible during streaming.
- If the user scrolls up during streaming, auto-scroll pauses; a "scroll to bottom" affordance appears.
- No layout jumps or flicker during token appends.
- Markdown formatting is applied progressively (or on completion if progressive is too complex for MVP).

---

### CS-07 — Error Display

When the AI backend is unreachable or returns an error, the Chat Shell displays a clear, styled error message. There is no fallback to a terminal or TTY.

| Error Type | Display |
|------------|---------|
| Network unreachable | Styled error message with retry option |
| API error (4xx/5xx) | Error description with retry option |
| Timeout | Timeout message with retry option |
| Rate limited | Rate limit message with suggested wait time |

Error messages are visually distinct from normal system messages (different color/border) but remain inline in the chat flow.

**Acceptance criteria:**
- API failure produces a styled, non-technical error message in the chat.
- Each error message includes a "Retry" affordance (keyboard shortcut or inline action).
- The system never drops to a TTY, terminal, or blank screen on API failure.
- The status bar reflects the disconnected state.
- After recovery, the status bar returns to "connected" state.

---

### CS-08 — Chat Search (P1)

Users can search the visible chat history with Ctrl+F.

| Aspect | Specification |
|--------|---------------|
| Activation | Ctrl+F opens a search bar overlay at the top of the chat |
| Matching | Case-insensitive substring search across all visible messages |
| Navigation | Enter/Shift+Enter to jump between matches; matches highlighted |
| Dismissal | Escape closes the search bar |

**Acceptance criteria:**
- Ctrl+F opens the search overlay without disrupting the chat layout.
- Matching text is highlighted in all messages.
- Navigation between matches scrolls to the match and highlights it distinctly.
- Escape closes search and removes highlights.

---

## First Boot Experience

On first boot, the Chat Shell displays a welcome message. There is no setup wizard, no language selection, no account creation.

```
Welcome. I'm your operating system.
Everything you need -- just ask.

I can manage your packages and tell you about your system.
More skills are coming soon.
```

The welcome message is a regular system message in the chat. It is stored in the conversation history.

**Acceptance criteria:**
- First boot shows the welcome message immediately after the GUI renders.
- The input field is focused and ready for typing.
- The status bar shows "connected" (assumes API key is valid and network is up).
- The welcome message persists in history across reboots.

---

## Performance Targets

| Metric | Target |
|--------|--------|
| GUI frame rate | 60fps for scrolling, animations, transitions |
| Streaming render latency | < 16ms per token append (one frame) |
| Input latency | < 50ms from keypress to character on screen |
| Memory (Chat Shell process) | < 350 MB RSS at idle |
| Startup to first frame | < 2 seconds after Wayland compositor is ready |

---

## Out of Scope (MVP)

These features are explicitly excluded from Phase 1:

- Light mode or theme switching
- Clickable links or inline browser
- Image rendering in chat
- File drag-and-drop
- Split-view or side panels
- Multiple chat sessions or tabs
- Customizable fonts or colors
- Voice input or output
- Notification badges
- Context menus or right-click
