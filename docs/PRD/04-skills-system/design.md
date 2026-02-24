# 04 — Skills System: Design

> **Visual styling follows [theme.design.md](../theme.design.md).** All colors, typography, spacing, and component styles are defined in the central theme document. This module must not define its own color palette or override theme tokens.

**Module:** Skills System (L2)
**Phase:** 1 — MVP
**Status:** Draft

---

## 1. Skill Bundle Structure

Each skill is a directory containing its manifest, prompt, tool definitions, and optional assets:

```
skills/
  built-in/
    package-manager/
      skill.yaml              # Manifest (required)
      prompt.md               # System prompt fragment (required)
      tools/
        install.json          # Tool definition
        remove.json
        search.json
        update.json
        list.json
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

At install time, built-in skills are placed under `/usr/share/levsha/skills/`. During development, they live in the repository at `skills/built-in/`.

---

## 2. Manifest Schema

The manifest (`skill.yaml`) is the entry point for every skill. The loader reads this file first, then resolves all referenced paths relative to the skill directory.

```yaml
# skill.yaml — full schema
name: string            # Required. Unique identifier (lowercase, hyphenated).
version: string         # Required. Semantic version "major.minor.patch".
description: string     # Required. One-line human-readable summary.
author: string          # Required. Author name or organization.
builtin: bool           # Optional. Default: false. If true, cannot be removed.

prompt: string          # Required. Relative path to system prompt fragment.

tools:                  # Required. List of relative paths to tool JSON files.
  - string

requires:               # Optional. System-level dependencies.
  packages:             # Optional. List of dnf package names.
    - string

assets:                 # Optional. List of relative paths to asset files/dirs.
  - string
```

### Validation Rules

- `name` must match `^[a-z][a-z0-9-]*$` (lowercase, hyphens, starts with letter).
- `version` must match `^\d+\.\d+\.\d+$`.
- `prompt` path must resolve to an existing `.md` file.
- Each entry in `tools` must resolve to an existing `.json` file.
- Each entry in `requires.packages` must be a valid package name string.

---

## 3. System Prompt Fragment Format

The prompt file is a Markdown document injected into the LLM's system prompt when the skill is active. It tells the LLM what the skill can do and how to behave.

### Structure

```markdown
# Package Manager

You can manage system packages using the tools below.

## Capabilities

- Install packages by name
- Remove installed packages
- Search the package repository
- Update all installed packages
- List currently installed packages

## Behavior Guidelines

- Always confirm before removing packages.
- When the user asks to install something vague, search first and present options.
- Report package versions after installation.
- For update operations, summarize what was updated.
```

### Rules

- The prompt fragment is plain Markdown. No frontmatter, no templating.
- It should be concise — every token counts against the context window.
- It should describe capabilities and behavioral guidelines, not implementation details.
- The intelligence engine composes the final system prompt by concatenating a base system prompt with all active skill prompt fragments.

---

## 4. Tool Definition Format

Each tool is a JSON file following the Anthropic function-calling schema. The intelligence engine sends these definitions to the Claude API as available tools.

### Schema

```json
{
  "name": "install_package",
  "description": "Install one or more system packages using dnf.",
  "input_schema": {
    "type": "object",
    "properties": {
      "packages": {
        "type": "array",
        "items": { "type": "string" },
        "description": "List of package names to install."
      }
    },
    "required": ["packages"]
  }
}
```

### Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | Yes | Unique tool name. Snake_case. Must be unique across all loaded skills. |
| `description` | string | Yes | What the tool does. The LLM uses this to decide when to call it. |
| `input_schema` | object | Yes | JSON Schema defining the tool's input parameters. |

### Rules

- Tool names must be globally unique across all skills. The loader rejects duplicates.
- The `input_schema` must be a valid JSON Schema (draft 2020-12 or compatible subset).
- Tool names should use `snake_case` and be descriptive (e.g., `install_package`, not `install`).
- Descriptions should be clear and specific — the LLM relies on them for dispatch.

---

## 5. Skill Loader

The skill loader runs once at engine startup. It discovers, validates, and registers all skills.

### Discovery

1. Scan the skill directory (`/usr/share/levsha/skills/`) for subdirectories.
2. In each subdirectory, look for `skill.yaml`.
3. Directories without `skill.yaml` are ignored with a warning log.

### Loading Sequence

```
For each skill directory:
  1. Parse skill.yaml
  2. Validate manifest fields (name, version, prompt path, tool paths)
  3. Read the prompt fragment file
  4. Parse each tool definition JSON file
  5. Validate tool name uniqueness across all skills
  6. Check system dependencies (required packages)
  7. Register skill in the skill registry
```

### Error Handling

| Error | Behavior |
|---|---|
| Missing `skill.yaml` | Skip directory, log warning. |
| Invalid YAML syntax | Skip skill, log error with file path and parse error. |
| Missing prompt file | Skip skill, log error. |
| Missing tool JSON file | Skip skill, log error. |
| Invalid tool JSON schema | Skip skill, log error. |
| Duplicate tool name | Skip the later skill, log error. First-loaded wins. |
| Missing dependency package | Log warning, attempt to install (see section 7). |

A malformed skill never crashes the engine. The engine must boot and function even if zero skills load successfully.

---

## 6. Skill Activation

In the MVP, all loaded skills are always active. There is no dynamic activation or deactivation based on user intent. This simplifies the architecture and is sufficient for two built-in skills.

### Prompt Composition

The intelligence engine builds the final system prompt as:

```
[Base System Prompt]

---

[Skill: package-manager]
{contents of package-manager/prompt.md}

---

[Skill: system-info]
{contents of system-info/prompt.md}
```

Each skill's prompt fragment is clearly delimited so the LLM can distinguish between base instructions and skill-specific context.

### Tool Registration

All tool definitions from all loaded skills are collected into a single list and passed to the Claude API in the `tools` parameter of every request. The API dispatches tool calls based on the LLM's response.

### Context Budget

With two built-in skills, the combined prompt and tool definitions should remain well under 4,000 tokens. If more skills are added in the future, the engine will need a context budget strategy (Phase 2 concern).

---

## 7. Dependency Resolution

When a skill declares `requires.packages`, the loader checks whether those packages are installed and attempts to install missing ones.

### Flow

```
For each required package:
  1. Run: dnf list installed <package> (or rpm -q <package>)
  2. If installed: continue
  3. If missing: run dnf install -y <package>
  4. If install fails: log error, mark skill as degraded
```

### Rules

- Dependency installation runs at engine startup, before the chat becomes interactive.
- Installation failures do not prevent the engine from starting.
- A skill with unmet dependencies is still loaded but logged as degraded.
- In the MVP, built-in skills depend only on packages already present in the base image, so dependency installation is a safety net, not a primary path.

---

## 8. Skill-to-Engine Interface

The skill system exposes a clean interface to the intelligence engine:

```
SkillRegistry
  .load_all(skill_dir: Path) -> Result<()>
  .get_all_prompts() -> Vec<SkillPrompt>
  .get_all_tools() -> Vec<ToolDefinition>
  .get_skill(name: &str) -> Option<&Skill>
  .loaded_count() -> usize
```

The intelligence engine calls `load_all` once at startup, then calls `get_all_prompts` and `get_all_tools` when composing each API request. The registry is read-only after loading.
