# 19 — Multi-Session Support: Product Requirements

**Module:** Multi-Session Support (L2 + L3)
**Phase:** 2
**Status:** Draft

---

## 1. Overview

Phase 1 supports a single, continuous conversation. Phase 2 introduces multi-session support — the ability to have multiple independent conversation threads with separate contexts, history, and active skills. Users can create new sessions, switch between them, archive old ones, and name them for quick navigation.

Sessions are how users organize different workflows — one for system administration, another for a coding project, another for experimentation. Each session maintains its own conversation history, context window, and open file state.

---

## 2. Functional Requirements

### 2.1 Session Creation (MS-01)

| Field | Value |
|-------|-------|
| **ID** | MS-01 |
| **Priority** | P0 |
| **Requirement** | Users can create new conversation sessions via chat or keyboard shortcut. |

**Details:**

- "New session" or "start a new conversation" creates a fresh session.
- Keyboard shortcut: `Ctrl+N` creates a new session.
- New sessions start with the standard welcome message and a clean context window.
- Active skills carry over to new sessions (global configuration).
- The previous session is preserved and accessible.

**Acceptance Criteria:**

- [ ] Users can create new sessions via chat.
- [ ] Keyboard shortcut Ctrl+N creates a new session.
- [ ] New sessions start with a clean context.
- [ ] Previous session is preserved.

### 2.2 Session Switching (MS-02)

| Field | Value |
|-------|-------|
| **ID** | MS-02 |
| **Priority** | P0 |
| **Requirement** | Users can switch between active sessions via chat or keyboard shortcut. |

**Details:**

- "Switch to session [name]" or "go to my coding session" switches context.
- Keyboard shortcuts: `Ctrl+Tab` cycles to next session, `Ctrl+Shift+Tab` to previous.
- Session switching is instant — the chat view swaps to the other session's history.
- The engine reloads the session's conversation context for API calls.
- Any open split-view content is session-specific and restores when switching back.
- Status bar shows the current session name.

**Acceptance Criteria:**

- [ ] Users can switch sessions via chat.
- [ ] Keyboard shortcuts cycle between sessions.
- [ ] Session switching restores history and context.
- [ ] Status bar shows current session name.

### 2.3 Session Naming (MS-03)

| Field | Value |
|-------|-------|
| **ID** | MS-03 |
| **Priority** | P0 |
| **Requirement** | Sessions can be named for easy identification. |

**Details:**

- "Name this session 'Server Setup'" assigns a name.
- Auto-generated names: the LLM generates a short name based on the first few messages (e.g., "Package Installation", "Config Editing").
- Session names appear in the session list and status bar.
- Names can be changed at any time.

**Acceptance Criteria:**

- [ ] Users can name sessions.
- [ ] Auto-generated names are assigned to unnamed sessions.
- [ ] Names are displayed in the session list and status bar.
- [ ] Names can be changed.

### 2.4 Session List (MS-04)

| Field | Value |
|-------|-------|
| **ID** | MS-04 |
| **Priority** | P0 |
| **Requirement** | Users can view a list of all sessions with metadata. |

**Details:**

- "List sessions" or "show my sessions" displays all sessions.
- Session list shows: name, last activity timestamp, message count, preview of last message.
- Sessions are ordered by last activity (most recent first).
- Keyboard shortcut: `Ctrl+Shift+S` opens the session list as an overlay.

**Acceptance Criteria:**

- [ ] Session list shows all sessions with metadata.
- [ ] Sessions are ordered by last activity.
- [ ] Keyboard shortcut opens session list.

### 2.5 Session Archive and Delete (MS-05)

| Field | Value |
|-------|-------|
| **ID** | MS-05 |
| **Priority** | P1 |
| **Requirement** | Users can archive completed sessions and delete sessions they no longer need. |

**Details:**

- "Archive this session" moves it to an archived state (not shown in active list by default).
- "Show archived sessions" reveals them.
- "Delete this session" permanently removes it (destructive — requires confirmation).
- Archived sessions can be restored: "Restore session 'Server Setup'".

**Acceptance Criteria:**

- [ ] Sessions can be archived.
- [ ] Archived sessions are hidden from the default list.
- [ ] Sessions can be permanently deleted (with confirmation).
- [ ] Archived sessions can be restored.

### 2.6 Session-Scoped State (MS-06)

| Field | Value |
|-------|-------|
| **ID** | MS-06 |
| **Priority** | P0 |
| **Requirement** | Each session maintains independent state: conversation history, open editor files, split-view content. |

**Details:**

Session-scoped state includes:
- Conversation history (messages)
- Context window content (for API calls)
- Open file in text/code editor
- Split-view panel content
- Project working directory (code editor)

Global state (shared across sessions):
- Installed skills
- API key and model selection
- Backend routing mode (auto/cloud/local)
- System configuration

**Acceptance Criteria:**

- [ ] Each session has independent conversation history.
- [ ] Open editor files are session-scoped.
- [ ] Split-view content is session-scoped.
- [ ] Skills and API config are global.

---

## 3. Database Schema

The existing `messages` table already has a `session_id` column. Multi-session adds a `sessions` table:

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,           -- UUID
    name TEXT,                      -- User-assigned or auto-generated name
    created_at INTEGER NOT NULL,    -- Unix timestamp
    updated_at INTEGER NOT NULL,    -- Last activity timestamp
    message_count INTEGER DEFAULT 0,
    is_archived INTEGER DEFAULT 0,  -- 0 = active, 1 = archived
    metadata TEXT                   -- JSON: open files, working directory, etc.
);
```

---

## 4. Configuration

New config entries in `/etc/levsha/config.toml`:

```toml
[sessions]
max_active = 20              # Maximum active (non-archived) sessions
auto_name = true             # Auto-generate session names from content
default_name = "New Session" # Name for new sessions before auto-naming
```

---

## 5. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Session switching UI, session list overlay, status bar session name. |
| Intelligence Engine (03) | Modified | Per-session context management, session-scoped tool state. |
| Persistence (07) | Modified | New sessions table, session-scoped message queries. |
| Split-View (12) | Modified | Session-scoped content panel state save/restore. |
| Text Editor Skill (17) | Modified | Editor state is per-session. |
| Code Editor Skill (18) | Modified | Project context is per-session. |

---

## 6. Out of Scope

- Session sharing or export.
- Multi-user sessions.
- Session templates (predefined skill sets for a session).
- Session-specific skill overrides.
- Concurrent sessions (only one session is active at a time).
- Cross-session context or memory.
