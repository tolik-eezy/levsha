# 05 — Package Manager Skill: Product Requirements

**Module:** Package Manager Skill (Built-In)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

The package manager skill is one of two built-in skills that ship with Levsha OS MVP. It enables users to install, remove, search, update, and list system packages through natural language conversation. The skill wraps Fedora's native `dnf` package manager — users never interact with `dnf` directly.

This skill is always active. It cannot be disabled or removed.

---

## 2. Requirements

| ID | Requirement | Priority | Acceptance Criteria |
|---|---|---|---|
| PK-01 | Install packages via chat | P0 | User says "install ffmpeg" or "I need a C compiler" and the correct package(s) are installed. Success/failure is reported inline. |
| PK-02 | Remove packages via chat | P0 | User says "uninstall imagemagick" or "remove curl" and the package is removed. Triggers destructive command confirmation before executing. |
| PK-03 | Search for packages | P0 | User says "is there a package for PDF editing?" and receives a formatted table of matching packages with name + short description. |
| PK-04 | Update all packages | P0 | User says "update everything" and all upgradable packages are updated. Progress is shown during the operation. |
| PK-05 | List installed packages | P0 | User says "what's installed?" and receives a formatted list of installed packages with version numbers. |
| PK-06 | Uses Fedora's native package manager (dnf) | P0 | All operations use `dnf` commands. No alternative package managers (snap, flatpak, pip) in MVP. |

---

## 3. Natural Language Examples

The LLM maps natural language to the appropriate tool call. Examples of supported phrasings:

**Install:**
- "install ffmpeg"
- "I need a C compiler"
- "get me imagemagick and ffmpeg"
- "add the nginx package"

**Remove:**
- "uninstall imagemagick"
- "remove curl"
- "I don't need gimp anymore"

**Search:**
- "is there a package for PDF editing?"
- "search for python libraries"
- "what image tools are available?"
- "find packages related to networking"

**Update:**
- "update everything"
- "upgrade all packages"
- "are there any updates?"

**List:**
- "what's installed?"
- "show me all packages"
- "list installed packages"
- "what do I have on this system?"

---

## 4. Destructive Command Confirmation

Package removal (PK-02) is a destructive operation. Before executing a remove command, the engine must present a visually distinct confirmation prompt in the chat:

```
> remove imagemagick

  ⚠ This will remove the following packages:
  ┌────────────────────────────────────┐
  │  imagemagick       7.1.1-1.fc41   │
  │  imagemagick-libs  7.1.1-1.fc41   │
  └────────────────────────────────────┘

  Confirm removal? [yes / no]
```

The operation proceeds only after explicit user confirmation.

---

## 5. Error Handling

| Scenario | Expected Behavior |
|---|---|
| Package not found | "I couldn't find a package called 'xyz'. Want me to search for something similar?" |
| Dependency conflict | Show the conflict details from dnf and suggest resolution (e.g., "Package A conflicts with B. Want me to remove B first?"). |
| Network unreachable | "Can't reach the package repository right now. Want me to retry?" with a retry option. |
| Insufficient disk space | Report available vs. required space. Suggest cleanup. |
| Permission error | Should not occur (runs as root). If it does, report the raw error. |

---

## 6. Acceptance Criteria

1. User asks to install a valid package — package is installed and confirmed in chat.
2. User asks to remove a package — confirmation prompt appears; package is removed only after approval.
3. User searches for packages — formatted table of results is displayed.
4. User requests update — all packages are updated with progress indication.
5. User asks what's installed — formatted list is displayed.
6. Invalid package name — friendly error with suggestion to search.
7. Network failure during operation — styled error with retry option.

---

## 7. Out of Scope (MVP)

- Alternative package managers (flatpak, snap, pip, npm)
- Package pinning or version locking
- Repository management (adding/removing repos)
- Automatic dependency explanation
- Rollback of package operations
