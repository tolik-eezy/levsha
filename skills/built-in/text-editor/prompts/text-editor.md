# Text Editor Skill

You have an in-memory text editor with full undo/redo support. Use it for multi-step file editing where you need to make several changes before saving.

## Opening and Closing

- Always use `edit_open` before any editing operation. This loads the file into the editor buffer.
- Only **one file** can be open at a time. Close the current file before opening another.
- Use `edit_close` when done editing. If there are unsaved changes, the editor will warn you. Pass `force: true` to discard unsaved changes.
- Always `edit_save` before `edit_close` unless the user explicitly wants to discard changes.

## Editing Operations

### Inserting Lines
- Use `edit_insert` to add new lines at a specific position. The `line` parameter is 1-indexed and specifies where the new content is inserted **before**.
- Content can be a single string (one line) or an array of strings (multiple lines).
- To insert at the very beginning of the file, use `line: 1`.

### Deleting Lines
- Use `edit_delete_lines` to remove lines starting at `start_line`. The `count` parameter specifies how many lines to delete (default: 1).
- Line numbers are 1-indexed.

### Replacing Lines
- Use `edit_replace_lines` to replace a range of lines with new content. Specify `start_line`, `count` (number of lines to replace), and the replacement `content`.
- The replacement content can have a different number of lines than what it replaces.

### Find and Replace
- Use `edit_replace_text` to find and replace text across the entire file. This replaces **all occurrences** of the `find` string with the `replace` string.
- This operates on the raw text content, not on line boundaries.

### Appending
- Use `edit_append` to add lines at the end of the file. Content can be a single string or an array of strings.

## Saving

- Use `edit_save` to write the buffer contents back to disk. This overwrites the original file.
- After saving, the editor remains open with the buffer intact for further editing.
- The diff baseline resets after each save.

## Undo and Redo

- Use `edit_undo` to revert the last editing operation. Each individual operation (insert, delete, replace, etc.) is one undo step.
- Use `edit_redo` to re-apply an undone operation.
- The undo history is linear — making a new edit after undoing clears the redo stack.
- `edit_save` does not count as an undoable operation.

## Reviewing Changes

- Use `edit_diff` to see a unified diff of all changes since the last save (or since opening, if the file hasn't been saved yet).
- Review the diff before saving to verify changes are correct.

## When to Use the Editor vs. fs_write

- **Use `fs_write`** for quick, single-step writes: creating new files, overwriting with entirely new content, or simple one-line edits.
- **Use the text editor** when you need to:
  - Make multiple edits to different parts of the same file
  - Review changes before committing them to disk
  - Have the ability to undo mistakes during a complex edit session
  - Perform find-and-replace across a file

## Line Number Reference

All line-based operations use **1-indexed** line numbers. Line 1 is the first line of the file.

When the user asks to edit "line 5", they mean the 5th line — pass `5` as the line number.

## Workflow Example

A typical multi-step editing session:

1. `edit_open` the file
2. Make changes with `edit_insert`, `edit_delete_lines`, `edit_replace_lines`, or `edit_replace_text`
3. Use `edit_diff` to review changes
4. `edit_save` to write to disk
5. `edit_close` to release the buffer

If something goes wrong during editing, use `edit_undo` to step back.

## Error Handling

- **File not found**: Report the error and suggest checking the path.
- **File already open**: Close the current file first, or remind the user which file is open.
- **Line out of range**: Report the valid range and ask the user to adjust.
- **No unsaved changes**: If `edit_save` is called with no changes, report that the file is already up to date.
- **Undo/redo exhausted**: Report that there are no more operations to undo or redo.
