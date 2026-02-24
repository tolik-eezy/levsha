# 18 — Code Editor Skill: Technical Plan

**Module:** Code Editor Skill
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

The code editor extends the text editor with project-level tools. It shares Git tools with the self-improvement module.

```
engine/
  src/
    code_editor/
      mod.rs              # Project management, skill coordination
      project.rs          # Project detection and navigation
      symbols.rs          # Symbol search (grep-based)
      builder.rs          # Build/run/test orchestration
      git.rs              # Git operations (shared with self-improvement)

skills/
  built-in/
    code-editor/
      skill.yaml
      prompts/
        code-editor.md
      tools/
        code_open_project.json
        code_find_definition.json
        code_find_references.json
        code_project_tree.json
        code_search.json
        code_build.json
        code_run.json
        code_test.json
        git_status.json
        git_diff.json
        git_add.json
        git_commit.json
        git_log.json
        git_branch.json
        git_push.json
        git_pull.json
```

### Dependencies

```toml
# Shared with self-improvement module
git2 = "0.19"          # Git operations
```

---

## 2. Project Detection

```rust
pub struct Project {
    pub root: PathBuf,
    pub project_type: ProjectType,
    pub name: String,
}

pub enum ProjectType {
    RustWorkspace { members: Vec<String> },
    RustPackage,
    NodeJs,
    Python,
    Make,
    Generic,
}

impl Project {
    pub fn detect(path: &Path) -> Result<Self> {
        let root = path.to_path_buf();

        // Check for Cargo workspace
        let cargo_toml = root.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = std::fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                let members = parse_workspace_members(&content);
                return Ok(Self {
                    root,
                    project_type: ProjectType::RustWorkspace { members },
                    name: path.file_name().unwrap().to_string_lossy().into(),
                });
            }
            return Ok(Self {
                root,
                project_type: ProjectType::RustPackage,
                name: path.file_name().unwrap().to_string_lossy().into(),
            });
        }

        // Check for package.json
        if root.join("package.json").exists() {
            return Ok(Self {
                root,
                project_type: ProjectType::NodeJs,
                name: path.file_name().unwrap().to_string_lossy().into(),
            });
        }

        // Check for Python project
        if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
            return Ok(Self {
                root,
                project_type: ProjectType::Python,
                name: path.file_name().unwrap().to_string_lossy().into(),
            });
        }

        // Check for Makefile
        if root.join("Makefile").exists() {
            return Ok(Self {
                root,
                project_type: ProjectType::Make,
                name: path.file_name().unwrap().to_string_lossy().into(),
            });
        }

        Ok(Self {
            root,
            project_type: ProjectType::Generic,
            name: path.file_name().unwrap().to_string_lossy().into(),
        })
    }

    pub fn build_command(&self) -> &str {
        match &self.project_type {
            ProjectType::RustWorkspace { .. } | ProjectType::RustPackage => "cargo build",
            ProjectType::NodeJs => "npm run build",
            ProjectType::Python => "python -m build",
            ProjectType::Make => "make",
            ProjectType::Generic => "make",
        }
    }

    pub fn test_command(&self) -> &str {
        match &self.project_type {
            ProjectType::RustWorkspace { .. } | ProjectType::RustPackage => "cargo test",
            ProjectType::NodeJs => "npm test",
            ProjectType::Python => "pytest",
            ProjectType::Make => "make test",
            ProjectType::Generic => "make test",
        }
    }

    pub fn run_command(&self) -> &str {
        match &self.project_type {
            ProjectType::RustWorkspace { .. } | ProjectType::RustPackage => "cargo run",
            ProjectType::NodeJs => "npm start",
            ProjectType::Python => "python main.py",
            ProjectType::Make => "make run",
            ProjectType::Generic => "./run.sh",
        }
    }
}
```

---

## 3. Symbol Search (Grep-Based)

