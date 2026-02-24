# 17 — Text Editor Skill: Technical Plan

**Module:** Text Editor Skill
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

The text editor is primarily an in-memory edit buffer in the engine with a display component in the chat shell.

```
engine/
  src/
    editor/
      mod.rs              # Public API, edit session management
      buffer.rs           # In-memory text buffer with undo stack
      operations.rs       # Edit operations (insert, delete, replace)
      diff.rs             # Diff generation between buffer and disk

skills/
  built-in/
    text-editor/
      skill.yaml
      prompts/
        text-editor.md
      tools/
        edit_open.json
        edit_insert.json
        edit_delete_lines.json
        edit_replace_lines.json
        edit_replace_text.json
        edit_append.json
        edit_save.json
        edit_undo.json
        edit_redo.json
        edit_diff.json
        edit_close.json
```

---

## 2. Edit Buffer

```rust
pub struct EditBuffer {
    file_path: PathBuf,
    lines: Vec<String>,
    original_lines: Vec<String>,  // Snapshot at open time
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    is_modified: bool,
}

#[derive(Clone)]
pub enum EditOperation {
    Insert { line: usize, content: Vec<String> },
    Delete { line: usize, count: usize, deleted: Vec<String> },
    Replace { line: usize, count: usize, old: Vec<String>, new: Vec<String> },
    ReplaceText { matches: Vec<TextMatch>, old_lines: Vec<String> },
}

#[derive(Clone)]
pub struct TextMatch {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
    pub old_text: String,
    pub new_text: String,
}

impl EditBuffer {
    pub fn open(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        Ok(Self {
            file_path: path.to_path_buf(),
            original_lines: lines.clone(),
            lines,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_modified: false,
        })
    }

    pub fn insert(&mut self, line: usize, content: &[String]) -> Result<()> {
        if line > self.lines.len() {
            return Err(EditError::LineOutOfRange(line, self.lines.len()));
        }
        let op = EditOperation::Insert {
            line,
            content: content.to_vec(),
        };
        for (i, text) in content.iter().enumerate() {
            self.lines.insert(line + i, text.clone());
        }
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.is_modified = true;
        Ok(())
    }

    pub fn delete_lines(&mut self, start: usize, count: usize) -> Result<Vec<String>> {
        if start + count > self.lines.len() {
            return Err(EditError::LineOutOfRange(start + count, self.lines.len()));
        }
        let deleted: Vec<String> = self.lines.drain(start..start + count).collect();
        let op = EditOperation::Delete {
            line: start,
            count,
            deleted: deleted.clone(),
        };
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.is_modified = true;
        Ok(deleted)
    }

    pub fn replace_lines(
        &mut self, start: usize, count: usize, new_content: &[String]
    ) -> Result<Vec<String>> {
        let old: Vec<String> = self.lines[start..start + count].to_vec();
        let op = EditOperation::Replace {
            line: start,
            count,
            old: old.clone(),
            new: new_content.to_vec(),
        };
        self.lines.splice(start..start + count, new_content.iter().cloned());
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.is_modified = true;
        Ok(old)
    }

    pub fn replace_text(
        &mut self, find: &str, replace: &str, regex: bool
    ) -> Result<usize> {
        let old_lines = self.lines.clone();
        let mut match_count = 0;

        if regex {
            let re = regex::Regex::new(find)?;
            for line in &mut self.lines {
                let new = re.replace_all(line, replace).to_string();
                if new != *line {
                    match_count += 1;
                    *line = new;
                }
            }
        } else {
            for line in &mut self.lines {
                if line.contains(find) {
                    match_count += 1;
                    *line = line.replace(find, replace);
                }
            }
        }

        if match_count > 0 {
            self.undo_stack.push(EditOperation::ReplaceText {
                matches: Vec::new(), // simplified
                old_lines,
            });
            self.redo_stack.clear();
            self.is_modified = true;
        }
        Ok(match_count)
    }

    pub fn undo(&mut self) -> Result<Option<String>> {
        let op = self.undo_stack.pop()
            .ok_or(EditError::NothingToUndo)?;
        let description = self.reverse_operation(&op);
        self.redo_stack.push(op);
        self.is_modified = self.lines != self.original_lines;
        Ok(Some(description))
    }

    pub fn redo(&mut self) -> Result<Option<String>> {
        let op = self.redo_stack.pop()
            .ok_or(EditError::NothingToRedo)?;
        let description = self.apply_operation(&op);
        self.undo_stack.push(op);
        self.is_modified = true;
        Ok(Some(description))
    }

    pub fn save(&mut self) -> Result<()> {
        let content = self.lines.join("\n");
        std::fs::write(&self.file_path, &content)?;
        self.original_lines = self.lines.clone();
        self.is_modified = false;
        Ok(())
    }

    pub fn diff(&self) -> String {
        generate_unified_diff(&self.original_lines, &self.lines, &self.file_path)
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn reverse_operation(&mut self, op: &EditOperation) -> String {
        match op {
            EditOperation::Insert { line, content } => {
                self.lines.drain(*line..*line + content.len());
                format!("Undone: removed {} inserted line(s) at line {}", content.len(), line + 1)
            }
            EditOperation::Delete { line, deleted, .. } => {
                for (i, text) in deleted.iter().enumerate() {
                    self.lines.insert(line + i, text.clone());
                }
                format!("Undone: restored {} deleted line(s) at line {}", deleted.len(), line + 1)
            }
            EditOperation::Replace { line, old, new, .. } => {
                self.lines.splice(*line..*line + new.len(), old.iter().cloned());
                format!("Undone: reverted {} line(s) at line {}", old.len(), line + 1)
            }
            EditOperation::ReplaceText { old_lines, .. } => {
                self.lines = old_lines.clone();
                "Undone: reverted text replacements".to_string()
            }
        }
    }
}
```

