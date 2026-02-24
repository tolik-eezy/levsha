//! In-memory text editor buffer with undo/redo support (Track H).
//!
//! Provides a line-oriented editing model where all operations are reversible.
//! Line numbers in the public API are 1-indexed.

use std::fmt;
use std::path::PathBuf;

/// Represents a single reversible editing operation.
#[derive(Debug, Clone)]
pub enum EditOperation {
    Insert {
        line: usize,
        content: Vec<String>,
    },
    Delete {
        line: usize,
        count: usize,
        deleted: Vec<String>,
    },
    ReplaceLines {
        line: usize,
        old: Vec<String>,
        new: Vec<String>,
    },
    ReplaceText {
        find: String,
        replace: String,
        old_lines: Vec<String>,
        match_count: usize,
    },
}

/// Errors from edit operations.
#[derive(Debug)]
pub enum EditError {
    NoFileOpen,
    LineOutOfRange(usize, usize),
    NothingToUndo,
    NothingToRedo,
    UnsavedChanges(String),
    Io(std::io::Error),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFileOpen => write!(f, "No file is currently open"),
            Self::LineOutOfRange(req, total) => {
                write!(f, "Line {} out of range (file has {} lines)", req, total)
            }
            Self::NothingToUndo => write!(f, "Nothing to undo"),
            Self::NothingToRedo => write!(f, "Nothing to redo"),
            Self::UnsavedChanges(path) => write!(
                f,
                "File '{}' has unsaved changes. Save first or use force=true.",
                path
            ),
            Self::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl From<std::io::Error> for EditError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// In-memory edit buffer with undo/redo support.
pub struct EditState {
    pub file_path: PathBuf,
    lines: Vec<String>,
    original_lines: Vec<String>,
    undo_stack: Vec<EditOperation>,
    redo_stack: Vec<EditOperation>,
    is_dirty: bool,
    pub content_id: String,
}

impl EditState {
    /// Open a file for editing. If the file doesn't exist, creates an empty buffer.
    pub fn open(path: &str, content_id: String) -> Result<Self, EditError> {
        let file_path = PathBuf::from(path);
        let lines = if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)?;
            content.lines().map(String::from).collect()
        } else {
            Vec::new()
        };

        let original_lines = lines.clone();

