# 04 — Skills System: Product Requirements

**Module:** Skills System (L2)
**Phase:** 1 — MVP
**Status:** Draft

---

## 1. Overview

In Levsha OS, software is not installed as standalone applications with their own windows and menus. Instead, software is packaged as **skills** — modular capability bundles that extend what the chat can do. The skill system is the framework that defines, loads, and activates these bundles.

A skill is the fundamental unit of functionality beyond general conversation. When a skill is active, its instructions and tools become part of what the LLM can do. The user never interacts with skills directly — they simply ask for things, and the intelligence engine activates the right skill context.

In the MVP, only **built-in skills** exist. They ship with the OS and cannot be added or removed. Skill install/remove from external repositories is Phase 2.

---

## 2. Core Concept: Skills, Not Apps

| Traditional OS | Levsha OS |
|---|---|
| Install an application | Acquire a skill |
| Application has its own window and UI | Skill extends the chat interface |
| Application runs as a separate process | Skill provides context and tools to the LLM |
| User launches apps from a menu | LLM activates skills based on user intent |
| Apps are sandboxed (sometimes) | Skills run with full system access |

A skill does not have its own process, window, or event loop. A skill is a set of instructions and callable functions that the intelligence engine injects into the LLM context when relevant.

---

## 3. Skill Components

Every skill is a bundle containing up to five components:

| Component | Description | Required |
|---|---|---|
| **Manifest** | Metadata: name, version, description, author, dependencies, file references. | Yes |
| **System Prompt Fragment** | Markdown instructions injected into the LLM context when the skill is active. Tells the LLM what the skill can do and how to use it. | Yes |
| **Tool Definitions** | JSON schemas for function-calling endpoints the LLM can invoke. Each tool maps to a concrete system action. | Yes |
| **Dependencies** | System packages or binaries the skill requires to function. Installed via `dnf`. | No |
| **Assets** | Static files, configs, templates, or data bundled with the skill. | No |

---

## 4. Skill Manifest Format

Skills are defined using a YAML manifest (`skill.yaml`):

```yaml
name: package-manager
version: 0.1.0
description: "Install, remove, update, and search system packages"
author: levsha
builtin: true

prompt: prompt.md

tools:
  - tools/install.json
  - tools/remove.json
  - tools/search.json
  - tools/update.json
  - tools/list.json

requires:
  packages:
    - dnf

assets: []
```

### Manifest Fields

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | Yes | Unique skill identifier. Lowercase, hyphenated. |
| `version` | string | Yes | Semantic version (major.minor.patch). |
| `description` | string | Yes | One-line human-readable description. |
| `author` | string | Yes | Author or organization name. |
| `builtin` | bool | No | If `true`, skill ships with the OS and cannot be removed. Default: `false`. |
| `prompt` | string | Yes | Relative path to the system prompt fragment file. |
| `tools` | list | Yes | Relative paths to tool definition JSON files. |
| `requires.packages` | list | No | System packages to install via `dnf`. |
| `assets` | list | No | Relative paths to asset files or directories. |

---

## 5. MVP Scope

### In Scope

- Skill manifest format definition (`skill.yaml`).
- System prompt fragment loading from Markdown files.
- Tool definition loading from JSON files (Anthropic function-calling compatible).
- Skill loader: discover and parse all built-in skills at engine startup.
- Skill activation: inject skill prompts and tools into the LLM context.
- Dependency resolution: ensure required system packages are installed.
- Two built-in skills: package manager and system info.

### Out of Scope (Phase 2+)

- Skill install/remove from external repositories.
- Skill marketplace or discovery.
- Skill versioning and updates.
- Skill conflicts resolution.
- Per-skill sandboxing or capability restrictions.
- Dynamic skill creation ("help me build a skill for...").
- Skill enable/disable at runtime.
- Hardware requirement checks (e.g., GPU).

---

## 6. Security Model

In the MVP, skills run with **full system access**. There is no sandbox, no permission model, and no capability restrictions. A skill's tools can execute any command the OS can execute.

This is acceptable because:

- Only built-in skills exist in the MVP — they are trusted by definition.
- The intelligence engine enforces destructive command confirmation regardless of which skill triggers it.
- Skill sandboxing is a Phase 2+ concern, relevant only when third-party skills are introduced.

---

## 7. Built-In Skills (MVP)

| Skill | Description | Detailed In |
|---|---|---|
| `package-manager` | Install, remove, update, search, and list system packages via `dnf`. | `05-package-manager-skill/` |
| `system-info` | Query disk, memory, CPU, uptime, network status, and running processes. | `06-system-info-skill/` |

Both skills are always active — their prompts and tools are always included in the LLM context.

---

## 8. Acceptance Criteria

| ID | Criterion | Priority |
|---|---|---|
| SK-01 | Skill manifest format is defined and documented as YAML schema. | P0 |
| SK-02 | Built-in skills are discovered and loaded automatically at engine startup. | P0 |
| SK-03 | Skill system prompt fragments are injected into the LLM context. | P0 |
| SK-04 | Skill tool definitions are registered with the intelligence engine for function calling. | P0 |
| SK-05 | Tool definitions are compatible with Anthropic's tool use / function calling format. | P0 |
| SK-06 | Skill dependencies (system packages) are checked and installed if missing. | P1 |
| SK-07 | Engine logs which skills were loaded and how many tools were registered. | P1 |
| SK-08 | Malformed skill manifests produce clear error messages and do not crash the engine. | P1 |

---

## 9. Dependencies

| Dependency | Direction | Description |
|---|---|---|
| Intelligence Engine (03) | Upstream | Skills inject prompts and tools into the engine's LLM context. |
| Package Manager Skill (05) | Downstream | Concrete built-in skill implemented on this framework. |
| System Info Skill (06) | Downstream | Concrete built-in skill implemented on this framework. |
| Base System (01) | Upstream | Skills depend on system packages installed via `dnf`. |
