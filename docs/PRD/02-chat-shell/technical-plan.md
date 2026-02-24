# 02 — Chat Shell: Technical Plan

**Module:** L3 — Chat Shell
**Language:** Rust
**Scope:** Phase 1 (MVP)

---

## Rust Project Setup

The Chat Shell lives in `chat-shell/` at the repository root. It is a single Rust binary.

```
chat-shell/
  Cargo.toml
  src/
    main.rs           # Entry point, application init
    app.rs            # Top-level ChatShellApp state and event loop
    ui/
      mod.rs
      chat_view.rs    # Message list container, scroll logic
      message.rs      # MessageBubble widget, content rendering
      input.rs        # InputField widget, multi-line editing
      status_bar.rs   # StatusBar widget
      search.rs       # SearchOverlay widget (P1)
    render/
      mod.rs
      markdown.rs     # Markdown parsing and styled text generation
      syntax.rs       # Syntax highlighting for code blocks
      table.rs        # Table layout and rendering
    state/
      mod.rs
      messages.rs     # Message data model, streaming buffer
      session.rs      # UI state machine (idle, waiting, streaming, error)
    ipc/
      mod.rs
      engine.rs       # Communication with intelligence engine (L2)
    db/
      mod.rs
      history.rs      # SQLite read for history display on launch
    theme/
      mod.rs
      colors.rs       # Color palette constants
      typography.rs   # Font definitions and size scale
  assets/
    fonts/            # Bundled fonts (Inter, JetBrains Mono)
```

**Rust edition:** 2021
**MSRV:** 1.75+ (for async trait stability and other recent features)

---

## Toolkit Decision: GTK4 + libadwaita

**Decision:** Use GTK4 with libadwaita bindings for Rust.

| Factor | GTK4 + libadwaita | Iced |
|--------|-------------------|------|
| Wayland support | Native, mature, battle-tested on Fedora | Good but less mature |
| Visual polish out of the box | Excellent -- libadwaita provides polished theming, animations, HiDPI | Requires building from scratch |
| Text rendering | Pango -- excellent subpixel rendering, font shaping | Built-in, less mature |
| Rust bindings | gtk4-rs, well-maintained | Native Rust, first-class |
| Scroll physics | Built-in kinetic scrolling | Manual implementation |
| Accessibility | Built-in ATK support | Limited |
| Custom widget complexity | Moderate -- subclass GtkWidget or compose | Low -- pure Rust composition |
| Fedora integration | First-class (Fedora ships GTK4/libadwaita) | Requires bundling |

**Rationale:** GTK4 + libadwaita gives us the fastest path to a visually polished, production-quality chat GUI on Fedora. libadwaita's theming (custom light theme per theme.design.md), scroll physics, and text rendering are exactly what the Chat Shell needs. While Iced offers a purer Rust experience, it would require building text rendering, scroll physics, and theming from scratch -- work that does not serve the MVP goal.

**Key crates:**

| Crate | Version | Purpose |
|-------|---------|---------|
| gtk4 | 0.9+ | Core UI toolkit |
| libadwaita | 0.7+ | Custom light theme (theme.design.md), adaptive widgets |
| sourceview5 (optional) | 0.9+ | Syntax-highlighted code view (GtkSourceView bindings) |
| pulldown-cmark | 0.11+ | Markdown parsing |
| tree-sitter (alternative) | 0.24+ | Syntax highlighting if not using GtkSourceView |
| syntect | 0.5+ | Alternative syntax highlighting (in-process, no GtkSourceView dep) |
| rusqlite | 0.32+ | SQLite access for history display |
| serde / serde_json | 1.x | Message serialization for IPC |
| tokio | 1.x | Async runtime for IPC and API streaming bridge |

---

## Wayland Integration

The Chat Shell runs as a full-screen Wayland client. GTK4 on Fedora uses the Wayland backend by default.

