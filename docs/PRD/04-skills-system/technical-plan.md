# 04 — Skills System: Technical Plan

**Module:** Skills System (L2)
**Phase:** 1 — MVP
**Status:** Draft

---

## 1. Directory Structure

### Runtime Layout

```
/usr/share/levsha/
  skills/
    package-manager/
      skill.yaml
      prompt.md
      tools/
        install_package.json
        remove_package.json
        search_packages.json
        update_packages.json
        list_packages.json
    system-info/
      skill.yaml
      prompt.md
      tools/
        disk_usage.json
        memory_usage.json
        cpu_info.json
        uptime.json
        network_status.json
        list_processes.json
```

### Repository Layout

```
Levsha.OS/
  skills/
    built-in/
      package-manager/
        skill.yaml
        prompt.md
        tools/
          install_package.json
          remove_package.json
          search_packages.json
          update_packages.json
          list_packages.json
      system-info/
        skill.yaml
        prompt.md
        tools/
          disk_usage.json
          memory_usage.json
          cpu_info.json
          uptime.json
          network_status.json
          list_processes.json
  engine/
    src/
      skills/
        mod.rs           # Module root, re-exports
        manifest.rs      # Manifest parsing and validation
        loader.rs        # Skill discovery and loading
        registry.rs      # Skill registry (read-only after init)
        tool_schema.rs   # Tool definition types
        prompt.rs        # Prompt fragment handling
```

---

## 2. Manifest Parsing

Use `serde` with `serde_yaml` to deserialize `skill.yaml` into a Rust struct.

### Rust Types

```rust
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub builtin: bool,
    pub prompt: PathBuf,
    pub tools: Vec<PathBuf>,
    #[serde(default)]
    pub requires: SkillRequires,
    #[serde(default)]
    pub assets: Vec<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SkillRequires {
    #[serde(default)]
    pub packages: Vec<String>,
}
```

### Validation

After deserialization, validate:

- `name` matches `^[a-z][a-z0-9-]*$`.
- `version` matches `^\d+\.\d+\.\d+$`.
- `prompt` path exists relative to the skill directory.
- Each `tools` entry exists relative to the skill directory.

Validation errors produce structured error types, not panics.

---

## 3. Tool Schema Format

Tool definitions use JSON files compatible with the Anthropic tool use format.

### Rust Types

```rust
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
```

### Example Tool File

```json
{
  "name": "install_package",
  "description": "Install one or more system packages using dnf. Returns installation output.",
  "input_schema": {
    "type": "object",
    "properties": {
      "packages": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Package names to install."
      }
    },
    "required": ["packages"]
  }
}
```

The `ToolDefinition` struct serializes directly into the `tools` array of an Anthropic API request. No transformation needed.

---

## 4. Skill Registry

The registry is the central store of all loaded skills. It is populated once at startup and then treated as read-only.

### Interface

```rust
pub struct SkillRegistry {
    skills: Vec<Skill>,
}

pub struct Skill {
    pub manifest: SkillManifest,
    pub prompt_content: String,
    pub tools: Vec<ToolDefinition>,
    pub skill_dir: PathBuf,
}

impl SkillRegistry {
    /// Scan directory, load all valid skills.
    pub fn load_all(skill_dir: &Path) -> Result<Self, SkillLoadError>;

    /// Get concatenated prompt fragments for all loaded skills.
    pub fn get_all_prompts(&self) -> Vec<SkillPrompt>;

    /// Get all tool definitions from all loaded skills.
    pub fn get_all_tools(&self) -> Vec<&ToolDefinition>;

    /// Look up a skill by name.
    pub fn get_skill(&self, name: &str) -> Option<&Skill>;

    /// Number of successfully loaded skills.
    pub fn loaded_count(&self) -> usize;
}

pub struct SkillPrompt {
    pub skill_name: String,
    pub content: String,
}
```

### Loading Algorithm

