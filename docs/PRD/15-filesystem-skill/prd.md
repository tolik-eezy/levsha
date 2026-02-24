# 15 — Filesystem Skill: Product Requirements

**Module:** Filesystem Skill
**Phase:** 2 (Separate Project)
**Status:** Draft

---

## 1. Overview

The filesystem skill provides structured file management capabilities through the chat. While Phase 1's tool executor can run arbitrary shell commands (including `ls`, `cp`, `mv`), this skill gives the LLM well-defined tools with proper parameter handling, output formatting, and safety checks.

---

## 2. Functional Requirements

### 2.1 File Browsing (FS-01)

| Field | Value |
|-------|-------|
| **ID** | FS-01 |
| **Priority** | P0 |
| **Requirement** | Users can browse the filesystem via chat. |

**Tools:**

| Tool | Description | Command Template |
|------|-------------|-----------------|
| `fs_list` | List directory contents | `ls -la --color=never {{path}}` |
| `fs_tree` | Show directory tree | `tree -L {{depth}} {{path}}` |
| `fs_find` | Find files by name/pattern | `find {{path}} -name '{{pattern}}' -maxdepth {{depth}}` |
| `fs_size` | Show file/directory size | `du -sh {{path}}` |

**Acceptance Criteria:**

- [ ] User can list directory contents.
- [ ] User can browse with tree view.
- [ ] User can search for files by name or pattern.
- [ ] Output is formatted cleanly in the chat.

### 2.2 File Operations (FS-02)

| Field | Value |
|-------|-------|
| **ID** | FS-02 |
| **Priority** | P0 |
| **Requirement** | Users can copy, move, rename, and delete files via chat. |

**Tools:**

| Tool | Description | Command Template | Destructive |
|------|-------------|-----------------|-------------|
| `fs_copy` | Copy file or directory | `cp -r {{source}} {{destination}}` | No |
| `fs_move` | Move or rename | `mv {{source}} {{destination}}` | Yes (if overwrite) |
| `fs_delete` | Delete file or directory | `rm -rf {{path}}` | **Yes** |
| `fs_mkdir` | Create directory | `mkdir -p {{path}}` | No |
| `fs_chmod` | Change permissions | `chmod {{mode}} {{path}}` | Yes |

**Acceptance Criteria:**

- [ ] File copy, move, delete, and mkdir work.
- [ ] Delete operations trigger destructive command confirmation.
- [ ] Move with overwrite triggers confirmation.
- [ ] Operations report success/failure clearly.

### 2.3 File Viewing (FS-03)

| Field | Value |
|-------|-------|
| **ID** | FS-03 |
| **Priority** | P0 |
| **Requirement** | Users can read file contents via chat. |

**Tools:**

| Tool | Description | Command Template |
|------|-------------|-----------------|
| `fs_read` | Read entire file | `cat {{path}}` |
| `fs_head` | Read first N lines | `head -n {{lines}} {{path}}` |
| `fs_tail` | Read last N lines | `tail -n {{lines}} {{path}}` |

When split-view (module 12) is available, large files open in the content panel. Otherwise, content is displayed inline with truncation for very long files.

**Acceptance Criteria:**

- [ ] User can read files via chat.
- [ ] Large files are truncated with a "show more" option.
- [ ] Binary files are detected and not dumped as text.
- [ ] Integration with split-view panel when available.

### 2.4 File Editing (FS-04)

| Field | Value |
|-------|-------|
| **ID** | FS-04 |
| **Priority** | P1 |
| **Requirement** | Users can make simple edits to text files via chat. |

**Tools:**

| Tool | Description |
|------|-------------|
| `fs_write` | Write content to a file (create or overwrite). |
| `fs_append` | Append content to a file. |
| `fs_replace` | Replace a string or pattern in a file. |

**Acceptance Criteria:**

- [ ] User can create new files with specified content.
- [ ] User can append to existing files.
- [ ] User can do find-and-replace in files.
- [ ] File overwrite triggers confirmation.

---

## 3. Skill Manifest

```yaml
name: filesystem
version: 0.1.0
description: "Browse, copy, move, delete, and edit files"
author: levsha
builtin: false

prompt: prompts/filesystem.md
tools:
  - tools/fs_list.json
  - tools/fs_tree.json
  - tools/fs_find.json
  - tools/fs_size.json
  - tools/fs_copy.json
  - tools/fs_move.json
  - tools/fs_delete.json
  - tools/fs_mkdir.json
  - tools/fs_chmod.json
  - tools/fs_read.json
  - tools/fs_head.json
  - tools/fs_tail.json
  - tools/fs_write.json
  - tools/fs_append.json
  - tools/fs_replace.json

requires:
  packages:
    - tree
    - coreutils
```

---

## 4. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Skills System (04) | Upstream | Uses the skill framework. |
| Split-View (12) | Optional | File preview can use the content panel. |
| Intelligence Engine (03) | Upstream | Tools dispatched by the engine. |