**Compositor:** The system runs a minimal Wayland compositor (likely Mutter in kiosk mode or cage). The Chat Shell is the only application.

**Full-screen approach:**
1. Set the GTK4 window to fullscreen on realize.
2. Disable all window decorations (CSD disabled).
3. The compositor is configured to launch the Chat Shell as its sole surface.

**Clipboard:** Use GTK4's built-in Wayland clipboard support (GdkClipboard). No X11 fallback.

**Input method:** GTK4's built-in IM support. No special configuration for MVP (English-only).

---

## Widget Implementation Order

Build the Chat Shell incrementally, one widget layer at a time. Each stage produces a runnable application.

**Stage 1 -- Skeleton (Week 1)**
1. Application window: full-screen GTK4 + libadwaita window with custom light theme (theme.design.md).
2. Vertical layout: message area (scrolled window) + input field + status bar.
3. Input field: single-line GtkEntry or GtkTextView, captures Enter to send.
4. Status bar: three static labels (backend, connection, time).
5. Hard-coded messages to test layout.

**Stage 2 -- Message Rendering (Week 2)**
1. MessageBubble widget: frame with role label and body text.
2. Visual distinction between user and system messages (background color, alignment).
3. MessageList: vertical box inside a GtkScrolledWindow.
4. Auto-scroll to bottom on new messages.
5. Plain-text message rendering (no Markdown yet).

**Stage 3 -- Streaming and State Machine (Week 3)**
1. IPC channel to receive tokens from the engine (L2).
2. Streaming buffer: tokens append to the active MessageBubble.
3. Typing indicator widget (three pulsing dots).
4. UI state machine: IDLE -> WAITING -> STREAMING -> IDLE.
5. Ctrl+C to cancel streaming.

**Stage 4 -- Rich Content (Week 4)**
1. Markdown parser integration (pulldown-cmark).
2. Styled text rendering: bold, italic, inline code.
3. Code blocks with syntax highlighting (syntect or GtkSourceView).
4. Table rendering.
5. Progress indicator widget (spinner + status text).

**Stage 5 -- Polish (Week 5)**
1. Message appearance animations (fade/slide-in).
2. Smooth scroll physics tuning.
3. Multi-line input with auto-expand.
4. Input history (Up arrow to recall).
5. Error display styling.
6. Search overlay (Ctrl+F).

**Stage 6 -- Integration (Week 6)**
1. SQLite history load on startup.
2. First-boot welcome message logic.
3. Connection status monitoring (live updates to status bar).
4. Performance profiling and 60fps validation.
5. Resolution testing (1280x720, 1920x1080).

---

## Rendering Pipeline for Streaming Tokens

The streaming render path is performance-critical. Design for minimal work per token.

```
Token received (IPC channel)
    |
    +-- Append to raw text buffer (O(1) string append)
    |
    +-- Check if inside a code fence
    |     |
    |     yes -> append to code buffer, skip re-render
    |     no  -> continue
    |
    +-- Incremental text update:
    |     Append a GtkLabel or Pango text run for the new token
    |     (avoid re-parsing the entire message)
    |
    +-- Trigger size re-allocation on the MessageBubble
    |
    +-- If auto-scroll active: scroll_to_bottom()
    |
    +-- GTK event loop repaints at vsync (60fps)
```

**Optimization strategies:**
- Batch tokens that arrive within the same frame (~16ms window).
- Defer Markdown formatting: render as plain text during streaming, apply formatting on completion or on paragraph breaks.
- Code blocks: accumulate text until the closing fence, then render the entire block with syntax highlighting in one pass.
- Avoid re-creating widgets per token. Use a single GtkTextView per message with a GtkTextBuffer that supports efficient appends.

---

## Syntax Highlighting

**Primary choice:** syntect (Rust-native).

