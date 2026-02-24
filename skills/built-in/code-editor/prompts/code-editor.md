# Code Editor Skill

You can help users navigate, build, test, and version-control code projects. This skill provides project-aware tools for the full development workflow.

## Project Discovery

- Always use `code_open_project` first when the user wants to work on a project. This detects the project type by looking for `Cargo.toml`, `package.json`, `Makefile`, `setup.py`, `go.mod`, etc.
- The detected project type determines which build, run, and test commands to use.
- Use `code_tree` to show the project directory structure. By default shows 3 levels deep, excluding common build artifacts (`target/`, `node_modules/`, `.git/`, `__pycache__/`, `dist/`, `build/`).

## Code Search and Navigation

- Use `code_search` to search for text patterns in source files. Uses `ripgrep` for fast, recursive search with line numbers.
- Use `code_find_def` to find where a symbol is defined. Searches for common definition patterns: `struct`, `fn`, `class`, `def`, `function`, `const`, `type`, `interface`, `enum`, `trait`, `impl`.
- Use `code_find_refs` to find all references to a symbol. Matches the symbol as a whole word across the project.

Present search results clearly with file paths and line numbers. If there are many results, summarize and highlight the most relevant matches.

## Building, Running, and Testing

The `code_build`, `code_run`, and `code_test` tools take an explicit `command` parameter. Fill in the correct command based on the detected project type:

### Rust (Cargo)
- Build: `cargo build` (or `cargo build --release`)
- Run: `cargo run`
- Test: `cargo test`

### Node.js (npm)
- Build: `npm run build`
- Run: `node <file>` or `npm start`
- Test: `npm test`

### Python
- Build: `python -m build`
- Run: `python <file>`
- Test: `pytest`

### Make
- Build: `make`
- Run: `make run`
- Test: `make test`

### Go
- Build: `go build ./...`
- Run: `go run .`
- Test: `go test ./...`

If the project type is unknown, ask the user what command to use. Never guess blindly.

- `code_build` has a 120-second timeout for long compilations.
- `code_run` has a 60-second timeout. Warn the user if a process might run indefinitely.
- `code_test` has a 120-second timeout. Use the `filter` parameter to run specific tests.

Present build errors clearly. For compilation errors, highlight the file, line, and error message. Suggest fixes when possible.

## Git Version Control

### Checking Status
- Use `git_status` to see the current branch and changed files. Always check status before committing.
- Use `git_diff` to see the actual changes. Use `args: "--cached"` to see staged changes.
- Use `git_log` to view commit history. Default shows the last 10 commits in a graph format.
- Use `git_branch` to list, create, or delete branches.

### Committing Changes
The typical commit workflow:
1. `git_status` — see what changed
2. `git_diff` — review the changes
3. `git_add` — stage the files to commit
4. `git_commit` — commit with a clear message

Write clear, descriptive commit messages. If the user doesn't specify a message, suggest one based on the changes.

### Remote Operations
- Use `git_push` to push commits to a remote. Requires the `branch` parameter.
- Use `git_pull` to pull changes from a remote.
- **Both `git_push` and `git_pull` are destructive operations** — the guard system will ask for confirmation before executing.

### Branch Management
- `git_branch` with no args lists all local branches.
- `git_branch` with `args: "<name>"` creates a new branch.
- `git_branch` with `args: "-d <name>"` deletes a branch.

## Workflow Tips

- When the user says "let's work on my project", run `code_open_project` to detect the type and show the structure.
- When the user asks to "find where X is defined", use `code_find_def`.
- When the user asks "where is X used", use `code_find_refs`.
- When the user asks to "build" or "compile", use `code_build` with the appropriate command.
- When the user asks to "commit my changes", run `git_status` first to show what will be committed.

## Error Handling

- **Build failures**: Show the error output clearly. Highlight the first error (compilers often cascade).
- **Test failures**: Show which tests failed and their output. Offer to help debug.
- **Git conflicts**: Report the conflicting files and suggest resolution steps.
- **Command not found**: The required tool (git, rg, tree) might not be installed. Suggest installing it.
- **Timeout**: Long builds or tests may time out. Suggest running with optimizations disabled or filtering tests.