```
fn load_all(skill_dir: &Path) -> Result<SkillRegistry> {
    let mut skills = Vec::new();
    let mut tool_names: HashSet<String> = HashSet::new();

    for entry in read_dir(skill_dir)? {
        let manifest_path = entry.path().join("skill.yaml");
        if !manifest_path.exists() {
            warn!("No skill.yaml in {}, skipping", entry.path());
            continue;
        }

        match load_single_skill(&entry.path(), &mut tool_names) {
            Ok(skill) => {
                info!("Loaded skill: {} v{} ({} tools)",
                    skill.manifest.name,
                    skill.manifest.version,
                    skill.tools.len()
                );
                skills.push(skill);
            }
            Err(e) => {
                error!("Failed to load skill from {}: {}", entry.path(), e);
            }
        }
    }

    info!("Skill registry ready: {} skills, {} tools",
        skills.len(),
        skills.iter().map(|s| s.tools.len()).sum::<usize>()
    );

    Ok(SkillRegistry { skills })
}
```

---

## 5. Integration with Intelligence Engine

The intelligence engine consumes the skill registry at two points:

### System Prompt Composition

When building the system prompt for a Claude API request:

```rust
fn compose_system_prompt(
    base_prompt: &str,
    registry: &SkillRegistry,
) -> String {
    let mut prompt = base_prompt.to_string();

    for skill_prompt in registry.get_all_prompts() {
        prompt.push_str("\n\n---\n\n");
        prompt.push_str(&format!("## Skill: {}\n\n", skill_prompt.skill_name));
        prompt.push_str(&skill_prompt.content);
    }

    prompt
}
```

### Tool Registration

When building the `tools` array for a Claude API request:

```rust
fn collect_tools(registry: &SkillRegistry) -> Vec<serde_json::Value> {
    registry.get_all_tools()
        .iter()
        .map(|tool| {
            serde_json::json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.input_schema,
            })
        })
        .collect()
}
```

### Tool Execution Dispatch

When the LLM responds with a `tool_use` block, the engine must execute it. Tool execution is not part of the skill system itself — the engine owns execution. However, the skill system provides the metadata needed for dispatch:

1. Engine receives `tool_use` with a tool `name` and `input`.
2. Engine looks up the tool name in the registry to verify it exists.
3. Engine executes the corresponding system action (command execution, API call, etc.).
4. Engine returns the result to the LLM as a `tool_result`.

The mapping from tool name to actual execution logic lives in the engine, not in the skill bundle. Skills define *what* can be called; the engine defines *how* it runs.

---

## 6. Dependency Installation

At startup, after loading manifests but before marking skills as ready:

```rust
fn ensure_dependencies(manifest: &SkillManifest) -> Result<()> {
    for package in &manifest.requires.packages {
        if !is_package_installed(package)? {
            info!("Installing missing dependency: {}", package);
            let status = Command::new("dnf")
                .args(["install", "-y", package])
                .status()?;

            if !status.success() {
                warn!("Failed to install {}, skill may be degraded", package);
            }
        }
    }
    Ok(())
}

fn is_package_installed(name: &str) -> Result<bool> {
    let output = Command::new("rpm")
        .args(["-q", name])
        .output()?;
    Ok(output.status.success())
}
```

For the MVP, built-in skill dependencies are included in the base ISO image. Dependency installation is a safety net for development scenarios where the base image may be incomplete.

---

## 7. Built-In Skill Packaging

Built-in skills are bundled into the ISO during the build process.

### Build Integration

In the ISO build (kickstart or overlay):

```bash
# Copy built-in skills to the system image
mkdir -p /usr/share/levsha/skills/
cp -r skills/built-in/* /usr/share/levsha/skills/
```

### Filesystem Overlay

The `base/overlay/` directory includes:

```
base/
  overlay/
    usr/
      share/
        levsha/
          skills/
            package-manager/
              skill.yaml
              prompt.md
              tools/
                ...
            system-info/
              skill.yaml
              prompt.md
              tools/
                ...
```

This overlay is applied during ISO generation so skills are present on first boot.

---

