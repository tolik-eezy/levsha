# ADR-003: GTK4 + libadwaita for Chat Shell GUI

**Status:** Accepted
**Date:** 2025-01

---

## Context

The Chat Shell (L3) is the only visual interface of Levsha OS. It must:

- Run full-screen on Wayland with no window decorations
- Render rich text: markdown (bold, italic, inline code), tables, ordered/unordered lists, fenced code blocks with syntax highlighting
- Stream text smoothly from the LLM (token by token)
- Provide a polished, visually appealing experience -- the chat is literally the entire OS
- Have good Rust bindings (the project is Rust-only)
- Work well on Fedora with excellent font rendering out of the box

## Options Considered

### 1. GTK4 + libadwaita (gtk4-rs + libadwaita-rs)

The GNOME UI toolkit with its adaptive/HIG companion library.

**Pros:**
- First-class Fedora citizen -- GTK4 and libadwaita are the default toolkit
- Excellent font rendering (fontconfig + FreeType + HarfBuzz, all tuned on Fedora)
- Native Wayland support -- no X11 compatibility layer needed
- libadwaita provides dark mode, adaptive layouts, and polished default styles
- Mature Rust bindings (gtk4-rs 0.9, libadwaita-rs 0.7)
- CSS-based theming for custom look (parchment/copper palette)

**Cons:**
- Linux-only (acceptable -- this is a Linux OS)
- GTK4 CSS is more limited than web CSS (no flexbox, limited grid)
- Callback-heavy API requires `Rc<RefCell<>>` patterns in Rust
- Larger dependency tree than pure GPU-rendered alternatives

### 2. Iced

A cross-platform Rust GUI library inspired by Elm.

**Pros:**
- Pure Rust, no C dependencies
- Elm architecture -- clean state management
- GPU-rendered (wgpu)
- Cross-platform

**Cons:**
- Immature compared to GTK4 -- breaking API changes between versions
- Font rendering quality on Linux does not match GTK4/fontconfig
- No libadwaita integration -- would need to reimplement all styling
- Smaller ecosystem, fewer widgets
- Text layout and selection less polished for a chat application

### 3. egui

An immediate-mode GUI library for Rust.

**Pros:**
- Simple API, easy to get started
- GPU-rendered
- Works on all platforms

**Cons:**
- Immediate-mode is wrong paradigm for a chat UI (retained-mode is better for scrollable lists)
- Font rendering quality significantly below GTK4
- No native Wayland integration (relies on winit or eframe)
- Text selection, accessibility, and input method support are limited
- Not suitable for a polished, production-quality OS interface

### 4. Tauri / webview

Render the chat UI as a web page in an embedded browser.

**Pros:**
- Full CSS/HTML for layout -- maximum styling flexibility
- Large ecosystem (React, Vue, etc.)
- Easy to iterate on visual design

**Cons:**
- Pulls in a full browser engine (WebKitGTK ~100+ MB)
- Memory overhead and startup latency
- Security surface area (web engine vulnerabilities)
- Philosophical mismatch -- building a Linux OS and then rendering its UI in a browser
- IPC between Rust backend and web frontend adds complexity

## Decision

**GTK4 + libadwaita.** For a Fedora-based OS where visual polish is a core requirement, GTK4 is the natural choice. It provides the best font rendering, the most polished default appearance, and first-class Wayland support without any compatibility layers.

The custom theme uses GTK4 CSS to achieve the warm parchment + copper look:
- Background: `#FBF8F3` (warm parchment)
- Accent: `#C67A52` (copper)
- Text: `#3B2E25` (dark brown)

## Consequences

- **Linux-only:** The chat shell cannot be compiled for macOS or Windows. This is a non-issue for an operating system.
- **Fedora dependency:** The binary links against system GTK4/libadwaita libraries. All Rust compilation must happen inside a Fedora container (handled by the Makefile and `infra/Containerfile`).
- **Rich text rendering:** Custom `MessageWidget` implementation renders markdown elements (bold, italic, code, tables, lists, code blocks) using GTK4 `Label` and `TextView` widgets. Syntax highlighting is hand-rolled for Python, Rust, Bash, JavaScript, Go, JSON, and YAML.
- **Streaming performance:** Token-by-token text appending works well with GTK4's text buffer. The `ChatView` auto-scrolls to keep the latest content visible.
- **Theming:** All visual customization is done through GTK4 CSS loaded at startup. No runtime theme switching in MVP.
