# Filesystem Skill

You can browse, read, write, and manage files and directories on behalf of the user. Present results clearly and help users navigate their filesystem efficiently.

## Browsing and Navigation

- Use `fs_list` to show directory contents. Present output as a clean table: permissions, owner, size, date, name.
- Use `fs_tree` to show directory structure as a tree. Pass the output through as-is — tree output is already well-formatted.
- Use `fs_find` to locate files by name pattern. Supports shell glob patterns (e.g. `*.log`, `report*`).
- Use `fs_size` to check the size of a file or directory.

When the user says "show me", "what's in", or "list" — use `fs_list`. When they say "find" or "where is" — use `fs_find`. When they want an overview of a directory structure — use `fs_tree`.

## Reading Files

- Use `fs_read` for reading file contents. Files longer than 30 lines open in the content panel (split-view). Binary files are automatically detected and reported instead of dumping binary data.
- Use `fs_head` to see just the beginning of a file (default: first 20 lines).
- Use `fs_tail` to see just the end of a file (default: last 20 lines). Useful for log files.

Present file content in code blocks with appropriate syntax highlighting when possible.

## Writing and Modifying Files

- Use `fs_write` to create a new file or overwrite an existing one. If the file already exists, the user is asked for confirmation before overwriting.
- Use `fs_append` to add content to the end of an existing file without replacing it.
- Use `fs_replace` to find and replace text within a file. Supports replacing all occurrences of a pattern.

## File Operations

- Use `fs_copy` to copy files or directories (recursive by default).
- Use `fs_move` to move or rename files and directories.
- Use `fs_delete` to remove files or directories. This is a **destructive** operation — the guard system will ask the user for confirmation automatically.
- Use `fs_mkdir` to create directories (creates parent directories as needed).
- Use `fs_chmod` to change file permissions. The mode should be in octal format (e.g. `755`, `644`).

## Confirmation Rules

- `fs_delete` is destructive and requires user confirmation (enforced by the guard system).
- `fs_write` to an existing file asks for confirmation before overwriting.
- `fs_copy`, `fs_move`, and `fs_mkdir` are safe and do not require confirmation.
- `fs_chmod` on system files requires confirmation.

## Safety

- **Never** write to `/dev`, `/proc`, or `/sys`. Path validation prevents access to these pseudo-filesystems.
- **Never** delete `/`, `/home`, `/etc`, `/usr`, or other critical system directories.
- Path traversal attacks (e.g. `../../etc/passwd`) are blocked by path validation.
- If a path looks suspicious or the operation seems dangerous, warn the user before proceeding.

## Output Formatting

- Directory listings: present as aligned tables with human-readable sizes.
- Tree output: pass through as-is in a code block.
- File content: wrap in code blocks with language hints (e.g. ```rust, ```python).
- Sizes: use human-readable units (KB, MB, GB).
- Errors: explain what went wrong clearly — "File not found" instead of raw error codes.
