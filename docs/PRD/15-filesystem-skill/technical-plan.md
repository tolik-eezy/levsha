# 15 — Filesystem Skill: Technical Plan

**Module:** Filesystem Skill
**Language:** Rust (engine tools) + YAML/JSON (skill definition)
**Phase:** 2

---

## 1. Skill Structure

The filesystem skill is a standard skill definition — prompt fragments and tool JSON schemas. All tool execution goes through the engine's existing tool executor (shell commands).

```
skills/
  built-in/
    filesystem/
      skill.yaml
      prompts/
        filesystem.md
      tools/
        fs_list.json
        fs_tree.json
        fs_find.json
        fs_size.json
        fs_copy.json
        fs_move.json
        fs_delete.json
        fs_mkdir.json
        fs_chmod.json
        fs_read.json
        fs_head.json
        fs_tail.json
        fs_write.json
        fs_append.json
        fs_replace.json
```

---

## 2. Tool Definitions

### fs_list

```json
{
  "name": "fs_list",
  "description": "List directory contents with details (permissions, size, modification time).",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Directory path to list"
      },
      "all": {
        "type": "boolean",
        "description": "Include hidden files (default: false)"
      }
    },
    "required": ["path"]
  }
}
```

**Command template:** `ls -la --color=never {{path}}`

### fs_read

```json
{
  "name": "fs_read",
  "description": "Read file contents. For files over 30 lines, content opens in the split-view panel.",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path to read"
      }
    },
    "required": ["path"]
  }
}
```

**Command template:** `cat {{path}}`

### fs_delete

```json
{
  "name": "fs_delete",
  "description": "Delete a file or directory. This is a destructive operation that requires user confirmation.",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Path to delete"
      },
      "recursive": {
        "type": "boolean",
        "description": "Delete directories recursively (default: false)"
      }
    },
    "required": ["path"]
  }
}
```

**Command template:** `rm -rf {{path}}` (when recursive) or `rm {{path}}`

### fs_write

```json
{
  "name": "fs_write",
  "description": "Write content to a file. Creates the file if it doesn't exist, overwrites if it does.",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path to write to"
      },
      "content": {
        "type": "string",
        "description": "Content to write"
      }
    },
    "required": ["path", "content"]
  }
}
```

### fs_replace

```json
{
  "name": "fs_replace",
  "description": "Find and replace text in a file. Supports regex patterns.",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "File path"
      },
      "find": {
        "type": "string",
        "description": "Text or regex pattern to find"
      },
      "replace": {
        "type": "string",
        "description": "Replacement text"
      },
      "regex": {
        "type": "boolean",
        "description": "Treat 'find' as a regex pattern (default: false)"
      }
    },
    "required": ["path", "find", "replace"]
  }
}
```

**Command template:** `sed -i 's/{{find}}/{{replace}}/g' {{path}}`

---

## 3. Engine-Side Tool Handlers

Some filesystem tools need custom handlers beyond simple shell commands.