| Factor | syntect | GtkSourceView |
|--------|---------|---------------|
| Integration | Pure Rust, in-process | C library with Rust bindings |
| Theme compatibility | Sublime Text themes (large ecosystem) | GtkSourceView themes |
| Performance | Fast (compiled regex) | Fast (C implementation) |
| Languages | 100+ via Sublime syntax definitions | 100+ via lang specs |
| Custom rendering | We control output (Pango markup or custom) | Widget-based, less flexible |
| Dependency weight | Light | Pulls in GtkSourceView system package |

**Rationale:** syntect gives us full control over how highlighted code is rendered within our MessageBubble. We convert syntect's styled output to Pango markup and display it in a GtkTextView with a monospace font and distinct background.

**MVP language support:** Bash, Python, Rust, JSON, YAML, TOML, JavaScript, C, Go, Markdown, plain text fallback.

---

## SQLite Integration (History Display)

The Chat Shell reads conversation history from SQLite on startup. The engine (L2) is responsible for writing to the database. The Chat Shell is a read-mostly consumer.

**On startup:**
1. Open the SQLite database (path from config or default `~/.local/share/levsha/history.db`).
2. Query the most recent N messages (e.g., 100) ordered by timestamp.
3. Render them as MessageBubbles in the MessageList.
4. Scroll to the bottom.

**During operation:**
- New messages are written by the engine after the user sends them and after responses complete.
- The Chat Shell does not write to SQLite directly (separation of concerns).
- On Ctrl+L (clear visible chat), the Chat Shell clears its in-memory message list but does not delete from SQLite. The engine handles any "clear history" user command.

**Schema (read by Chat Shell):**

```sql
CREATE TABLE messages (
    id INTEGER PRIMARY KEY,
    role TEXT NOT NULL,       -- 'user' | 'assistant' | 'system'
    content TEXT NOT NULL,    -- raw text (Markdown)
    timestamp INTEGER NOT NULL,
    session_id TEXT,
    metadata TEXT             -- JSON blob for progress states, error info
);
```

---

## IPC with Intelligence Engine (L2)

The Chat Shell communicates with the engine via a local Unix domain socket or stdin/stdout pipe.

**Protocol:** Newline-delimited JSON messages.

**Chat Shell -> Engine:**

```json
{"type": "user_message", "content": "install ffmpeg"}
{"type": "cancel"}
{"type": "retry"}
```

**Engine -> Chat Shell:**

```json
{"type": "token", "content": "Let"}
{"type": "token", "content": " me"}
{"type": "token", "content": " search"}
{"type": "stream_start", "message_id": "abc123"}
{"type": "stream_end", "message_id": "abc123"}
{"type": "error", "code": "api_unreachable", "message": "Cannot reach API"}
{"type": "status", "connection": "connected", "backend": "claude-sonnet"}
{"type": "progress", "task": "Installing ffmpeg", "state": "running"}
{"type": "progress", "task": "Installing ffmpeg", "state": "done"}
```

**Implementation:** Tokio async runtime drives the IPC read loop on a background task. Tokens are sent to the GTK main thread via a glib::MainContext channel (thread-safe, integrates with the GTK event loop).

---

## Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|------------|
| Markdown parser | Correct conversion of Markdown to Pango markup or widget tree |
| Syntax highlighting | Correct token classification for supported languages |
| State machine | All transitions, invalid transitions rejected |
| Message model | Streaming buffer append, finalization, serialization |
| History DB | Query correctness, empty database handling |
| IPC parser | JSON message parsing, malformed input handling |

Run with `cargo test`.

### Integration Tests (Chat Shell Internal)

| Test | Method |
|------|--------|
| Startup with empty history | Launch Chat Shell with empty DB, verify welcome message |
| Startup with existing history | Pre-populate DB, verify messages render correctly |
| Streaming simulation | Send mock tokens via IPC, verify they appear in order |
| Error display | Send error via IPC, verify styled error message appears |
| Keyboard shortcuts | Programmatic key events, verify actions (Ctrl+L clears, etc.) |

### Cross-Module Integration Tests