```rust
pub struct SymbolSearcher {
    project_root: PathBuf,
}

pub struct SymbolMatch {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub context: String,
    pub match_type: MatchType,
}

pub enum MatchType {
    Definition,
    Reference,
}

impl SymbolSearcher {
    /// Find where a symbol is defined (struct, fn, const, let, class, def, etc.)
    pub fn find_definition(&self, symbol: &str) -> Result<Vec<SymbolMatch>> {
        // Patterns for common definition syntaxes
        let patterns = vec![
            format!(r"(pub\s+)?(struct|enum|trait|fn|const|static|type|mod)\s+{}\b", regex::escape(symbol)),
            format!(r"(class|def|function|const|let|var)\s+{}\b", regex::escape(symbol)),
            format!(r"(interface|type)\s+{}\b", regex::escape(symbol)),
        ];

        let mut results = Vec::new();
        for pattern in &patterns {
            let output = std::process::Command::new("rg")
                .args(["--json", "--no-heading", pattern, &self.project_root.to_string_lossy()])
                .output()?;
            results.extend(parse_rg_matches(&output.stdout, MatchType::Definition)?);
        }
        Ok(results)
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol: &str) -> Result<Vec<SymbolMatch>> {
        let pattern = format!(r"\b{}\b", regex::escape(symbol));
        let output = std::process::Command::new("rg")
            .args(["--json", "--no-heading", &pattern, &self.project_root.to_string_lossy()])
            .output()?;
        parse_rg_matches(&output.stdout, MatchType::Reference)
    }

    /// Search code by pattern across the project
    pub fn search(&self, pattern: &str, file_glob: Option<&str>) -> Result<Vec<SymbolMatch>> {
        let mut cmd = std::process::Command::new("rg");
        cmd.args(["--json", "--no-heading", pattern]);
        if let Some(glob) = file_glob {
            cmd.args(["--glob", glob]);
        }
        cmd.arg(&self.project_root.to_string_lossy().as_ref());
        let output = cmd.output()?;
        parse_rg_matches(&output.stdout, MatchType::Reference)
    }
}
```

---

## 4. Build/Run/Test Orchestration

```rust
pub struct CodeBuilder {
    project: Project,
}

impl CodeBuilder {
    pub async fn build(
        &self,
        progress_tx: mpsc::Sender<BuildProgress>,
    ) -> Result<BuildResult> {
        let cmd = self.project.build_command();
        self.run_with_progress(cmd, progress_tx).await
    }

    pub async fn test(
        &self,
        progress_tx: mpsc::Sender<BuildProgress>,
    ) -> Result<TestResult> {
        let cmd = self.project.test_command();
        let output = self.run_with_progress(cmd, progress_tx).await?;

        // Parse test output based on project type
        match &self.project.project_type {
            ProjectType::RustWorkspace { .. } | ProjectType::RustPackage => {
                parse_cargo_test_output(&output.stderr)
            }
            _ => Ok(TestResult::from_exit_code(output.success, &output.stderr))
        }
    }

    pub async fn run(
        &self,
        args: Option<&str>,
        progress_tx: mpsc::Sender<BuildProgress>,
    ) -> Result<RunResult> {
        let mut cmd = self.project.run_command().to_string();
        if let Some(extra) = args {
            cmd.push(' ');
            cmd.push_str(extra);
        }
        let output = self.run_with_progress(&cmd, progress_tx).await?;
        Ok(RunResult {
            success: output.success,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    async fn run_with_progress(
        &self,
        command: &str,
        progress_tx: mpsc::Sender<BuildProgress>,
    ) -> Result<CommandOutput> {
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.project.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Stream stderr (where cargo outputs progress)
        let stderr = child.stderr.take().unwrap();
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut output = String::new();

        while let Some(line) = lines.next_line().await? {
            output.push_str(&line);
            output.push('\n');
            if let Some(progress) = parse_build_line(&line) {
                let _ = progress_tx.send(progress).await;
            }
        }

        let status = child.wait().await?;
        let stdout = if let Some(mut out) = child.stdout {
            let mut s = String::new();
            out.read_to_string(&mut s).await?;
            s
        } else {
            String::new()
        };

        Ok(CommandOutput {
            success: status.success(),
            stdout,
            stderr: output,
        })
    }
}
```

