# 18 — Code Editor Skill: Design Specification

**Module:** Code Editor Skill
**Phase:** 2

---

## 1. Interaction Design

The code editor extends the text editor with development-oriented capabilities. It uses the split-view panel for file display and adds project navigation, build/run output, and Git information to the chat.

### Project Navigation

```
User: open project /usr/src/levsha

Levsha: Project loaded: /usr/src/levsha (Rust workspace)

  ┌─ Project: levsha ──────────────────────────────────┐
  │                                                     │
  │  Type: Rust workspace (Cargo.toml)                  │
  │  Components: chat-shell, engine                     │
  │  Build: cargo build                                 │
  │  Test: cargo test                                   │
  │                                                     │
  │  .                                                  │
  │  ├── Cargo.toml                                     │
  │  ├── Makefile                                       │
  │  ├── chat-shell/                                    │
  │  │   ├── Cargo.toml                                 │
  │  │   └── src/ (12 files)                            │
  │  ├── engine/                                        │
  │  │   ├── Cargo.toml                                 │
  │  │   └── src/ (18 files)                            │
  │  └── skills/                                        │
  │      └── built-in/ (4 skills)                       │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

### Symbol Search

```
User: find where DestructiveGuard is defined

Levsha: Found definition of DestructiveGuard:

  ┌─ Definition ───────────────────────────────────────┐
  │                                                     │
  │  📄 engine/src/tools/guard.rs:15                    │
  │                                                     │
  │  pub struct DestructiveGuard {                      │
  │      patterns: Vec<Regex>,                          │
  │  }                                                  │
  │                                                     │
  │  References (4):                                    │
  │  • engine/src/tools/mod.rs:42                       │
  │  • engine/src/tools/executor.rs:28                  │
  │  • engine/src/main.rs:67                            │
  │  • engine/src/tools/guard.rs:32 (impl block)       │
  │                                                     │
  └─────────────────────────────────────────────────────┘

  Click any reference to open it in the editor.
```

---

## 2. Build & Test Output

Build and test output renders in the split-view panel.

### Build Output

```
┌─ Build: cargo build --release ──────────────────── [×] ┐
│                                                          │
│  ████████████████████████░░░░ 85%                       │
│                                                          │
│  Compiling levsha-chat v0.2.0                           │
│    ✓ pulldown-cmark 0.11.0           (0.8s)             │
│    ✓ syntect 0.5.0                   (2.1s)             │
│    ✓ gtk4 0.9.0                      (4.3s)             │
│    ▶ levsha-chat 0.2.0               (compiling...)     │
│                                                          │
│  ─────────────────────────────────────────────────────  │
│  Elapsed: 12.4s                                         │
│  Warnings: 2  Errors: 0                                  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Build Error

```
┌─ Build: cargo build ─────────────── ✗ Failed ──── [×] ┐
│                                                          │
│  error[E0308]: mismatched types                         │
│    --> chat-shell/src/ui/message.rs:42:12                │
│     │                                                    │
│  42 │     let x: u32 = "hello";                          │
│     │            ^^^   ─────── expected `u32`,           │
│     │                          found `&str`              │
│                                                          │
│  ─────────────────────────────────────────────────────  │
│  1 error, 0 warnings                                     │
│                                                          │
│  ┌──────────────────┐                                    │
│  │  Fix this error  │                                    │
│  └──────────────────┘                                    │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Test Results

```
┌─ Tests: cargo test ──────────────────────────────── [×] ┐
│                                                          │
│  Running engine tests...                                 │
│                                                          │
│  ✓ test_sse_parser_basic               (0.01s)          │
│  ✓ test_sse_parser_partial_chunk       (0.01s)          │
│  ✓ test_destructive_guard_rm           (0.00s)          │
│  ✗ test_token_counting_accuracy        (0.02s)          │
│    assertion failed: count == 42                         │
│    left: 40, right: 42                                   │
│  ✓ test_skill_manifest_parsing         (0.01s)          │
│                                                          │
│  ─────────────────────────────────────────────────────  │
│  4 passed, 1 failed                                      │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## 3. Git Integration Display

### Git Status