These tests verify Chat Shell is correctly wired to adjacent modules. See `00-system-architecture/integration-checks.md` for full details.

| Check | Seam | What it verifies |
|-------|------|------------------|
| IC-01 | L3 <-> L2 | Chat Shell connects to Engine socket and status bar shows "connected" |
| IC-02 | L3 -> L2 | User message typed in input field arrives at Engine via IPC |
| IC-03 | L2 -> L3 | Streaming tokens from Engine appear incrementally in message bubble |
| IC-04 | L3 -> L2 | Ctrl+C sends cancel signal, Engine stops streaming, UI returns to IDLE |
| IC-05 | L2 -> L3 | Engine error message renders with copper styling per theme.design.md |
| IC-06 | L2 -> L3 | Status bar updates reflect Engine connection/backend changes |
| IC-22 | L2 -> L3 -> L2 | Destructive confirmation prompt renders, user response returns to Engine |
| IC-40 | L3 <- SQLite | Chat Shell loads conversation history from SQLite on startup |
| IC-52 | L3 + systemd | Chat Shell crash triggers systemd restart, reconnects to Engine, reloads history |
| IC-70 | L3 + theme | All UI elements match theme.design.md colors — no GTK defaults leaking |
| IC-72 | Plymouth -> L3 | Smooth visual transition from boot splash to Chat Shell (same parchment bg) |

### Visual Verification

Manual or screenshot-based testing against reference images:

- Light theme renders correctly (warm parchment background, correct contrast per theme.design.md).
- Code blocks have syntax highlighting and distinct background.
- Tables are aligned and readable.
- Message layout at 1280x720 and 1920x1080.
- Animations are smooth (manual 60fps verification with frame counter overlay in debug builds).

### Performance Testing

- Frame rate measurement during scroll and streaming (debug overlay or external tool).
- Token append latency measurement (instrument the streaming pipeline with timing).
- Memory profiling with Valgrind/heaptrack to verify < 350 MB RSS.

---

## Build and Packaging

**Development build:**

```sh
cd chat-shell
cargo build
```

**Release build (for ISO):**

```sh
cargo build --release
strip target/release/levsha-chat
```

**System dependencies (Fedora):**

```sh
dnf install gtk4-devel libadwaita-devel sqlite-devel
```

**Binary name:** `levsha-chat`
**Install path:** `/usr/bin/levsha-chat`
**Desktop entry:** Not needed (no desktop environment). The compositor launches `levsha-chat` directly.

**systemd service:**

```ini
[Unit]
Description=Levsha OS Chat Shell
After=graphical.target

[Service]
ExecStart=/usr/bin/levsha-chat
Restart=on-failure
RestartSec=1

[Install]
WantedBy=graphical.target
```

The compositor (cage or Mutter kiosk) is configured to launch `levsha-chat` as its sole client. If the Chat Shell crashes, systemd restarts it within 1 second.

---

## Dependencies Summary

| Category | Crate / Package | Purpose |
|----------|----------------|---------|
| UI toolkit | gtk4, libadwaita | Widgets, light theme (theme.design.md), scroll, text rendering |
| Markdown | pulldown-cmark | Parse Markdown to events |
| Syntax | syntect | Code block highlighting |
| Database | rusqlite | Read conversation history |
| Async | tokio | IPC read loop |
| Serialization | serde, serde_json | IPC message format |
| Fonts | Inter, JetBrains Mono (bundled) | Typography |

---

## Open Questions

| Question | Decision Needed By |
|----------|--------------------|
| GtkSourceView vs syntect for syntax highlighting? | Stage 4 (rich content) |
| Compositor choice: cage (simple) vs Mutter kiosk mode (heavier but better HiDPI)? | Stage 1 (skeleton) |
| IPC mechanism: Unix socket vs stdin/stdout pipe? | Stage 3 (streaming) |
| Font bundling: embed in binary or install to system fonts? | Stage 1 (skeleton) |
