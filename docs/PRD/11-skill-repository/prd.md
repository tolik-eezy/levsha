# 11 — Skill Repository: Product Requirements

**Module:** Skill Install/Remove from Repositories
**Phase:** 2
**Status:** Draft

---

## 1. Overview

In Phase 1, only built-in skills exist. Phase 2 introduces the ability to install and remove skills from external Git-based repositories. This transforms Levsha OS from a fixed-capability system into an extensible platform.

Skills are the Levsha OS equivalent of applications. Skill install/remove is the equivalent of an app store — but done entirely through conversation.

---

## 2. Functional Requirements

### 2.1 Skill Install from Git (SR-01)

| Field | Value |
|-------|-------|
| **ID** | SR-01 |
| **Priority** | P0 |
| **Requirement** | Users can install skills from Git repositories via chat. |

**Details:**

- A skill repository is a Git repository containing one or more skill directories, each with a valid `skill.yaml` manifest.
- Installation flow:
  1. User requests a skill: "install the image editor skill" or provides a repo URL.
  2. Engine clones the repository to a staging area.
  3. Engine validates the skill manifest (schema, required fields, tool definitions).
  4. Engine resolves and installs system package dependencies via `dnf`.
  5. Engine copies skill files to `/usr/share/levsha/skills/<skill-name>/`.
  6. Engine hot-reloads the skill (or prompts for restart if needed).
  7. Skill prompt and tools are now available in the LLM context.

- **Skill registry:** A curated index file (YAML/JSON) mapping skill names to Git repository URLs. Hosted as a Git repository itself. Allows `install image-editor` without knowing the URL.
- **Direct URL install:** `install skill from https://github.com/levsha-community/skill-image-editor` works without the registry.

**Acceptance Criteria:**

- [ ] Skills can be installed from a Git URL.
- [ ] Skills can be installed by name from the registry.
- [ ] Manifest is validated before installation.
- [ ] System dependencies are installed automatically.
- [ ] Installed skill is immediately available (hot-reload or prompted restart).
- [ ] Installation errors produce clear messages.

### 2.2 Skill Remove (SR-02)

| Field | Value |
|-------|-------|
| **ID** | SR-02 |
| **Priority** | P0 |
| **Requirement** | Users can remove installed skills via chat. |

**Details:**

- Removal flow:
  1. User requests removal: "remove the image editor skill".
  2. Engine identifies the skill by name.
  3. Engine checks for conflicts (no other skill depends on this one).
  4. Confirmation prompt (removal is destructive).
  5. Engine removes skill files from `/usr/share/levsha/skills/<skill-name>/`.
  6. Engine hot-reloads skill context (or prompts for restart).
  7. Optionally: remove orphaned system package dependencies.

- Built-in skills (`builtin: true`) cannot be removed. Attempting to remove one produces a clear message.

**Acceptance Criteria:**

- [ ] Installed skills can be removed via chat.
- [ ] Built-in skills cannot be removed.
- [ ] Removal requires confirmation.
- [ ] Skill is no longer available after removal.
- [ ] Orphaned dependencies are optionally cleaned up.

### 2.3 Skill List and Search (SR-03)

| Field | Value |
|-------|-------|
| **ID** | SR-03 |
| **Priority** | P0 |
| **Requirement** | Users can list installed skills and search available skills from the registry. |

**Details:**

- `list skills` — shows all installed skills with name, version, and description.
- `search skills for image editing` — queries the skill registry for matching skills.
- `skill info image-editor` — shows detailed info for a specific skill.

**Acceptance Criteria:**

- [ ] `list skills` shows all installed skills.
- [ ] `search skills` queries the registry index.
- [ ] `skill info` shows detailed metadata for a skill.

### 2.4 Skill Update (SR-04)

| Field | Value |
|-------|-------|
| **ID** | SR-04 |
| **Priority** | P1 |
| **Requirement** | Users can update installed skills to newer versions. |

**Details:**

- `update image-editor` — pulls latest from the skill's Git repository, validates, and replaces.
- `update all skills` — updates all installed skills.
- Version comparison is based on the `version` field in `skill.yaml`.
- Before update, the current version is backed up (like self-improvement checkpoints).

