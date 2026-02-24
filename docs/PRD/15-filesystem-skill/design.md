# 15 — Filesystem Skill: Design Specification

**Module:** Filesystem Skill
**Phase:** 2

---

## 1. Interaction Design

The filesystem skill formats all output as styled chat messages. Large content (file listings, file previews) can optionally render in the split-view panel.

### Directory Listing

```
User: what's in /etc/levsha/?

Levsha:
  ┌─ /etc/levsha/ ─────────────────────────────────────┐
  │                                                      │
  │  📁 skills/              drwxr-xr-x   4.0 KB        │
  │  📄 config.toml          -rw-------   1.2 KB        │
  │  📄 destructive.toml     -rw-r--r--    842 B        │
  │                                                      │
  │  3 items, 6.0 KB total                               │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### Tree View

```
User: show me the project structure

Levsha:
  ┌─ /usr/src/levsha/ ─────────────────────────────────┐
  │                                                      │
  │  .                                                   │
  │  ├── chat-shell/                                     │
  │  │   ├── Cargo.toml                                  │
  │  │   └── src/                                        │
  │  │       ├── main.rs                                 │
  │  │       ├── ui/                                     │
  │  │       │   ├── chat_view.rs                        │
  │  │       │   ├── input.rs                            │
  │  │       │   └── message.rs                          │
  │  │       └── render/                                 │
  │  │           └── markdown.rs                         │
  │  ├── engine/                                         │
  │  │   ├── Cargo.toml                                  │
  │  │   └── src/                                        │
  │  │       └── main.rs                                 │
  │  └── Makefile                                        │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### File Content (Inline)

```
User: show me the config file

Levsha: Here's /etc/levsha/config.toml:

  ┌─ config.toml ──────────────── toml ────────────────┐
  │   1 │ [api]                                         │
  │   2 │ key = "sk-ant-•••••"                          │
  │   3 │ model = "claude-sonnet-4-20250514"            │
  │   4 │ base_url = "https://api.anthropic.com"        │
  │   5 │ timeout_seconds = 30                          │
  │   6 │ max_retries = 3                               │
  │   7 │                                               │
  │   8 │ [context]                                     │
  │   9 │ max_tokens = 200000                           │
  │  10 │ response_reserve = 4096                       │
  └─────────────────────────────────────────────────────┘
```

For files longer than 30 lines, content opens in the split-view panel instead.

---

## 2. File Operation Confirmations

### Delete Confirmation

File deletion is destructive and triggers the standard confirmation dialog.

```
User: delete /tmp/old-data

Levsha:
  ┌─ ⚠ Delete File ───────────────────────────────────┐
  │                                                      │
  │  This will permanently delete:                       │
  │                                                      │
  │  /tmp/old-data/                                      │
  │  • 12 files, 3 directories                           │
  │  • 4.2 MB total                                      │
  │                                                      │
  │  ┌──────────┐  ┌──────────┐                         │
  │  │  Delete  │  │  Cancel  │                         │
  │  └──────────┘  └──────────┘                         │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### Overwrite Confirmation

```
User: write "hello world" to /tmp/test.txt

Levsha:
  ┌─ ⚠ File Exists ───────────────────────────────────┐
  │                                                      │
  │  /tmp/test.txt already exists (256 B).               │
  │  Overwrite it?                                       │
  │                                                      │
  │  ┌─────────────┐  ┌──────────┐                     │
  │  │  Overwrite  │  │  Cancel  │                     │
  │  └─────────────┘  └──────────┘                     │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

---

## 3. Styling Reference

### Directory Listing Card

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Card cornerRadius | 8 |
| Directory icon (📁) | Lucide `folder`, `$accent-gold` (#D4A853) |
| File icon (📄) | Lucide `file`, `$text-tertiary` (#8C7E6E) |
| File name | IBM Plex Mono 13px, weight 500, `$text-primary` (#3A3228) |
| Permissions | IBM Plex Mono 11px, `$text-tertiary` |
| Size | IBM Plex Mono 11px, `$text-tertiary`, right-aligned |
| Summary line | IBM Plex Sans 12px, `$text-tertiary` |
| Path header | IBM Plex Mono 13px, weight 600, `$text-primary` |

### Tree View Card

| Element | Style |
|---------|-------|
| Tree lines (├── └──) | `$border-primary` (#EBE6DC), IBM Plex Mono 13px |
| Directory names | IBM Plex Mono 13px, weight 600, `$accent-gold` |
| File names | IBM Plex Mono 13px, weight 400, `$text-primary` |

### File Content Card

| Element | Style |
|---------|-------|
| Code background | `$bg-code` (#F7F4EE) |
| Language label | IBM Plex Sans 11px, weight 500, `$text-tertiary` |
| Line numbers | IBM Plex Mono 13px, `$text-tertiary`, right-aligned in 48px column |
| Code text | IBM Plex Mono 13px, `$text-primary`, syntax highlighted |
| Syntax colors | Per theme: `$syn-keyword`, `$syn-string`, `$syn-comment`, `$syn-function`, `$syn-number` |

### Delete Confirmation

| Element | Style |
|---------|-------|
| Container | `$danger-bg` (#FFF5F0), 2px `$accent-copper` border |
| Warning icon | Lucide `alert-triangle`, `$accent-gold` (#D4A853) |
| Title | IBM Plex Sans 15px, weight 600, `$text-primary` |
| Path | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Stats | IBM Plex Sans 12px, `$text-secondary` |
| Delete button | `$accent-copper` bg, white text |
| Cancel button | `$bg-secondary` bg, `$text-primary` text |

---

## 4. Search Results

```
User: find all .toml files

Levsha: Found 4 .toml files:

  ┌─ Search: *.toml ───────────────────────────────────┐
  │                                                      │
  │  📄 /etc/levsha/config.toml           1.2 KB        │
  │  📄 /etc/levsha/destructive.toml       842 B        │
  │  📄 /usr/src/levsha/Cargo.toml        1.8 KB        │
  │  📄 /usr/src/levsha/chat-shell/Cargo.toml  956 B   │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

---

## 5. Binary File Detection

When a user tries to read a binary file:

```
User: show me /usr/bin/levsha-chat

Levsha: That's a binary file (ELF executable, 4.2 MB).
        I can't display it as text.

        Would you like me to show you:
        • File size and type info
        • The file's permissions
```

---

## 6. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in filesystem messages |
| Lucide `folder` | Directory icon |
| Lucide `file` | File icon |
| Lucide `file-text` | Text file icon |
| Lucide `search` | Find results header |
| Lucide `alert-triangle` | Delete confirmation |
| Lucide `copy` | Copy operation |
| Lucide `move` | Move operation |
| Lucide `trash-2` | Delete operation |
