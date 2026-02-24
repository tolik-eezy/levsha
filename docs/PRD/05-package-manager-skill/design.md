# 05 — Package Manager Skill: Design

> **Visual styling follows [theme.design.md](../theme.design.md).** All colors, typography, spacing, and component styles are defined in the central theme document. This module must not define its own color palette or override theme tokens.

**Module:** Package Manager Skill (Built-In)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document defines the tool interface, system prompt fragment, response formatting, and UX flows for the package manager skill. The skill provides five tools that the LLM invokes via function calling to manage Fedora packages through `dnf`.

---

## 2. Tool Definitions

### 2.1 `package_install`

Installs one or more packages.

- **Parameters:** `packages` (array of strings) — package names to install.
- **Behavior:** Runs `dnf install -y` for each package. Returns per-package success/failure with version installed.
- **Confirmation:** Not required (install is non-destructive).

### 2.2 `package_remove`

Removes one or more packages.

- **Parameters:** `packages` (array of strings) — package names to remove.
- **Behavior:** Requires destructive command confirmation from the user before executing. Runs `dnf remove -y`. Returns list of removed packages.
- **Confirmation:** Required (destructive operation).

### 2.3 `package_search`

Searches for packages matching a query.

- **Parameters:** `query` (string) — search term.
- **Behavior:** Runs `dnf search`. Returns up to 20 results with name and summary. If more results exist, indicates truncation.

### 2.4 `package_update`

Updates all installed packages or specific packages.

- **Parameters:** `packages` (array of strings, optional) — if empty, updates all.
- **Behavior:** Runs `dnf upgrade -y`. Returns count of updated packages and their names.

### 2.5 `package_list`

Lists installed packages.

- **Parameters:** `filter` (string, optional) — filter by name substring.
- **Behavior:** Runs `dnf list installed`. Returns package names and versions.

---

## 3. System Prompt Fragment

The following is injected into the LLM context when the skill is active (always, in MVP):

```
You have access to the system's package manager (dnf on Fedora). You can install, remove, search, update, and list packages.

When a user asks about software, tools, or utilities — check if a relevant package exists. Offer to install it.

When the user asks to remove a package, always confirm before proceeding. Show what will be removed, including dependencies.

Format package listings as tables. Show progress for long operations. If an operation fails, explain why in plain language and suggest next steps.

Never suggest using dnf directly. You are the interface.
```

---

## 4. Response Formatting

All response formatting uses theme tokens from `theme.design.md`. Refer to the theme for exact hex values.

**Color token reference for this module:**

| Symbol / Element | Theme Token | Color | Usage |
|---|---|---|---|
| `◐` (spinner) | `text-secondary` | `#6B5D4F` warm mid-brown | In-progress indicator |
| `✓` (checkmark) | `success` | `#62B37B` green | Success indicator |
| `✗` (failure X) | `error` | `#C67A52` copper | Failure indicator |
| `⚠` (warning) | `warning` | `#D4A853` gold | Warning / destructive prompt |
| Table borders | `bg-tertiary` | `#EBE6DC` gentle tan | Box-drawing characters |
| Table text | `text-primary` | `#3A3228` warm brown | Package names, descriptions |
| Status text | `text-secondary` | `#6B5D4F` warm mid-brown | Progress messages |
| Error background | `error-bg` | `#FBF2ED` faint copper wash | Error message container |
| Warning background | `warning-bg` | `#FBF6EC` pale gold wash | Removal confirmation container |
| Info background | `info-bg` | `#F5F1EA` secondary parchment | Search results container |

### 4.1 Search Results

Display as a bordered table with package name and description. Table borders use `bg-tertiary`, text uses `text-primary`.

```
  ┌──────────────────────────────────────────────────┐
  │  imagemagick    — CLI image manipulation suite    │
  │  gimp           — full image editor (GUI)         │
  │  ffmpeg         — video/image conversion          │
  │  libvips        — fast image processing library   │
  └──────────────────────────────────────────────────┘
```

### 4.2 Install Progress

Show a spinner (`◐`, `text-secondary` warm mid-brown) during installation, then a checkmark (`✓`, `success` green) on completion:

```
  ◐ Installing imagemagick...          ← spinner: text-secondary (#6B5D4F)
  ✓ imagemagick 7.1.1 installed        ← checkmark: success (#62B37B)

  ◐ Installing ffmpeg...
  ✓ ffmpeg 6.1.1 installed
```

### 4.3 Update Progress

```
  ◐ Updating package database...       ← spinner: text-secondary (#6B5D4F)
  ✓ Package database updated.          ← checkmark: success (#62B37B)
  ◐ Upgrading 7 packages...
  ✓ All packages up to date.
```

### 4.4 Removal Confirmation

Container uses `warning-bg` background with `danger-border` border (see theme `§5.7 Destructive Command Confirmation`).

```
  ⚠ This will remove the following packages:    ← warning: warning (#D4A853)
  ┌────────────────────────────────────┐
  │  imagemagick       7.1.1-1.fc41   │
  │  imagemagick-libs  7.1.1-1.fc41   │
  └────────────────────────────────────┘

  Confirm removal? [yes / no]
```

Confirm button: `accent-copper` background (`#C67A52`), `text-inverse` (`#FAF8F4`) text.
Cancel button: `bg-secondary` background (`#F5F1EA`), `text-primary` (`#3A3228`) text.

### 4.5 Error Display

Container uses `error-bg` background with `error` border (see theme `§5.8 Error Display`).

```
  ✗ Package 'foobar' not found.        ← failure X: error (#C67A52)

  Did you mean one of these?
  ┌────────────────────────────────────┐
  │  foomatic         — print filters  │
  │  fuseiso          — mount ISOs     │
  └────────────────────────────────────┘
```

Retry button (when applicable): `accent-copper` background (`#C67A52`), `text-inverse` (`#FAF8F4`) text.

---

## 5. UX Flow: Install Packages (Scenario 10.1)

```
User: "I need to work with some images, what tools are available?"

1. LLM interprets intent as package search for image tools.
2. LLM calls package_search(query="image").
3. Tool returns list of matching packages.
4. LLM formats results as a table and asks if the user wants to install any.

User: "install imagemagick and ffmpeg"

5. LLM calls package_install(packages=["imagemagick", "ffmpeg"]).
6. Chat Shell renders progress spinners for each package.
7. Tool returns success with versions.
8. LLM confirms installation and notes that no image-editing skill
   exists yet, but CLI tools are available.
```

---

## 6. Error Handling Flows

### 6.1 Package Not Found

```
User: "install foobar"
→ dnf returns "no match"
→ LLM responds: "I couldn't find 'foobar'. Want me to search for something similar?"
```

### 6.2 Dependency Conflict

```
User: "install packageA"
→ dnf returns conflict with packageB
→ LLM shows the conflict and asks: "packageA conflicts with packageB. Remove packageB first?"
```

### 6.3 Network Failure

```
User: "update everything"
→ dnf cannot reach mirror
→ LLM responds: "Can't reach the package repository. Want me to retry?"
→ Displays [ Retry ] [ Skip ] options
```

---

## 7. Interaction with Intelligence Engine

- The engine detects package-related intent and dispatches to the appropriate tool.
- For `package_remove`, the engine triggers the destructive command confirmation flow (requirement IE-03) before executing.
- Tool output (stdout/stderr from dnf) is parsed by the engine and passed to the LLM for formatting into a human-friendly response.
- The LLM never exposes raw `dnf` output directly — it always reformats.