```rust
// In engine/src/tools/filesystem.rs

pub fn handle_fs_read(input: &serde_json::Value) -> Result<ToolOutput> {
    let path = input["path"].as_str().ok_or("missing path")?;
    let full_path = Path::new(path);

    // Binary detection
    if is_binary_file(full_path)? {
        let size = std::fs::metadata(full_path)?.len();
        let file_type = detect_file_type(full_path)?;
        return Ok(ToolOutput::text(
            format!("Binary file: {} ({}, {})", path, file_type, format_size(size))
        ));
    }

    let content = std::fs::read_to_string(full_path)?;
    let line_count = content.lines().count();

    if line_count > 30 {
        // Send to split-view panel
        Ok(ToolOutput::content_panel(ContentPayload::File {
            path: path.to_string(),
            content,
            language: detect_language(full_path),
        }))
    } else {
        Ok(ToolOutput::text(content))
    }
}

pub fn handle_fs_write(input: &serde_json::Value) -> Result<ToolOutput> {
    let path = input["path"].as_str().ok_or("missing path")?;
    let content = input["content"].as_str().ok_or("missing content")?;

    // Check if file exists for overwrite confirmation
    if Path::new(path).exists() {
        return Ok(ToolOutput::confirm(ConfirmRequest {
            command: format!("write {} bytes to {}", content.len(), path),
            reason: "File already exists. Overwrite?".into(),
            risk: "destructive".into(),
        }));
    }

    std::fs::write(path, content)?;
    Ok(ToolOutput::text(format!("Written {} bytes to {}", content.len(), path)))
}

fn is_binary_file(path: &Path) -> Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; 8192];
    let n = file.read(&mut buffer)?;
    Ok(buffer[..n].contains(&0))
}

fn detect_language(path: &Path) -> Option<String> {
    match path.extension()?.to_str()? {
        "rs" => Some("rust".into()),
        "py" => Some("python".into()),
        "js" => Some("javascript".into()),
        "ts" => Some("typescript".into()),
        "toml" => Some("toml".into()),
        "yaml" | "yml" => Some("yaml".into()),
        "json" => Some("json".into()),
        "sh" | "bash" => Some("bash".into()),
        "md" => Some("markdown".into()),
        "css" => Some("css".into()),
        "html" => Some("html".into()),
        _ => None,
    }
}
```

---

## 4. Skill Prompt

```markdown
## Filesystem Skill

You can manage files and directories using these tools:

### Browsing
- `fs_list` — List directory contents (like `ls -la`)
- `fs_tree` — Show directory tree structure
- `fs_find` — Find files by name or pattern
- `fs_size` — Show file or directory size

### File Operations
- `fs_copy` — Copy files or directories
- `fs_move` — Move or rename files
- `fs_delete` — Delete files or directories (requires confirmation)
- `fs_mkdir` — Create directories
- `fs_chmod` — Change file permissions

### File Content
- `fs_read` — Read file contents (opens in panel for large files)
- `fs_head` — Read first N lines
- `fs_tail` — Read last N lines
- `fs_write` — Write content to a file
- `fs_append` — Append content to a file
- `fs_replace` — Find and replace text in a file

### Guidelines
- Always show the result of file operations.
- For large directories, use `fs_tree` with a depth limit.
- Deletion and overwrite operations require user confirmation.
- Detect binary files and avoid displaying them as text.
- For files over 30 lines, use the split-view panel.
```

---

## 5. Implementation Stages

**Stage 1 — Tool Definitions (0.5 day)**
1. Create all 15 tool JSON schemas.
2. Create skill.yaml manifest.
3. Write prompt fragment.

**Stage 2 — Browsing Tools (0.5 day)**
1. fs_list, fs_tree, fs_find, fs_size.
2. Command templates with parameter substitution.
3. Output formatting.

**Stage 3 — File Operations (1 day)**
1. fs_copy, fs_move, fs_delete, fs_mkdir, fs_chmod.
2. Destructive guard integration for delete.
3. Overwrite detection for move.

**Stage 4 — File Content (1 day)**
1. fs_read with binary detection and split-view integration.
2. fs_head, fs_tail.
3. fs_write, fs_append, fs_replace with confirmation.

**Stage 5 — Testing (0.5 day)**
1. Tool execution tests in tmpdir.
2. Binary detection tests.
3. Destructive guard integration tests.

---

## 6. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| Tool JSON schemas | Valid JSON Schema format, required fields present |
| Binary detection | Correctly identifies ELF, images, vs. text files |
| Language detection | File extension to language mapping |
| Command templates | Parameter substitution correctness |

### Integration Tests

| Test | Method |
|------|--------|
| List directory | Create test dir with files, run fs_list, verify output |
| Read text file | Create text file, run fs_read, verify content |
| Read binary file | Create binary, run fs_read, verify binary detection |
| Delete with confirmation | Run fs_delete, verify confirmation triggered |
| Write new file | Run fs_write, verify file created |
| Write existing file | Run fs_write on existing, verify overwrite prompt |
| Find files | Create test tree, run fs_find with pattern, verify results |
| Large file → panel | Create 50-line file, verify split-view triggered |