## 8. Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum SkillLoadError {
    #[error("Failed to read manifest: {0}")]
    ManifestRead(#[from] std::io::Error),

    #[error("Invalid manifest YAML: {0}")]
    ManifestParse(#[from] serde_yaml::Error),

    #[error("Invalid skill name '{0}': must be lowercase alphanumeric with hyphens")]
    InvalidName(String),

    #[error("Invalid version '{0}': must be semver (major.minor.patch)")]
    InvalidVersion(String),

    #[error("Prompt file not found: {0}")]
    PromptNotFound(PathBuf),

    #[error("Tool file not found: {0}")]
    ToolNotFound(PathBuf),

    #[error("Invalid tool JSON: {0}")]
    ToolParse(String),

    #[error("Duplicate tool name '{0}' (already registered by skill '{1}')")]
    DuplicateToolName(String, String),
}
```

---

## 9. Crate Dependencies

| Crate | Purpose |
|---|---|
| `serde` + `serde_derive` | Struct serialization/deserialization. |
| `serde_yaml` | YAML manifest parsing. |
| `serde_json` | Tool definition parsing and API payload construction. |
| `thiserror` | Structured error types. |
| `tracing` | Structured logging (info, warn, error during loading). |
| `regex` | Name and version format validation. |

---

## 10. Testing Strategy

### Unit Tests

| Test | Validates |
|---|---|
| Parse valid manifest | `serde_yaml` deserialization with all fields. |
| Parse minimal manifest | Only required fields, defaults applied. |
| Reject invalid name | Names with uppercase, spaces, special chars. |
| Reject invalid version | Non-semver strings. |
| Parse valid tool definition | JSON with name, description, input_schema. |
| Reject tool with missing fields | Missing name or description. |
| Detect duplicate tool names | Two skills with same tool name. |

### Integration Tests

| Test | Validates |
|---|---|
| Load skills from test directory | Full loading pipeline with valid fixture skills. |
| Skip directories without manifest | Graceful handling of non-skill directories. |
| Skip skills with missing files | Missing prompt or tool files logged, not fatal. |
| Prompt composition | System prompt includes all skill fragments. |
| Tool collection | All tools from all skills are collected. |
| Dependency check | Verify `rpm -q` check for installed packages. |

### Test Fixtures

Create a `tests/fixtures/skills/` directory with:

```
tests/fixtures/skills/
  valid-skill/
    skill.yaml
    prompt.md
    tools/
      example_tool.json
  missing-prompt/
    skill.yaml           # References nonexistent prompt file
  invalid-yaml/
    skill.yaml           # Contains broken YAML
  duplicate-tool/
    skill.yaml
    prompt.md
    tools/
      example_tool.json  # Same tool name as valid-skill
```

### Acceptance Tests

- Engine starts with built-in skills loaded and logs the count.
- LLM receives skill prompts in system message.
- LLM can invoke skill tools and receive results.
- Engine boots successfully even if a skill is malformed.

### Cross-Module Integration Tests

These tests verify skills are correctly wired from filesystem to API request. See `00-system-architecture/integration-checks.md` for full details.

| Check | Seam | What it verifies |
|-------|------|------------------|
| IC-30 | Skills -> L2 -> API | All skill prompts appear in system message, all tool definitions in API tools array |
| IC-31 | API -> L2 -> Skill -> L1 | Tool call from API routed to correct skill, command executed, result returned |
| IC-32 | API -> L2 | Unknown tool name handled gracefully (error result, no crash) |
| IC-21 | Full chain | Package install works end-to-end through skill system |

---

## 11. Implementation Order

| Step | Task | Depends On |
|---|---|---|
| 1 | Define Rust types: `SkillManifest`, `ToolDefinition`, `Skill` | None |
| 2 | Implement manifest parsing and validation | Step 1 |
| 3 | Implement tool definition parsing | Step 1 |
| 4 | Implement skill loader (discovery + file reading) | Steps 2, 3 |
| 5 | Implement skill registry | Step 4 |
| 6 | Write unit tests for parsing and validation | Steps 2, 3 |
| 7 | Write built-in skill bundles (manifest, prompt, tools) | Steps 1, 2, 3 |
| 8 | Integrate registry with intelligence engine (prompt composition) | Step 5, engine exists |
| 9 | Integrate registry with intelligence engine (tool registration) | Step 5, engine exists |
| 10 | Write integration tests | Steps 7, 8, 9 |
| 11 | Add skill directory to ISO overlay | Step 7, ISO build exists |