```
User: git status

Levsha:
  ┌─ Git Status ───────────────────────────────────────┐
  │                                                     │
  │  Branch: main (up to date)                          │
  │                                                     │
  │  Modified:                                          │
  │    M  chat-shell/src/ui/message.rs                  │
  │    M  engine/src/config.rs                          │
  │                                                     │
  │  Untracked:                                         │
  │    ?  skills/built-in/filesystem/                    │
  │                                                     │
  │  2 modified, 1 untracked                            │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

### Git Diff (in split-view panel)

```
┌─ Diff: 2 files changed ─────────────────────────── [×] ┐
│                                                          │
│  chat-shell/src/ui/message.rs                           │
│  @@ -42,6 +42,8 @@ impl MessageWidget                  │
│     fn configure_font(&self) {                          │
│  -      let size = 15;                                   │
│  +      let size = 16;                                   │
│  +      let hinting = true;                              │
│                                                          │
│  engine/src/config.rs                                    │
│  @@ -12,1 +12,1 @@                                     │
│  -  pub timeout_seconds: u64,     // 30                  │
│  +  pub timeout_seconds: u64,     // 60                  │
│                                                          │
│  2 files, +3 -2                                          │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Git Log

```
User: show recent commits

Levsha:
  ┌─ Git Log ──────────────────────────────────────────┐
  │                                                     │
  │  b688be5  phase 2                      2 hours ago  │
  │  626eeb6  phase 1                      3 hours ago  │
  │  aad7e07  init                         yesterday    │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

---

## 4. Styling Reference

### Project Card

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Project type | IBM Plex Sans 13px, `$text-secondary` |
| Tree lines | IBM Plex Mono 13px, `$border-primary` |
| Directory names | IBM Plex Mono 13px, weight 600, `$accent-gold` (#D4A853) |
| File names | IBM Plex Mono 13px, `$text-primary` |
| File count | IBM Plex Sans 12px, `$text-tertiary` |

### Build Output

| Element | Style |
|---------|-------|
| Progress bar fill | `$accent-copper` (#C67A52) |
| Progress bar track | `$bg-secondary` (#F5F1EA) |
| Completed (✓) | `$accent-green` (#62B37B) |
| Active (▶) | `$accent-copper` (#C67A52) |
| Failed (✗) | `$accent-copper` (#C67A52) |
| Crate names | IBM Plex Mono 13px, `$text-primary` |
| Timing | IBM Plex Mono 12px, `$text-tertiary` |
| Error text | IBM Plex Mono 13px, `$accent-copper` |
| Error line highlight | `$accent-copper` at 10% opacity |
| "Fix this error" button | `$accent-copper` bg, white text, cornerRadius 6 |

### Test Results

| Element | Style |
|---------|-------|
| Pass (✓) | `$accent-green` (#62B37B) |
| Fail (✗) | `$accent-copper` (#C67A52) |
| Test name | IBM Plex Mono 13px, `$text-primary` |
| Duration | IBM Plex Mono 11px, `$text-tertiary` |
| Failure message | IBM Plex Mono 12px, `$accent-copper` |
| Summary | IBM Plex Sans 13px, weight 500 |

### Git Status Card

| Element | Style |
|---------|-------|
| Branch name | IBM Plex Mono 13px, weight 600, `$accent-copper` |
| Modified (M) | `$accent-gold` (#D4A853) |
| Added (A) | `$accent-green` (#62B37B) |
| Deleted (D) | `$accent-copper` (#C67A52) |
| Untracked (?) | `$text-tertiary` (#8C7E6E) |
| File paths | IBM Plex Mono 13px, `$text-primary` |
| Summary | IBM Plex Sans 12px, `$text-secondary` |

### Git Log

| Element | Style |
|---------|-------|
| Commit hash | IBM Plex Mono 12px, `$accent-copper` |
| Commit message | IBM Plex Sans 13px, `$text-primary` |
| Timestamp | IBM Plex Sans 12px, `$text-tertiary`, right-aligned |
| Row separator | 1px `$border-primary` |

---

## 5. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in code editor messages |
| Lucide `folder-code` | Project header icon |
| Lucide `search` | Symbol search |
| Lucide `file-code` | Source file reference |
| Lucide `hammer` | Build action |
| Lucide `play` | Run action |
| Lucide `test-tube` | Test action |
| Lucide `git-branch` | Git branch display |
| Lucide `git-commit` | Git commit log |
| Lucide `check-circle` | Test pass / build success |
| Lucide `x-circle` | Test fail / build error |
| Lucide `terminal` | Build output panel |

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Text Editor Skill (17) | Upstream | Inherits all text editing capabilities. |
| Split-View (12) | Required | Build output, diffs, file preview use the content panel. |
| Self-Improvement (10) | Sibling | Shares Git tools and source editing tools. |
| Intelligence Engine (03) | Modified | Project detection, build tool dispatch. |