---

## 3. Edit Session Manager

```rust
pub struct EditSessionManager {
    active_session: Option<EditBuffer>,
}

impl EditSessionManager {
    pub fn open(&mut self, path: &Path) -> Result<EditSessionInfo> {
        if let Some(existing) = &self.active_session {
            if existing.is_modified {
                return Err(EditError::UnsavedChanges(
                    existing.file_path.to_string_lossy().to_string()
                ));
            }
        }
        let buffer = EditBuffer::open(path)?;
        let info = EditSessionInfo {
            path: path.to_string_lossy().to_string(),
            line_count: buffer.line_count(),
            language: detect_language(path),
        };
        self.active_session = Some(buffer);
        Ok(info)
    }

    pub fn close(&mut self, force: bool) -> Result<()> {
        if let Some(buffer) = &self.active_session {
            if buffer.is_modified && !force {
                return Err(EditError::UnsavedChanges(
                    buffer.file_path.to_string_lossy().to_string()
                ));
            }
        }
        self.active_session = None;
        Ok(())
    }

    pub fn buffer(&self) -> Result<&EditBuffer> {
        self.active_session.as_ref()
            .ok_or(EditError::NoActiveSession)
    }

    pub fn buffer_mut(&mut self) -> Result<&mut EditBuffer> {
        self.active_session.as_mut()
            .ok_or(EditError::NoActiveSession)
    }
}
```

---

## 4. IPC: Editor State Updates

The editor sends content updates to the split-view panel.

```rust
#[derive(Serialize, Deserialize)]
pub enum EditorUpdate {
    #[serde(rename = "editor_open")]
    Open {
        path: String,
        content: String,
        language: Option<String>,
    },
    #[serde(rename = "editor_content")]
    ContentChanged {
        content: String,
        modified_lines: Vec<usize>,  // Lines that changed (for highlighting)
        is_modified: bool,
    },
    #[serde(rename = "editor_saved")]
    Saved,
    #[serde(rename = "editor_closed")]
    Closed,
}
```

---

## 5. Tool Implementations

```rust
pub fn handle_edit_open(input: &serde_json::Value, session: &mut EditSessionManager) -> Result<ToolOutput> {
    let path = input["path"].as_str().ok_or("missing path")?;
    let info = session.open(Path::new(path))?;
    let buffer = session.buffer()?;

    // Send file content to split-view panel
    Ok(ToolOutput::with_editor_update(
        format!("Opened {} ({} lines, {})", path, info.line_count,
            info.language.as_deref().unwrap_or("text")),
        EditorUpdate::Open {
            path: path.to_string(),
            content: buffer.content(),
            language: info.language,
        }
    ))
}

pub fn handle_edit_replace_lines(
    input: &serde_json::Value, session: &mut EditSessionManager
) -> Result<ToolOutput> {
    let start = input["start_line"].as_u64().ok_or("missing start_line")? as usize - 1; // 1-indexed
    let end = input["end_line"].as_u64().ok_or("missing end_line")? as usize; // inclusive
    let content = input["content"].as_str().ok_or("missing content")?;
    let new_lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    let buffer = session.buffer_mut()?;
    let count = end - start;
    let old = buffer.replace_lines(start, count, &new_lines)?;

    let modified: Vec<usize> = (start..start + new_lines.len()).collect();
    Ok(ToolOutput::with_editor_update(
        format!("Replaced lines {}-{}", start + 1, end),
        EditorUpdate::ContentChanged {
            content: buffer.content(),
            modified_lines: modified,
            is_modified: buffer.is_modified,
        }
    ))
}
```

---

## 6. Implementation Stages

**Stage 1 — Edit Buffer (1-2 days)**
1. Implement `EditBuffer` with all operations.
2. Undo/redo stack.
3. Diff generation.
4. Unit tests for all operations.

**Stage 2 — Session Manager (0.5 day)**
1. `EditSessionManager` with open/close.
2. Unsaved changes detection.
3. Single-session enforcement.

**Stage 3 — Tool Handlers (1 day)**
1. All 11 tool handlers.
2. JSON tool schemas.
3. Prompt fragment.

**Stage 4 — Panel Integration (1 day)**
1. Editor content display in split-view panel.
2. Modified line highlighting.
3. Footer with action buttons.
4. Save/undo/close keyboard shortcuts.

**Stage 5 — Testing (0.5 day)**
1. Buffer operation unit tests.
2. Undo/redo sequences.
3. End-to-end: open, edit, save, verify file changed.

---

## 7. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `buffer.rs` | Insert, delete, replace at various positions; edge cases (line 0, last line) |
| `buffer.rs` | Undo single operation, undo multiple, redo after undo |
| `buffer.rs` | Replace text (literal), replace text (regex) |
| `buffer.rs` | Save writes correct content, is_modified flag updates |
| `diff.rs` | Diff generation accuracy |
| `operations.rs` | Line number recalculation after insert/delete |

### Integration Tests

| Test | Method |
|------|--------|
| Full edit cycle | Open file, edit lines, save, verify file on disk |
| Undo after save | Open, edit, save, undo, verify buffer changed but file unchanged |
| Close without saving | Open, edit, close (force), verify file unchanged |
| Close with unsaved | Open, edit, close, verify error returned |
| Large file | Open 1000-line file, edit line 500, verify performance |
| Regex replace | Open file with pattern, replace with regex, verify |
| Panel sync | Open file, edit, verify IPC content update sent |
