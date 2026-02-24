# 10 — Self-Improvement System: Design Specification

**Module:** Self-Improvement (L2 + L3)
**Phase:** 2

---

## 1. Architecture Overview

The self-improvement system spans L2 (coding agent orchestration, Git versioning, build orchestration) and L3 (agent activity stream in split-view, diff rendering, confirmation dialogs, progress display).

```
┌───────────────────────────────────────────────────────────┐
│                    Chat Shell (L3)                         │
│                                                           │
│  ┌─────────────────────┬─────────────────────────────┐   │
│  │     Chat View        │     Content Panel (SV)       │   │
│  │                      │                              │   │
│  │  User: "Fix the      │  ┌─ Agent Activity ────────┐ │   │
│  │  blurry font"        │  │ 🔍 Reading message.rs   │ │   │
│  │                      │  │ 💭 The issue is in the  │ │   │
│  │  Levsha: I've        │  │    font config...       │ │   │
│  │  started the coding  │  │ ✏️ Editing message.rs   │ │   │
│  │  agent. Watch its    │  │   +desc.set_hint(Full)  │ │   │
│  │  progress →          │  │   +desc.set_subpixel(t) │ │   │
│  │                      │  │ ▶ cargo test            │ │   │
│  │                      │  │   ✓ 12 tests passed     │ │   │
│  │                      │  │ ✅ Agent complete        │ │   │
│  │  ┌────────────────┐  │  └─────────────────────────┘ │   │
│  │  │ ⚠ Deploy this  │  │                              │   │
│  │  │ change?        │  │  Build Progress              │   │
│  │  │ [Apply] [Skip] │  │  ████████░░ 80%              │   │
│  │  └────────────────┘  │  Compiling chat-shell...     │   │
│  └─────────────────────┴─────────────────────────────┘   │
│  ▸ claude-sonnet ▸ connected ▸ 14:32                      │
└───────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Layer | Responsibility |
|-----------|-------|---------------|
| Coding Agent Orchestrator | L2 | Spawn agent subprocess, parse JSONL, manage lifecycle |
| Git Manager | L2 | Commit, diff, log, checkpoint tagging |
| Build Orchestrator | L2 | Cargo build, test, artifact staging |
| Deploy Manager | L2 | Binary swap, service restart, health check |
| Rollback Manager | L2 | Checkpoint creation, restore, cleanup |
| Agent Activity Stream | L3 | Streaming agent output in split-view panel |
| Diff Renderer | L3 | Final diff display in split-view panel (post-agent) |
| Build Progress | L3 | Streaming build output in split-view |
| Deploy Confirm | L3 | Styled confirmation dialog for deploys |

---

## 2. Self-Improvement Flow

The flow is split into two phases: the autonomous coding agent session, and the build/deploy pipeline.

```
User request (natural language)
       │
       ▼
Engine classifies as self-improvement task
       │
       ▼
