# 17 — Text Editor Skill: Product Requirements

**Module:** Text Editor Skill
**Phase:** 2 (Separate Project)
**Status:** Draft

---

## 1. Overview

The text editor skill provides structured text editing capabilities through the chat. Unlike the filesystem skill's basic `fs_write`/`fs_replace` tools, the text editor skill is designed for multi-step editing workflows — editing config files, writing scripts, composing documents — with undo, line-level operations, and intelligent editing assistance from the LLM.

When the split-view panel (module 12) is available, the text editor renders the file being edited in the content panel while the user gives editing instructions in the chat.

---

## 2. Functional Requirements

### 2.1 File Opening and Display (TE-01)

| Field | Value |
|-------|-------|
| **ID** | TE-01 |
| **Priority** | P0 |
| **Requirement** | User can open a file for editing, which displays in the split-view panel. |

**Details:**

- "Edit /etc/levsha/config.toml" opens the file in the content panel with syntax highlighting and line numbers.
- The file stays open across multiple chat turns until explicitly closed.
- Unsaved changes are tracked and marked.

**Acceptance Criteria:**

- [ ] Files open in the split-view panel for editing.
- [ ] Syntax highlighting and line numbers are displayed.
- [ ] File remains open across chat turns.
- [ ] Unsaved changes are indicated.

### 2.2 Line-Level Editing (TE-02)

| Field | Value |
|-------|-------|
| **ID** | TE-02 |
| **Priority** | P0 |
| **Requirement** | User can perform line-level edit operations via chat. |

**Tools:**

| Tool | Description |
|------|-------------|
| `edit_insert` | Insert text at a specific line number. |
| `edit_delete_lines` | Delete a range of lines. |
| `edit_replace_lines` | Replace a range of lines with new content. |
| `edit_replace_text` | Find and replace text (with regex support). |
| `edit_append` | Append text to the end of the file. |

**Acceptance Criteria:**

- [ ] Line-level insert, delete, and replace work.
- [ ] Find-and-replace with regex works.
- [ ] Changes are reflected immediately in the split-view panel.
- [ ] Line numbers update after insertions/deletions.

### 2.3 Save and Undo (TE-03)

| Field | Value |
|-------|-------|
| **ID** | TE-03 |
| **Priority** | P0 |
| **Requirement** | User can save changes and undo recent edits. |

**Tools:**

| Tool | Description |
|------|-------------|
| `edit_save` | Write changes to disk. |
| `edit_undo` | Undo the last edit operation. |
| `edit_redo` | Redo an undone operation. |
| `edit_diff` | Show unsaved changes as a diff. |
| `edit_close` | Close the file (prompts to save if unsaved changes). |

**Acceptance Criteria:**

- [ ] Save writes to disk.
- [ ] Undo reverts the last operation.
- [ ] Multiple undos are supported (undo stack).
- [ ] Closing with unsaved changes prompts the user.

### 2.4 Intelligent Editing (TE-04)

| Field | Value |
|-------|-------|
| **ID** | TE-04 |
| **Priority** | P1 |
| **Requirement** | The LLM can make intelligent edits based on natural language instructions. |

**Details:**

Because the LLM has the file content in context and access to edit tools, the user can give high-level instructions:

- "Add a comment explaining what the `[api]` section does"
- "Fix the indentation"
- "Add error handling to this function"
- "Remove all the commented-out lines"

The LLM reads the file, decides which edit operations to perform, and executes them.

**Acceptance Criteria:**

- [ ] Natural language editing instructions are interpreted correctly.
- [ ] Multi-step edits are executed in sequence.
- [ ] User can review changes before saving.

---

## 3. Skill Manifest

```yaml
name: text-editor
version: 0.1.0
description: "Edit text files with line-level operations, undo, and LLM-assisted editing"
author: levsha
builtin: false

prompt: prompts/text-editor.md
tools:
  - tools/edit_open.json
  - tools/edit_insert.json
  - tools/edit_delete_lines.json
  - tools/edit_replace_lines.json
  - tools/edit_replace_text.json
  - tools/edit_append.json
  - tools/edit_save.json
  - tools/edit_undo.json
  - tools/edit_redo.json
  - tools/edit_diff.json
  - tools/edit_close.json

requires:
  packages: []
```

---

## 4. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Skills System (04) | Upstream | Uses the skill framework. |
| Split-View (12) | Required | File display uses the content panel. |
| Filesystem Skill (15) | Sibling | Basic file operations are in the filesystem skill. |