---

## 5. Git Operations

Git operations are shared with the self-improvement module (module 10). The code editor exposes them as skill tools.

```rust
// Reuse engine/src/self_improve/git.rs
// Additional wrappers for code editor tools:

pub fn handle_git_status(project: &Project) -> Result<ToolOutput> {
    let output = std::process::Command::new("git")
        .args(["status", "--porcelain", "-b"])
        .current_dir(&project.root)
        .output()?;

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(ToolOutput::text(format_git_status(&text)))
}

pub fn handle_git_diff(project: &Project, staged: bool) -> Result<ToolOutput> {
    let mut args = vec!["diff"];
    if staged { args.push("--cached"); }

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(&project.root)
        .output()?;

    let diff = String::from_utf8_lossy(&output.stdout).to_string();
    if diff.is_empty() {
        Ok(ToolOutput::text("No changes to show."))
    } else {
        Ok(ToolOutput::content_panel(ContentPayload::Diff {
            diff_text: diff,
            file_path: None,
        }))
    }
}

pub fn handle_git_add(project: &Project, files: &[String]) -> Result<ToolOutput> {
    let mut cmd = std::process::Command::new("git");
    cmd.arg("add").current_dir(&project.root);
    for file in files {
        cmd.arg(file);
    }
    let output = cmd.output()?;
    if output.status.success() {
        Ok(ToolOutput::text(format!("Staged {} file(s)", files.len())))
    } else {
        Ok(ToolOutput::error(String::from_utf8_lossy(&output.stderr).to_string()))
    }
}

pub fn handle_git_commit(project: &Project, message: &str) -> Result<ToolOutput> {
    let output = std::process::Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(&project.root)
        .output()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(ToolOutput::text(format!("Committed: {}", text.lines().next().unwrap_or(""))))
    } else {
        Ok(ToolOutput::error(String::from_utf8_lossy(&output.stderr).to_string()))
    }
}
```

---

## 6. Implementation Stages

**Stage 1 — Project Detection (0.5 day)**
1. Implement `Project::detect()` for Rust, Node, Python, Make.
2. Build/test/run command resolution.
3. Project tree display.

**Stage 2 — Symbol Search (1 day)**
1. `SymbolSearcher` with ripgrep integration.
2. Definition patterns for Rust, Python, JS/TS.
3. Reference search.
4. Code search with glob filtering.

**Stage 3 — Build/Run/Test (1-2 days)**
1. `CodeBuilder` with progress streaming.
2. Cargo output parsing.
3. Test result parsing.
4. Build output → split-view panel integration.

**Stage 4 — Git Tools (1 day)**
1. Git status, diff, add, commit, log.
2. Branch management (list, create, switch).
3. Push/pull with authentication handling.
4. Diff rendering in split-view panel.

**Stage 5 — Tool Integration (1 day)**
1. All 16 tool JSON schemas.
2. Skill manifest and prompt fragment.
3. Wire to engine tool registry.
4. End-to-end testing.

---

## 7. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `project.rs` | Project type detection for Rust, Node, Python, Make, generic |
| `symbols.rs` | Definition patterns match correctly, reference search |
| `builder.rs` | Cargo output parsing, test result parsing |

### Integration Tests

| Test | Method |
|------|--------|
| Open Rust project | Create test Cargo workspace, detect type, verify commands |
| Find definition | Create test file with struct, find definition, verify location |
| Find references | Create test files with symbol usage, verify all found |
| Build project | Create minimal Rust project, build, verify success |
| Build failure | Create project with syntax error, build, verify error output |
| Run tests | Create project with tests, run, verify results parsed |
| Git status | Init repo with changes, run status, verify output |
| Git diff → panel | Make changes, run diff, verify split-view content |
| Git commit | Stage changes, commit, verify commit created |

### Cross-Module Tests

| Test | Description |
|------|-------------|
| Code editor + text editor | Open file via code editor, edit via text editor tools |
| Code editor + self-improvement | Git tools work in both contexts |
| Code editor + split-view | Build output and diffs render correctly in panel |
