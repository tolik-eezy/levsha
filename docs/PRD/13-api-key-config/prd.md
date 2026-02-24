# 13 — User-Configurable API Key: Product Requirements

**Module:** API Key Configuration
**Phase:** 2
**Status:** Draft

---

## 1. Overview

In Phase 1, the API key is hardcoded in `/etc/levsha/config.toml`. Phase 2 replaces this with a user-configurable API key, making the OS distributable — users bring their own key.

---

## 2. Functional Requirements

### 2.1 First-Boot Key Setup (AK-01)

| Field | Value |
|-------|-------|
| **ID** | AK-01 |
| **Priority** | P0 |
| **Requirement** | On first boot (no API key configured), the Chat Shell displays a key entry prompt instead of the welcome message. |

**Details:**

- The first-boot experience changes:
  1. Chat Shell appears with a styled prompt: "Welcome to Levsha OS. To get started, enter your Anthropic API key."
  2. Input field accepts the key. Key is masked (dots/asterisks) after entry.
  3. Engine validates the key by sending a minimal test request to the API.
  4. On success: key is stored securely, welcome message appears, normal operation begins.
  5. On failure: clear error message ("Invalid key" or "Cannot reach API"), prompt to retry.

- The key entry screen is part of the Chat Shell — not a separate setup wizard. It looks and feels like a chat interaction.

**Acceptance Criteria:**

- [ ] First boot with no key shows a key entry prompt.
- [ ] Key is validated against the API before acceptance.
- [ ] Invalid key shows a clear error and allows retry.
- [ ] Successful key entry transitions to normal chat.

### 2.2 Key Storage (AK-02)

| Field | Value |
|-------|-------|
| **ID** | AK-02 |
| **Priority** | P0 |
| **Requirement** | API key is stored securely on the local filesystem with restricted permissions. |

**Details:**

- Key is stored in `/etc/levsha/config.toml` (same location as Phase 1, but now user-provided).
- File permissions: `600` (owner read/write only), owned by the `levsha` user.
- Key is not stored in conversation history, not logged, not included in any diagnostic output.
- The ISO ships without a key — the config file has a placeholder or empty `key` field.

**Acceptance Criteria:**

- [ ] Key is stored in the config file with restrictive permissions.
- [ ] Key does not appear in logs, history, or diagnostics.
- [ ] ISO ships without a hardcoded key.

### 2.3 Key Change via Chat (AK-03)

| Field | Value |
|-------|-------|
| **ID** | AK-03 |
| **Priority** | P1 |
| **Requirement** | User can change the API key via chat after initial setup. |

**Details:**

- "Change my API key" or "update API key" triggers the key change flow.
- Current key is not displayed. User enters new key.
- New key is validated before replacing the old one.
- Engine reconnects with the new key.

**Acceptance Criteria:**

- [ ] User can change the key via natural language request.
- [ ] New key is validated before saving.
- [ ] Engine uses the new key for subsequent requests.

### 2.4 Model Selection (AK-04)

| Field | Value |
|-------|-------|
| **ID** | AK-04 |
| **Priority** | P1 |
| **Requirement** | User can select which Claude model to use via chat. |

**Details:**

- "Use Claude Opus" or "switch to Sonnet" changes the model.
- Available models are listed in the config with display names.
- Model change takes effect on the next request.
- Status bar updates to show the active model.

**Acceptance Criteria:**

- [ ] User can switch models via chat.
- [ ] Status bar reflects the current model.
- [ ] Model change is persisted across restarts.

---

## 3. Configuration

Updated `/etc/levsha/config.toml`:

```toml
[api]
key = ""                           # Empty on fresh install
model = "claude-sonnet-4-20250514" # Default model
base_url = "https://api.anthropic.com"
timeout_seconds = 30
max_retries = 3

[api.models]
# Available models for user selection
sonnet = "claude-sonnet-4-20250514"
opus = "claude-opus-4-20250514"
haiku = "claude-haiku-4-5-20251001"
```

---

## 4. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Intelligence Engine (03) | Modified | Key validation, model switching, config reload. |
| Chat Shell (02) | Modified | Key entry UI, model display in status bar. |
| Boot & First Run (08) | Modified | First-boot flow changes from welcome to key setup. |

---

## 5. Out of Scope

- Multiple API key management (one key at a time).
- Non-Anthropic API providers (future: local LLM addresses this).
- Key rotation or expiry management.
- Team/organization key management.