┌─────────────────────────────────────────────────────┐
│ Phase 1: Coding Agent Session                        │
│                                                      │
│  Engine spawns coding agent (claude / opencode)      │
│  as a subprocess in /usr/src/levsha/                 │
│                                                      │
│  Agent works autonomously:                           │
│    • Reads source files                              │
│    • Searches for relevant code                      │
│    • Edits files to implement the fix/feature        │
│    • Runs tests to validate changes                  │
│                                                      │
│  JSONL output streams to split-view in real time     │
│  User can watch: thinking, reads, edits, commands    │
│                                                      │
│  On completion: agent exits, engine takes over       │
└──────────────┬──────────────────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────┐
│ Phase 2: Deploy                                      │
│                                                      │
│  1. git commit — commit all agent changes            │
│  2. cargo build --release                            │
│  3. cargo test                                       │
│  4. Checkpoint current binaries                      │
│  5. Show final diff + confirmation dialog            │
│  6. User approves → deploy + restart                 │
│  7. Health check → success or automatic rollback     │
└─────────────────────────────────────────────────────┘
```

---

## 3. Agent Activity Stream Design

The agent activity stream renders in the split-view content panel, showing real-time output from the coding agent subprocess. Each JSONL event is parsed and displayed with a categorized prefix.

### Visual Style

```
┌─ Agent Activity ────────────────────────────────────┐
│                                                      │
│  💭 Thinking                                         │
│  The user reports blurry font rendering. I'll check  │
│  the font configuration in the Chat Shell source...  │
│                                                      │
│  🔍 Reading chat-shell/src/ui/message.rs             │
│  ────────────────────────────────────────────────── │
│  42 │  fn configure_font(&self) {                    │
│  43 │      let desc = pango::FontDescription::       │
│  44 │          from_string("IBM Plex Sans 15");      │
│  ...│                                                │
│                                                      │
│  ✏️  Editing chat-shell/src/ui/message.rs             │
│  ────────────────────────────────────────────────── │
│  + desc.set_hint_style(Full);                        │
│  + desc.set_subpixel(true);                          │
│                                                      │
│  ▶ Running: cargo test -p chat-shell                 │
│  ────────────────────────────────────────────────── │
│    ✓ test_font_config ... ok                         │
│    ✓ test_message_render ... ok                      │
│    12 tests passed, 0 failed                         │
│                                                      │
│  ✅ Agent session complete (47s)                      │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### Event Categories and Prefixes

| JSONL Event Type | Prefix | Icon | Description |
|------------------|--------|------|-------------|
| `thinking` / `assistant` | Thinking | 💭 | Agent's reasoning and planning |
| `tool_use: Read` | Reading | 🔍 | File reads — show filename + abbreviated content |
| `tool_use: Edit` / `Write` | Editing | ✏️ | File modifications — show filename + diff snippet |
| `tool_use: Bash` | Running | ▶ | Shell commands — show command + abbreviated output |
| `tool_use: Grep` / `Glob` | Searching | 🔎 | Code search — show pattern + match count |
| `result` (success) | Complete | ✅ | Agent finished successfully |
| `result` (error) | Error | ❌ | Agent encountered an error |
| `result` (timeout) | Timeout | ⏰ | Agent hit the time limit |

### Activity Stream Styling

| Element | Style |
|---------|-------|
| Event prefix icons | 16px, inline with text |
| Category label (Thinking, Reading, etc.) | IBM Plex Sans 13px, weight 600, `$text-secondary` (#6B5D4F) |
| File paths | IBM Plex Mono 13px, `$accent-copper` (#C67A52) |
| Code content | IBM Plex Mono 12px, `$text-primary` (#3A3228) |
| Added lines (+) | `$accent-green` (#62B37B) |
| Command text | IBM Plex Mono 13px, `$text-primary` |
| Command output | IBM Plex Mono 12px, `$text-tertiary` (#8C7E6E) |
| Timestamps / duration | IBM Plex Sans 12px, `$text-tertiary` |
| Section separators | 1px `$border-primary` (#EBE6DC), dashed |
| Panel background | `$bg-surface` (#FDFBF7) |
| Auto-scroll | Panel scrolls to latest event; user scroll-up pauses auto-scroll |

---

## 4. Diff Rendering Design

The diff view renders in the split-view content panel after the coding agent completes. It shows the full `git diff` of all changes made by the agent, giving the user a clear picture before approving deployment.

### Visual Style

```
┌─ Changes (git diff) ───────────────────────────────┐
│  chat-shell/src/ui/message.rs                       │
│  ────────────────────────────────────────────────── │
│                                                      │
│  @@ -42,6 +42,8 @@ impl MessageWidget {             │
│     42 │  fn configure_font(&self) {                 │
│     43 │      let desc = pango::FontDescription::    │
│  -  44 │          from_string("IBM Plex Sans 15");   │
│  +  44 │          from_string("IBM Plex Sans 15");   │
│  +  45 │      desc.set_hint_style(Full);             │
│  +  46 │      desc.set_subpixel(true);               │
│     47 │      self.label.set_font_description(&desc  │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### Diff Color Tokens

| Element | Color | Token |
|---------|-------|-------|
| Added line background | Pale green | `$accent-green` at 10% opacity |
| Added line text | Green | `$accent-green` (#62B37B) |
| Removed line background | Pale copper | `$accent-copper` at 10% opacity |
| Removed line text | Copper | `$accent-copper` (#C67A52) |
| Context lines | Default | `$text-primary` (#3A3228) |
| Line numbers | Muted | `$text-tertiary` (#8C7E6E) |
| File path header | Bold | `$text-primary`, IBM Plex Mono 14px, weight 600 |
| Hunk header (@@ lines) | Muted | `$text-secondary` (#6B5D4F), IBM Plex Mono 13px |
| Panel background | Surface | `$bg-surface` (#FDFBF7) |
| Separator lines | Border | `$border-primary` (#EBE6DC) |

---

## 5. Build Progress Design

Build output streams into the split-view panel during the `deploy_build` phase.

```
┌─ Build Progress ────────────────────────────────────┐
│                                                      │
│  Building chat-shell (release)                       │
│  ────────────────────────────────────────────────── │
│                                                      │
│  ████████████████░░░░ 78%                            │
│                                                      │
│  Compiling levsha-chat v0.2.0                        │
│    ✓ pulldown-cmark 0.11.0                           │
│    ✓ syntect 0.5.0                                   │
│    ▶ levsha-chat 0.2.0 (src/ui/message.rs)           │
│                                                      │
│  Warnings: 0  Errors: 0                              │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### Progress Styling

| Element | Style |
|---------|-------|
| Progress bar fill | `$accent-copper` (#C67A52) |
| Progress bar track | `$bg-secondary` (#F5F1EA) |
| Progress bar height | 6px, cornerRadius 3 |
| Completed item icon (✓) | `$accent-green` (#62B37B) |
| Active item icon (▶) | `$accent-copper` (#C67A52) |
| Error item icon (✗) | `$accent-copper` (#C67A52) |
| Crate names | IBM Plex Mono 13px, `$text-primary` |
| Status summary | IBM Plex Sans 13px, `$text-secondary` |

---

## 6. Deploy Confirmation Dialog

The deploy confirmation is a variant of the destructive command confirmation, styled per the established design system. It is shown after the coding agent finishes and the build succeeds.

```
┌─────────────────────────────────────────────────────┐
│  ⚠ Deploy Changes                                    │
│  ────────────────────────────────────────────────── │
│                                                      │
│  This will restart the Chat Shell with the           │
│  following changes:                                  │
│                                                      │
│  • chat-shell/src/ui/message.rs (+3, -1)             │
│  • 1 file changed, 3 insertions, 1 deletion          │
│                                                      │
│  A checkpoint of the current version has been        │
│  saved. You can roll back at any time.               │
│                                                      │
│  ┌──────────┐  ┌──────────┐                          │
│  │  Apply   │  │  Cancel  │                          │
│  └──────────┘  └──────────┘                          │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### Confirmation Styling

| Element | Style |
|---------|-------|
| Container background | `$danger-bg` (#FFF5F0) |
| Container border | 2px `$accent-copper` (#C67A52) |
| Warning icon (⚠) | Lucide `alert-triangle`, `$accent-gold` (#D4A853) |
| Title | IBM Plex Sans 16px, weight 600, `$text-primary` |
| Body text | IBM Plex Sans 14px, `$text-secondary` |
| File list | IBM Plex Mono 13px, `$text-primary` |
| Apply button | `$accent-copper` background, white text, cornerRadius 6 |
| Cancel button | `$bg-secondary` background, `$text-primary` text, cornerRadius 6 |

---

## 7. Checkpoint Visualization

When the user asks about versions or rollback, a version history card renders in the chat.

```
┌─ Version History ───────────────────────────────────┐
│                                                      │
│  ● Current (v0.2.3) — 2 minutes ago                 │
│  │  "Fix font hinting for HiDPI displays"           │
│  │                                                   │
│  ○ Checkpoint (v0.2.2) — 1 hour ago                 │
│  │  "Add word wrap to file preview"                  │
│  │                                                   │
│  ○ Checkpoint (v0.2.1) — yesterday                  │
│     "Update syntax highlighting colors"              │
│                                                      │
│  ┌──────────────────┐                                │
│  │  Rollback to...  │                                │
│  └──────────────────┘                                │
│                                                      │
└──────────────────────────────────────────────────────┘
```

### Version History Styling

| Element | Style |
|---------|-------|
| Active version dot (●) | `$accent-green` (#62B37B) |
| Checkpoint dot (○) | `$text-tertiary` (#8C7E6E) |
| Version line | 2px `$border-primary` (#EBE6DC) |
| Version label | IBM Plex Sans 14px, weight 600, `$text-primary` |
| Timestamp | IBM Plex Sans 12px, `$text-tertiary` |
| Commit message | IBM Plex Sans 13px, `$text-secondary`, italic |
| Container | `$bg-surface`, 1px `$border-primary` border, cornerRadius 8 |

---

## 8. Health Check Status

After deploy, a health check indicator appears in the chat.

| State | Display |
|-------|---------|
| Checking | Pulsing dots + "Verifying the new version..." |
| Healthy | `$accent-green` checkmark + "Update applied successfully." |
| Failed | `$accent-copper` alert + "Health check failed. Rolling back..." |
| Rolled back | "Reverted to previous version. The change caused: [error details]" |

---

## 9. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in confirmation dialogs |
| Lucide `alert-triangle` | Warning icon in deploy confirmation |
| Lucide `git-commit` | Version history timeline nodes |
| Lucide `check-circle` | Successful health check |
| Lucide `x-circle` | Failed health check |
| Lucide `rotate-ccw` | Rollback action icon |

---

## 10. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Split-View (12) | Required | Agent activity stream, diff rendering, and build progress use the content panel. |
| Chat Shell (02) | Modified | Confirmation dialog, health check status, version history card. |
| Intelligence Engine (03) | Modified | Coding agent orchestration, self-improvement routing. |
