# 12 — Split-View Content Rendering: Product Requirements

**Module:** Split-View Content Rendering (L3)
**Phase:** 2
**Status:** Draft

---

## 1. Overview

Phase 1 renders all content inline within the chat message flow — text, code blocks, tables. Phase 2 introduces **split-view rendering**: the ability to display rich content (images, file previews, rendered HTML, large code files) in a dedicated panel alongside the chat.

The chat remains the primary interaction surface. The split view is a companion panel that the system opens when the content is better viewed outside the message flow.

---

## 2. Functional Requirements

### 2.1 Split-View Panel (SV-01)

| Field | Value |
|-------|-------|
| **ID** | SV-01 |
| **Priority** | P0 |
| **Requirement** | The Chat Shell supports a side panel for rich content rendering alongside the chat conversation. |

**Details:**

- The split view divides the screen into two regions: chat (left) and content panel (right).
- The content panel opens when the system has content that benefits from a larger view (images, file previews, diffs, long code).
- The user can close the panel via keyboard shortcut (Escape or Ctrl+W) or by asking ("close the preview").
- The panel width is adjustable (drag divider or keyboard shortcut).
- When the panel is closed, the chat returns to full-width.
- Only one content panel at a time (no nesting).

**Acceptance Criteria:**

- [ ] Chat Shell supports a side-by-side layout (chat + content panel).
- [ ] Content panel can be opened, closed, and resized.
- [ ] Keyboard shortcuts control the panel.
- [ ] Chat remains fully functional while the panel is open.

### 2.2 Image Rendering (SV-02)

| Field | Value |
|-------|-------|
| **ID** | SV-02 |
| **Priority** | P0 |
| **Requirement** | Images can be displayed in the split-view panel. |

**Details:**

- When a tool produces an image (e.g., `convert photo.png photo.jpg`), the result can be previewed in the split panel.
- Supported formats: PNG, JPEG, GIF, SVG, WebP.
- Image is scaled to fit the panel while maintaining aspect ratio.
- Basic controls: zoom in/out, fit to panel, actual size.

**Acceptance Criteria:**

- [ ] Images are rendered in the content panel.
- [ ] Common image formats are supported.
- [ ] Zoom controls work.
- [ ] Image scales to fit panel dimensions.

### 2.3 File Preview (SV-03)

| Field | Value |
|-------|-------|
| **ID** | SV-03 |
| **Priority** | P0 |
| **Requirement** | Text files and source code can be previewed in the split-view panel with syntax highlighting. |

**Details:**

- When the user asks to view a file ("show me /etc/levsha/config.toml"), the file content is displayed in the content panel.
- Syntax highlighting based on file extension.
- Line numbers displayed.
- Scrollable for long files.
- Word wrap toggle.

**Acceptance Criteria:**

- [ ] Text files render with syntax highlighting.
- [ ] Line numbers are displayed.
- [ ] Long files are scrollable.
- [ ] Multiple file types are recognized for highlighting.

### 2.4 Diff Rendering (SV-04)

| Field | Value |
|-------|-------|
| **ID** | SV-04 |
| **Priority** | P1 |
| **Requirement** | Git diffs and file comparisons render in a visual diff view in the split panel. |

**Details:**

- Used by the self-improvement system to show proposed changes before deployment.
- Side-by-side or unified diff view.
- Added lines highlighted green, removed lines red.
- Context lines shown around changes.

**Acceptance Criteria:**

- [ ] Diff output renders with color-coded additions/deletions.
- [ ] Side-by-side or unified view is available.
- [ ] Self-improvement diffs display in the panel before user confirmation.

### 2.5 Progress Dashboard (SV-05)

| Field | Value |
|-------|-------|
| **ID** | SV-05 |
| **Priority** | P1 |
| **Requirement** | Long-running tasks (builds, large downloads, batch operations) display a progress dashboard in the split panel. |

**Details:**

- Build progress: compilation steps, current file, errors/warnings count.
- Download progress: file name, percentage, speed.
- Batch operations: completed/total items.

**Acceptance Criteria:**

- [ ] Long-running tasks can display progress in the content panel.
- [ ] Progress updates in real-time.
- [ ] Build output streams into the panel.

---

## 3. L2↔L3 Protocol Extensions

New message types for the split view:

| Direction | Message | Payload |
|-----------|---------|---------|
| L2 → L3 | `ContentOpen` | Content type (image, text, diff, progress), data or file path, suggested panel width. |
| L2 → L3 | `ContentUpdate` | Updated content for the panel (e.g., streaming progress). |
| L2 → L3 | `ContentClose` | Close the content panel. |
| L3 → L2 | `ContentAction` | User action in the panel (e.g., approve diff, cancel build). |

---

## 4. Layout

```
┌──────────────────────────────┬──────────────────────────┐
│         Chat View            │      Content Panel       │
│                              │                          │
│  ◆ Show me the config file   │  /etc/levsha/config.toml │
│                              │  ─────────────────────── │
│  Here's the configuration:   │  [api]                   │
│  I've opened it in the       │  key = "sk-ant-..."      │
│  preview panel.              │  model = "claude-..."    │
│                              │  base_url = "https://..."│
│                              │                          │
│  ┌───────────────────────┐   │                          │
│  │ _                     │   │                          │
│  └───────────────────────┘   │                          │
│                              │                          │
│  ▸ claude-sonnet ▸ connected │              [×] Close   │
└──────────────────────────────┴──────────────────────────┘
```

---

## 5. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Major GUI changes to support split layout. |
| Intelligence Engine (03) | Modified | New message types for content panel. |
| Self-Improvement (10) | Consumer | Diff rendering used for self-improvement confirmation. |
| Filesystem Skill (15) | Consumer | File preview used when viewing files. |

---

## 6. Out of Scope

- Web page rendering (embedded browser).
- Video playback.
- Interactive content editing in the panel.
- Multiple simultaneous content panels.
- Floating/detached panels.