        Ok(Self {
            file_path,
            lines,
            original_lines,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            is_dirty: false,
            content_id,
        })
    }

    /// Insert lines before the given 1-indexed line number.
    /// If `line` equals `line_count() + 1`, appends at end.
    pub fn insert_lines(&mut self, line: usize, content: Vec<String>) -> Result<String, EditError> {
        if line == 0 || line > self.lines.len() + 1 {
            return Err(EditError::LineOutOfRange(line, self.lines.len()));
        }

        let idx = line - 1;
        let count = content.len();

        // Push inverse operation (Delete) to undo stack.
        self.undo_stack.push(EditOperation::Delete {
            line,
            count,
            deleted: content.clone(),
        });
        self.redo_stack.clear();

        // Insert lines.
        for (i, l) in content.into_iter().enumerate() {
            self.lines.insert(idx + i, l);
        }

        self.is_dirty = true;

        Ok(format!("Inserted {} line(s) at line {}", count, line))
    }

    /// Delete `count` lines starting at the 1-indexed `start_line`.
    pub fn delete_lines(&mut self, start_line: usize, count: usize) -> Result<String, EditError> {
        if start_line == 0 || start_line > self.lines.len() {
            return Err(EditError::LineOutOfRange(start_line, self.lines.len()));
        }

        let idx = start_line - 1;
        let actual_count = count.min(self.lines.len() - idx);

        let deleted: Vec<String> = self.lines.drain(idx..idx + actual_count).collect();

        // Push inverse operation (Insert) to undo stack.
        self.undo_stack.push(EditOperation::Insert {
            line: start_line,
            content: deleted.clone(),
        });
        self.redo_stack.clear();
        self.is_dirty = true;

        Ok(format!(
            "Deleted {} line(s) starting at line {}",
            actual_count, start_line
        ))
    }

    /// Replace `count` lines starting at the 1-indexed `start_line` with new content.
    pub fn replace_lines(
        &mut self,
        start_line: usize,
        count: usize,
        content: Vec<String>,
    ) -> Result<String, EditError> {
        if start_line == 0 || start_line > self.lines.len() {
            return Err(EditError::LineOutOfRange(start_line, self.lines.len()));
        }

        let idx = start_line - 1;
        let actual_count = count.min(self.lines.len() - idx);

        let old: Vec<String> = self.lines.drain(idx..idx + actual_count).collect();
        let new_count = content.len();

        for (i, l) in content.clone().into_iter().enumerate() {
            self.lines.insert(idx + i, l);
        }

        // Push inverse operation to undo stack.
        self.undo_stack.push(EditOperation::ReplaceLines {
            line: start_line,
            old: content,
            new: old,
        });
        self.redo_stack.clear();
        self.is_dirty = true;

        Ok(format!(
            "Replaced {} line(s) with {} line(s) at line {}",
            actual_count, new_count, start_line
        ))
    }

    /// Find and replace across all lines.
    pub fn replace_text(&mut self, find: &str, replace: &str) -> Result<String, EditError> {
        let old_lines = self.lines.clone();
        let mut match_count = 0;

        for line in &mut self.lines {
            let count = line.matches(find).count();
            if count > 0 {
                match_count += count;
                *line = line.replace(find, replace);
            }
        }

        if match_count == 0 {
            return Ok(format!("No occurrences of '{}' found", find));
        }

        // Push inverse: restore old_lines snapshot.
        self.undo_stack.push(EditOperation::ReplaceText {
            find: find.to_string(),
            replace: replace.to_string(),
            old_lines,
            match_count,
        });
        self.redo_stack.clear();
        self.is_dirty = true;

        Ok(format!(
            "Replaced {} occurrence(s) of '{}' with '{}'",
            match_count, find, replace
        ))
    }

    /// Append lines at the end of the buffer.
    pub fn append(&mut self, content: Vec<String>) -> Result<String, EditError> {
        let start_line = self.lines.len() + 1;
        let count = content.len();

        // Push inverse (Delete the appended lines).
        self.undo_stack.push(EditOperation::Delete {
            line: start_line,
            count,
            deleted: content.clone(),
        });
        self.redo_stack.clear();

        self.lines.extend(content);
        self.is_dirty = true;

        Ok(format!(
            "Appended {} line(s) (total: {} lines)",
            count,
            self.lines.len()
        ))
    }

    /// Save the buffer to disk.
    pub fn save(&mut self) -> Result<String, EditError> {
        let content = self.lines.join("\n");
        // Ensure parent directory exists.
        if let Some(parent) = self.file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.file_path, &content)?;
        self.original_lines = self.lines.clone();
        self.is_dirty = false;

        Ok(format!(
            "Saved {} lines to {}",
            self.lines.len(),
            self.file_path.display()
        ))
    }

    /// Undo the last edit operation.
    ///
    /// The undo stack stores **inverse operations**: applying them reverses
    /// the corresponding forward edit.
    pub fn undo(&mut self) -> Result<String, EditError> {
        let op = self.undo_stack.pop().ok_or(EditError::NothingToUndo)?;

        let description = match op {
            EditOperation::Insert { line, content } => {
                // The inverse was "insert these lines" -> apply the insert.
                let idx = line - 1;
                let count = content.len();
                for (i, l) in content.clone().into_iter().enumerate() {
                    self.lines.insert(idx + i, l);
                }
                // To redo (i.e., undo the undo), we need to delete them again.
                self.redo_stack.push(EditOperation::Delete {
                    line,
                    count,
                    deleted: content,
                });
                format!("Undid delete at line {}", line)
            }
            EditOperation::Delete {
                line,
                count,
                deleted,
            } => {
                // The inverse was "delete these lines" -> apply the delete.
                let idx = line - 1;
                let actual = count.min(self.lines.len().saturating_sub(idx));
                let removed: Vec<String> = self.lines.drain(idx..idx + actual).collect();
                // To redo (i.e., undo the undo), re-insert the deleted content.
                // Use `deleted` if available (more robust), fall back to `removed`.
                let redo_content = if !deleted.is_empty() { deleted } else { removed };
                self.redo_stack.push(EditOperation::Insert {
                    line,
                    content: redo_content,
                });
                format!("Undid insert at line {}", line)
            }
            EditOperation::ReplaceLines { line, old, new } => {
                // The inverse was "replace with `new`" -> apply it: remove `old`, insert `new`.
                let idx = line - 1;
                let _ = self.lines.drain(idx..idx + old.len());
                for (i, l) in new.clone().into_iter().enumerate() {
                    self.lines.insert(idx + i, l);
                }
                // To redo, swap back.
                self.redo_stack.push(EditOperation::ReplaceLines {
                    line,
                    old: new,
                    new: old,
                });
                format!("Undid replace at line {}", line)
            }
            EditOperation::ReplaceText {
                find,
                replace,
                old_lines,
                match_count,
            } => {
                // The inverse stores the old_lines snapshot -> restore them.
                let current_lines = std::mem::replace(&mut self.lines, old_lines);
                self.redo_stack.push(EditOperation::ReplaceText {
                    find: find.clone(),
                    replace: replace.clone(),
                    old_lines: current_lines,
                    match_count,
                });
                format!("Undid replace '{}' -> '{}'", find, replace)
            }
        };

        self.is_dirty = self.lines != self.original_lines;
        Ok(description)
    }

    /// Redo the last undone operation.
    ///
    /// The redo stack also stores **inverse operations** — applying them
    /// reverses the undo.
    pub fn redo(&mut self) -> Result<String, EditError> {
        let op = self.redo_stack.pop().ok_or(EditError::NothingToRedo)?;

        let description = match op {
            EditOperation::Insert { line, content } => {
                // Redo means "insert these lines back".
                let idx = line - 1;
                let count = content.len();
                for (i, l) in content.clone().into_iter().enumerate() {
                    self.lines.insert(idx + i, l);
                }
                // Push inverse (delete) to undo.
                self.undo_stack.push(EditOperation::Delete {
                    line,
                    count,
                    deleted: content,
                });
                format!("Redid insert at line {}", line)
            }
            EditOperation::Delete {
                line,
                count,
                deleted: _,
            } => {
                // Redo means "delete these lines again".
                let idx = line - 1;
                let actual = count.min(self.lines.len() - idx);
                let removed: Vec<String> = self.lines.drain(idx..idx + actual).collect();
                // Push inverse (insert) to undo.
                self.undo_stack.push(EditOperation::Insert {
                    line,
                    content: removed,
                });
                format!("Redid delete at line {}", line)
            }
            EditOperation::ReplaceLines { line, old, new } => {
                // Redo: remove `old`, insert `new`.
                let idx = line - 1;
                let _ = self.lines.drain(idx..idx + old.len());
                for (i, l) in new.clone().into_iter().enumerate() {
                    self.lines.insert(idx + i, l);
                }
                self.undo_stack.push(EditOperation::ReplaceLines {
                    line,
                    old: new,
                    new: old,
                });
                format!("Redid replace at line {}", line)
            }
            EditOperation::ReplaceText {
                find,
                replace,
                old_lines,
                match_count,
            } => {
                let current_lines = std::mem::replace(&mut self.lines, old_lines);
                self.undo_stack.push(EditOperation::ReplaceText {
                    find: find.clone(),
                    replace: replace.clone(),
                    old_lines: current_lines,
                    match_count,
                });
                format!("Redid replace '{}' -> '{}'", find, replace)
            }
        };

        self.is_dirty = self.lines != self.original_lines;
        Ok(description)
    }

    /// Generate a unified-style diff between original and current lines.
    pub fn diff(&self) -> String {
        if self.lines == self.original_lines {
            return "No changes.".to_string();
        }

        let mut output = String::new();
        output.push_str(&format!(
            "--- {}\n+++ {} (modified)\n",
            self.file_path.display(),
            self.file_path.display()
        ));

        let max_len = self.original_lines.len().max(self.lines.len());
        let context = 3;
        let mut last_printed = None;

        for i in 0..max_len {
            let orig = self.original_lines.get(i);
            let curr = self.lines.get(i);

            if orig != curr {
                // Print context lines before the change.
                let start = if i >= context { i - context } else { 0 };
                for j in start..i {
                    if last_printed.map_or(true, |lp| j > lp) {
                        if let Some(line) = self.original_lines.get(j) {
                            if last_printed.is_some() || j > start {
                                // Already have context, skip.
                            }
                            output.push_str(&format!(" {}\n", line));
                            last_printed = Some(j);
                        }
                    }
                }

                // Print the changed line.
                if let Some(line) = orig {
                    output.push_str(&format!("-{}\n", line));
                }
                if let Some(line) = curr {
                    output.push_str(&format!("+{}\n", line));
                }
                last_printed = Some(i);
            } else if let (Some(line), Some(_)) = (orig, curr) {
                // Context after a change.
                if let Some(lp) = last_printed {
                    if i <= lp + context {
                        output.push_str(&format!(" {}\n", line));
                        last_printed = Some(i);
                    }
                }
            }
        }

        output
    }

    /// Return the full content as a single string (lines joined by newlines).
    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Return the original content (before any edits) as a single string.
    pub fn original_content(&self) -> String {
        self.original_lines.join("\n")
    }

    /// Whether the buffer has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.is_dirty
    }

    /// Check whether it is safe to close. If dirty and not forced, returns an error.
    pub fn can_close(&self, force: bool) -> Result<(), EditError> {
        if self.is_dirty && !force {
            Err(EditError::UnsavedChanges(
                self.file_path.display().to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Return the number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_temp_file(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "{}", content).unwrap();
        f
    }

    #[test]
    fn test_open_file() {
        let f = create_temp_file("line1\nline2\nline3");
        let state = EditState::open(f.path().to_str().unwrap(), "id1".into()).unwrap();
        assert_eq!(state.line_count(), 3);
        assert_eq!(state.content(), "line1\nline2\nline3");
        assert!(!state.is_dirty());
    }

    #[test]
    fn test_open_nonexistent() {
        let state = EditState::open("/tmp/levsha_test_nonexistent_file_12345.txt", "id2".into())
            .unwrap();
        assert_eq!(state.line_count(), 0);
        assert_eq!(state.content(), "");
        assert!(!state.is_dirty());
    }

    #[test]
    fn test_insert_lines() {
        let f = create_temp_file("a\nb\nc");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let result = state.insert_lines(2, vec!["x".into(), "y".into()]).unwrap();
        assert!(result.contains("Inserted 2"));
        assert_eq!(state.content(), "a\nx\ny\nb\nc");
        assert_eq!(state.line_count(), 5);
        assert!(state.is_dirty());
    }

    #[test]
    fn test_delete_lines() {
        let f = create_temp_file("a\nb\nc\nd");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let result = state.delete_lines(2, 2).unwrap();
        assert!(result.contains("Deleted 2"));
        assert_eq!(state.content(), "a\nd");
        assert!(state.is_dirty());
    }

    #[test]
    fn test_replace_lines() {
        let f = create_temp_file("a\nb\nc\nd");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let result = state
            .replace_lines(2, 2, vec!["X".into(), "Y".into(), "Z".into()])
            .unwrap();
        assert!(result.contains("Replaced 2"));
        assert_eq!(state.content(), "a\nX\nY\nZ\nd");
    }

    #[test]
    fn test_replace_text() {
        let f = create_temp_file("hello world\nhello rust\ngoodbye");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let result = state.replace_text("hello", "hi").unwrap();
        assert!(result.contains("Replaced 2"));
        assert_eq!(state.content(), "hi world\nhi rust\ngoodbye");
    }

    #[test]
    fn test_append() {
        let f = create_temp_file("a\nb");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let result = state.append(vec!["c".into(), "d".into()]).unwrap();
        assert!(result.contains("Appended 2"));
        assert_eq!(state.content(), "a\nb\nc\nd");
    }

    #[test]
    fn test_undo_insert() {
        let f = create_temp_file("a\nb\nc");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.insert_lines(2, vec!["x".into()]).unwrap();
        assert_eq!(state.content(), "a\nx\nb\nc");

        state.undo().unwrap();
        assert_eq!(state.content(), "a\nb\nc");
        assert!(!state.is_dirty());
    }

    #[test]
    fn test_redo() {
        let f = create_temp_file("a\nb\nc");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.insert_lines(2, vec!["x".into()]).unwrap();
        state.undo().unwrap();
        assert_eq!(state.content(), "a\nb\nc");

        state.redo().unwrap();
        assert_eq!(state.content(), "a\nx\nb\nc");
        assert!(state.is_dirty());
    }

    #[test]
    fn test_save() {
        let f = create_temp_file("original");
        let path = f.path().to_str().unwrap().to_string();
        let mut state = EditState::open(&path, "id".into()).unwrap();
        state.append(vec!["new line".into()]).unwrap();
        assert!(state.is_dirty());

        let result = state.save().unwrap();
        assert!(result.contains("Saved"));
        assert!(!state.is_dirty());

        // Verify file content.
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, "original\nnew line");
    }

    #[test]
    fn test_diff() {
        let f = create_temp_file("a\nb\nc");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.replace_lines(2, 1, vec!["B".into()]).unwrap();
        let diff = state.diff();
        assert!(diff.contains("-b"));
        assert!(diff.contains("+B"));
    }

    #[test]
    fn test_line_out_of_range() {
        let f = create_temp_file("a\nb");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();

        // Line 0 is invalid.
        let err = state.insert_lines(0, vec!["x".into()]);
        assert!(err.is_err());

        // Line 4 is out of range for a 2-line file (max valid insert is 3).
        let err = state.insert_lines(4, vec!["x".into()]);
        assert!(err.is_err());

        // Line 3 is valid (append position).
        let ok = state.insert_lines(3, vec!["x".into()]);
        assert!(ok.is_ok());
    }

    #[test]
    fn test_nothing_to_undo() {
        let f = create_temp_file("a");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let err = state.undo();
        assert!(matches!(err, Err(EditError::NothingToUndo)));
    }

    #[test]
    fn test_nothing_to_redo() {
        let f = create_temp_file("a");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let err = state.redo();
        assert!(matches!(err, Err(EditError::NothingToRedo)));
    }

    #[test]
    fn test_unsaved_changes_warning() {
        let f = create_temp_file("a");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.append(vec!["b".into()]).unwrap();

        let err = state.can_close(false);
        assert!(matches!(err, Err(EditError::UnsavedChanges(_))));
    }

    #[test]
    fn test_close_force() {
        let f = create_temp_file("a");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.append(vec!["b".into()]).unwrap();

        let ok = state.can_close(true);
        assert!(ok.is_ok());
    }

    #[test]
    fn test_undo_delete() {
        let f = create_temp_file("a\nb\nc");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.delete_lines(2, 1).unwrap();
        assert_eq!(state.content(), "a\nc");

        state.undo().unwrap();
        assert_eq!(state.content(), "a\nb\nc");
    }

    #[test]
    fn test_undo_replace_lines() {
        let f = create_temp_file("a\nb\nc");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.replace_lines(2, 1, vec!["X".into()]).unwrap();
        assert_eq!(state.content(), "a\nX\nc");

        state.undo().unwrap();
        assert_eq!(state.content(), "a\nb\nc");
    }

    #[test]
    fn test_undo_replace_text() {
        let f = create_temp_file("hello world\nhello rust");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.replace_text("hello", "hi").unwrap();
        assert_eq!(state.content(), "hi world\nhi rust");

        state.undo().unwrap();
        assert_eq!(state.content(), "hello world\nhello rust");
    }

    #[test]
    fn test_undo_append() {
        let f = create_temp_file("a");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        state.append(vec!["b".into(), "c".into()]).unwrap();
        assert_eq!(state.content(), "a\nb\nc");

        state.undo().unwrap();
        assert_eq!(state.content(), "a");
    }

    #[test]
    fn test_replace_text_no_match() {
        let f = create_temp_file("hello world");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        let result = state.replace_text("xyz", "abc").unwrap();
        assert!(result.contains("No occurrences"));
        assert!(!state.is_dirty());
    }

    #[test]
    fn test_diff_no_changes() {
        let f = create_temp_file("a\nb\nc");
        let state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();
        assert_eq!(state.diff(), "No changes.");
    }

    #[test]
    fn test_multiple_undo_redo() {
        let f = create_temp_file("a\nb\nc");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();

        state.insert_lines(2, vec!["x".into()]).unwrap();
        state.append(vec!["d".into()]).unwrap();
        assert_eq!(state.content(), "a\nx\nb\nc\nd");

        state.undo().unwrap();
        assert_eq!(state.content(), "a\nx\nb\nc");

        state.undo().unwrap();
        assert_eq!(state.content(), "a\nb\nc");

        state.redo().unwrap();
        assert_eq!(state.content(), "a\nx\nb\nc");

        state.redo().unwrap();
        assert_eq!(state.content(), "a\nx\nb\nc\nd");
    }

    #[test]
    fn test_new_op_clears_redo_stack() {
        let f = create_temp_file("a\nb");
        let mut state = EditState::open(f.path().to_str().unwrap(), "id".into()).unwrap();

        state.insert_lines(2, vec!["x".into()]).unwrap();
        state.undo().unwrap();

        // Now do a different op, which should clear redo.
        state.append(vec!["y".into()]).unwrap();
        let err = state.redo();
        assert!(matches!(err, Err(EditError::NothingToRedo)));
    }
}
