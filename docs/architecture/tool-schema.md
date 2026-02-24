# Tool Schema Format

**Version:** 0.1.0
**Status:** Phase 1 (MVP)

---

## 1. Overview

Levsha OS skills define tools as JSON files compatible with the [Anthropic tool use API](https://docs.anthropic.com/en/docs/build-with-claude/tool-use). Each tool maps to a concrete system action (shell command, binary invocation, or internal function).

Tools are defined as individual `.json` files inside a skill's `tools/` directory. The skill manifest (`skill.yaml`) references them by path.

---

## 2. Tool Definition Schema

Each `.json` file contains a single tool definition:

```json
{
  "name": "package_install",
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
  },
  "execution": {
    "type": "shell",
    "command_template": "sudo dnf install -y {{packages | join(' ')}}"
  }
}
```

### 2.1 Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Unique tool identifier. Snake_case. Must be unique across all loaded skills. |
| `description` | string | Yes | Human-readable description sent to the LLM. Describes what the tool does and when to use it. |
| `input_schema` | object | Yes | JSON Schema defining the tool's input parameters. Sent directly to the Anthropic API as `input_schema`. |
| `execution` | object | Yes | Defines how the engine executes this tool. Not sent to the API. |

### 2.2 Input Schema

Follows standard [JSON Schema](https://json-schema.org/) conventions. The Anthropic API uses this to guide the LLM in constructing valid tool calls.

Supported types: `string`, `number`, `integer`, `boolean`, `array`, `object`.

```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "Search query for package names."
    },
    "limit": {
      "type": "integer",
      "description": "Maximum number of results to return.",
      "default": 10
    }
  },
  "required": ["query"]
}
```

### 2.3 Execution Block

The `execution` block tells the engine how to run the tool. It is **not** sent to the Anthropic API.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | Execution type: `"shell"` (run a shell command) or `"internal"` (call an engine function). |
| `command_template` | string | Conditional | Shell command template with `{{param}}` placeholders. Required when `type` is `"shell"`. |
| `function` | string | Conditional | Engine function name. Required when `type` is `"internal"`. |
| `timeout_seconds` | integer | No | Override the default command timeout. |
| `run_as` | string | No | User to run the command as. Default: current user. Use `"root"` for privileged operations. |

### 2.4 Command Templates

Templates use `{{parameter_name}}` syntax for parameter substitution. The engine substitutes parameter values from the LLM's tool call input.

| Syntax | Meaning |
|--------|---------|
| `{{param}}` | Substitute the parameter value directly |
| `{{param \| join(' ')}}` | Join array elements with a separator |
| `{{param \| quote}}` | Shell-quote the value for safety |

---

## 3. Anthropic API Mapping

When building API requests, the engine maps tool definitions to the Anthropic format:

```json
{
  "tools": [
    {
      "name": "package_install",
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
  ]
}
```

Only `name`, `description`, and `input_schema` are sent. The `execution` block stays on the engine side.

---

## 4. Tool Call and Result Flow

When the API returns a `tool_use` content block:

```json
{
  "type": "tool_use",
  "id": "toolu_abc123",
  "name": "package_install",
  "input": {
    "packages": ["cowsay"]
  }
}
```

The engine:

1. Looks up the tool definition by `name`.
2. Validates the `input` against `input_schema`.
3. Checks the risk classifier for destructive patterns.
4. If destructive: sends `ConfirmRequest` to shell, waits for `ConfirmResponse`.
5. Renders the `command_template` with the input parameters.
6. Executes the command via `tokio::process::Command`.
7. Captures stdout, stderr, exit code.
8. Sends the result back to the API:

```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_abc123",
  "content": "Last metadata expiration check: ...\nInstalled: cowsay-3.04-19.fc41.noarch"
}
```

---

## 5. Skill Manifest Integration

Tools are referenced from the skill manifest (`skill.yaml`):

```yaml
name: package-manager
version: 0.1.0
description: "Install, remove, update, and search system packages"
author: levsha
builtin: true

prompt: prompts/package-manager.md

tools:
  - tools/install.json
  - tools/remove.json
  - tools/search.json
  - tools/update.json
  - tools/list.json

requires:
  packages:
    - dnf
```

The engine's skill loader reads the manifest, resolves tool paths relative to the skill directory, and loads each tool definition.

---

## 6. Example: System Info Tool

```json
{
  "name": "disk_usage",
  "description": "Show disk usage for all mounted filesystems. Use when the user asks about disk space, storage, or filesystem usage.",
  "input_schema": {
    "type": "object",
    "properties": {
      "path": {
        "type": "string",
        "description": "Optional: specific path to check. Omit for all filesystems."
      }
    },
    "required": []
  },
  "execution": {
    "type": "shell",
    "command_template": "df -h {{path | quote}}"
  }
}
```

---

## 7. Validation Rules

The engine validates tool definitions at load time:

1. `name` must be non-empty, snake_case, and unique across all skills.
2. `description` must be non-empty.
3. `input_schema` must be valid JSON Schema with `type: "object"`.
4. `execution.type` must be `"shell"` or `"internal"`.
5. For `"shell"` type, `command_template` must be present and non-empty.
6. For `"internal"` type, `function` must be present and map to a known engine function.
7. All `required` parameters in `input_schema` must have corresponding properties.

Invalid tool definitions produce a warning log and are skipped (the skill loads without that tool).
