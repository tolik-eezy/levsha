# 05 — Package Manager Skill: Technical Plan

**Module:** Package Manager Skill (Built-In)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document specifies the implementation details for the package manager skill: dnf command mappings, tool JSON schemas, output parsing, the skill manifest, and the testing plan.

---

## 2. dnf Command Mapping

| Tool | dnf Command | Notes |
|---|---|---|
| `package_install` | `dnf install -y <packages>` | `-y` auto-confirms. Multiple packages in one command. |
| `package_remove` | `dnf remove -y <packages>` | Executed only after user confirmation via the engine. |
| `package_search` | `dnf search <query>` | Returns name + summary. Limit output to 20 results. |
| `package_update` | `dnf upgrade -y [packages]` | No args = upgrade all. Specific packages if provided. |
| `package_list` | `dnf list installed [filter]` | Pipe through grep if filter provided. |

All commands run as root (the system runs single-user with root access in MVP).

---

## 3. Tool JSON Schemas

### 3.1 package_install

```json
{
  "name": "package_install",
  "description": "Install one or more system packages using dnf.",
  "parameters": {
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

### 3.2 package_remove

```json
{
  "name": "package_remove",
  "description": "Remove one or more system packages. Requires user confirmation.",
  "parameters": {
    "type": "object",
    "properties": {
      "packages": {
        "type": "array",
        "items": { "type": "string" },
        "description": "List of package names to remove."
      }
    },
    "required": ["packages"]
  }
}
```

### 3.3 package_search

```json
{
  "name": "package_search",
  "description": "Search for available packages matching a query.",
  "parameters": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search term to find matching packages."
      }
    },
    "required": ["query"]
  }
}
```

### 3.4 package_update

```json
{
  "name": "package_update",
  "description": "Update installed packages. Updates all if no packages specified.",
  "parameters": {
    "type": "object",
    "properties": {
      "packages": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Specific packages to update. Empty array or omitted = update all."
      }
    },
    "required": []
  }
}
```

### 3.5 package_list

```json
{
  "name": "package_list",
  "description": "List installed system packages.",
  "parameters": {
    "type": "object",
    "properties": {
      "filter": {
        "type": "string",
        "description": "Optional substring to filter package names."
      }
    },
    "required": []
  }
}
```

---

## 4. Output Parsing

Each tool execution captures stdout and stderr from the dnf command. The engine parses the raw output into structured data before passing it to the LLM.

### 4.1 Install/Remove Output

Parse dnf's transaction summary to extract:
- Package name, version, and architecture for each installed/removed package.
- Total download size and installed size.
- Any error messages or warnings.

### 4.2 Search Output

Parse `dnf search` output. Each result line contains `name.arch : summary`. Extract:
- Package name (strip architecture suffix).
- Summary description.
- Truncate to 20 results maximum.

### 4.3 Update Output

Parse upgrade transaction summary:
- Count of packages upgraded.
- List of package names and new versions.
- "Nothing to do" if already up to date.

### 4.4 List Output

Parse `dnf list installed` output. Each line contains `name.arch version repo`. Extract:
- Package name (strip architecture suffix).
- Version string.
- Apply filter substring match if provided.

---

## 5. Skill Manifest

```yaml
# skills/built-in/package-manager/skill.yaml
name: package-manager
version: 1.0.0
description: "Install, remove, search, update, and list system packages via dnf."
author: levsha-os
built_in: true
removable: false

requires:
  packages:
    - dnf

prompt: prompts/package-manager.md

tools:
  - tools/package_install.json
  - tools/package_remove.json
  - tools/package_search.json
  - tools/package_update.json
  - tools/package_list.json
```

---

## 6. File Structure

```
skills/built-in/package-manager/
  skill.yaml
  prompts/
    package-manager.md          # System prompt fragment
  tools/
    package_install.json        # Tool schema
    package_remove.json
    package_search.json
    package_update.json
    package_list.json
```

---

## 7. Testing Plan

### 7.1 Unit Tests

| Test | Description |
|---|---|
| Parse install output | Verify structured data extraction from `dnf install` stdout. |
| Parse search output | Verify name/summary extraction and 20-result truncation. |
| Parse update output | Verify upgrade count and package list extraction. |
| Parse list output | Verify name/version extraction with and without filter. |
| Parse error output | Verify error messages are extracted for not-found, conflicts, network errors. |

### 7.2 Integration Tests (VM)

| Test | Description |
|---|---|
| Install a package | Ask "install cowsay" — verify package is installed (`rpm -q cowsay`). |
| Remove a package | Ask "remove cowsay" — verify confirmation prompt appears, then package is removed. |
| Search packages | Ask "search for text editors" — verify table of results appears. |
| Update packages | Ask "update everything" — verify progress and completion. |
| List packages | Ask "what's installed?" — verify formatted list appears. |
| Package not found | Ask "install nonexistent-pkg-xyz" — verify friendly error. |
| Network failure | Disconnect network, ask "install curl" — verify error with retry option. |

### 7.3 Acceptance Tests

| Test | Maps to |
|---|---|
| Full install flow from natural language | PK-01 |
| Removal with confirmation | PK-02, IE-03 |
| Search with formatted results | PK-03 |
| Update all packages | PK-04 |
| List installed packages | PK-05 |
| All operations use dnf | PK-06 |