**Acceptance Criteria:**

- [ ] Individual skills can be updated.
- [ ] Bulk update is supported.
- [ ] Current version is backed up before update.
- [ ] Update errors leave the previous version intact.

### 2.5 Skill Hot-Reload (SR-05)

| Field | Value |
|-------|-------|
| **ID** | SR-05 |
| **Priority** | P1 |
| **Requirement** | Skill installation, removal, and updates take effect without restarting the system. |

**Details:**

Since skills are prompt fragments and tool definitions (not compiled code), they can be reloaded at runtime:

1. Skill loader re-scans the skills directory.
2. Updated skill prompts are injected into the next API call's system prompt.
3. Updated tool definitions are included in the next API call.
4. No restart is needed for prompt/tool changes.

If a skill includes custom compiled tools (future), a restart may be needed.

**Acceptance Criteria:**

- [ ] Skill install/remove/update takes effect on the next user message.
- [ ] No system restart required for prompt/tool-only skills.
- [ ] Skill loader detects changes and reloads automatically.

---

## 3. Skill Registry Format

The skill registry is a Git repository containing an `index.yaml`:

```yaml
# index.yaml
skills:
  - name: image-editor
    description: "Edit, resize, crop, and filter images"
    repository: https://github.com/levsha-community/skill-image-editor
    version: 1.2.0
    author: levsha-community
    tags: [image, media, editing]

  - name: filesystem
    description: "Browse, copy, move, and edit files"
    repository: https://github.com/levsha-community/skill-filesystem
    version: 0.1.0
    author: levsha
    tags: [files, filesystem, core]

  - name: network-config
    description: "Configure network interfaces, Wi-Fi, DNS"
    repository: https://github.com/levsha-community/skill-network-config
    version: 0.1.0
    author: levsha
    tags: [network, wifi, core]
```

Default registry URL is configured in `/etc/levsha/config.toml`.

---

## 4. Skill Directory Layout (Installed)

```
/usr/share/levsha/skills/
├── built-in/
│   ├── package-manager/       # Cannot be removed
│   │   ├── skill.yaml
│   │   ├── prompts/
│   │   └── tools/
│   └── sysinfo/               # Cannot be removed
│       ├── skill.yaml
│       ├── prompts/
│       └── tools/
└── installed/                  # User-installed skills
    ├── image-editor/
    │   ├── skill.yaml
    │   ├── prompts/
    │   ├── tools/
    │   └── assets/
    └── filesystem/
        ├── skill.yaml
        ├── prompts/
        └── tools/
```

---

## 5. Security Considerations

- **Skill validation:** Manifests are validated for schema compliance. Tool definitions are checked for valid JSON schema.
- **No sandboxing (Phase 2):** Like built-in skills, installed skills have full system access. The LLM decides when to use them.
- **User confirmation:** Installation from untrusted sources shows a warning. The user must confirm.
- **Destructive guard:** The existing destructive command confirmation applies to all skill tools, regardless of source.
- **Skill signing (future):** Cryptographic signing of skill manifests for trusted sources. Not in Phase 2.

---

## 6. Configuration

New config entries in `/etc/levsha/config.toml`:

```toml
[skills]
path = "/usr/share/levsha/skills"
registry_url = "https://github.com/levsha-community/skill-registry"
installed_path = "/usr/share/levsha/skills/installed"
```

---

## 7. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Skills System (04) | Upstream | Extends the existing skill framework with install/remove. |
| Intelligence Engine (03) | Upstream | Skill management tools are dispatched by the engine. |
| Package Manager Skill (05) | Sibling | Used to install system package dependencies. |
| Git | Infrastructure | Required for cloning skill repositories. |

---

## 8. Out of Scope

- Skill marketplace UI (Phase 3).
- Skill creation wizard via chat (Phase 3).
- Cryptographic signing and trust levels.
- Per-skill sandboxing or capability restrictions.
- Skill dependency resolution between skills.
- Automatic skill discovery and recommendation.
