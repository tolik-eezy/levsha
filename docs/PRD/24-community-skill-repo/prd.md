# 24 — Community Skill Repository: Product Requirements

**Module:** Community Skill Repository
**Phase:** 3
**Status:** Draft

---

## 1. Overview

Phase 2 introduced Git-based skill install/remove (Module 11) with a simple `index.yaml` registry file. Phase 3 evolves this into a full community platform with a central registry server, rich metadata, trust levels, ratings, dependency resolution, and skill publishing. This is the Levsha OS equivalent of an app store — but entirely through conversation.

The community skill repository transforms Levsha OS from a platform where skills come from known Git URLs into one where users can discover, evaluate, and install skills by browsing categories, reading reviews, and trusting verified authors — all without leaving the chat.

---

## 2. Functional Requirements

### 2.1 Registry HTTP API (CR-01)

| Field | Value |
|-------|-------|
| **ID** | CR-01 |
| **Priority** | P0 |
| **Requirement** | A central registry server with a REST API for searching, browsing, downloading, and publishing skills. |

**Details:**

- The registry replaces the Git-based `index.yaml` with a proper HTTP API server.
- Endpoints:
  - `GET /skills/search?q=&tags=&trust=&sort=` — search with filters.
  - `GET /skills/{id}` — full skill metadata and version history.
  - `GET /skills/{id}/download` — download skill archive for installation.
  - `POST /skills/{id}/rate` — submit a rating (authenticated).
  - `POST /skills/publish` — publish a new skill or version (authenticated).
- The registry stores metadata, not skill files — skill archives are hosted alongside (e.g., Git-backed storage).
- Backward compatibility: direct Git URL install from Module 11 continues to work.
- Default registry URL is configured in `/etc/levsha/config.toml`.

**Acceptance Criteria:**

- [ ] Registry server exposes REST API for search, browse, download, publish.
- [ ] Search supports query text, tag filtering, trust level filtering, and sorting.
- [ ] Skill detail endpoint returns full metadata with version history.
- [ ] Download endpoint returns a skill archive.
- [ ] Direct Git URL install continues to work alongside the registry.

### 2.2 Rich Skill Metadata (CR-02)

| Field | Value |
|-------|-------|
| **ID** | CR-02 |
| **Priority** | P0 |
| **Requirement** | Skills in the registry have rich metadata: author, version, description, tags, ratings, download count, trust level, dependencies, and OS version compatibility. |

**Details:**

- Metadata fields:
  - `name` — unique skill identifier (e.g., `image-editor`).
  - `display_name` — human-readable name (e.g., "Image Editor").
  - `author` — author username.
  - `version` — semver version string.
  - `description` — short description (one line).
  - `long_description` — markdown-formatted detailed description.
  - `tags` — categorization tags.
  - `trust_level` — `official`, `verified`, or `community`.
  - `downloads` — total download count.
  - `avg_rating` — average rating (1.0-5.0).
  - `rating_count` — total number of ratings.
  - `dependencies` — list of required skills and system packages.
  - `compatible_versions` — min/max Levsha OS version range.
  - `created_at`, `updated_at` — timestamps.
  - `manifest_url` — URL to the skill's Git repository.

**Acceptance Criteria:**

- [ ] Skill metadata includes all listed fields.
- [ ] Metadata is returned by search and detail endpoints.
- [ ] Download count increments on each install.

### 2.3 Trust Levels (CR-03)

| Field | Value |
|-------|-------|
| **ID** | CR-03 |
| **Priority** | P0 |
| **Requirement** | Skills have trust levels indicating their review status: official, verified, or community. |

**Details:**

- **Official** — maintained by the Levsha team. Highest trust. Green shield badge.
- **Verified** — reviewed and approved by the Levsha team or trusted reviewers. Blue checkmark badge.
- **Community** — submitted by any user, not reviewed. Gray circle badge.
- Trust level is set during publishing and can be upgraded by reviewers.
- Trust level is prominently displayed in search results and skill detail views.
- Users are warned when installing community (unreviewed) skills.

**Acceptance Criteria:**

- [ ] Skills have one of three trust levels: official, verified, community.
- [ ] Trust level badges are displayed in search results and detail views.
- [ ] Warning is shown when installing community-level skills.

### 2.4 Search with Filters (CR-04)

| Field | Value |
|-------|-------|
| **ID** | CR-04 |
| **Priority** | P0 |
| **Requirement** | Users can search for skills by name, tags, and author, and filter by trust level, compatibility, and sort order. |

**Details:**

- Natural language search: "find skills for image editing" queries the registry.
- Filters:
  - By name/description (text search).
  - By tags (e.g., "media", "development").
  - By author.
  - By trust level ("show only verified skills").
  - By compatibility (current OS version).
- Sort options: by downloads (most popular), by rating, by newest.
- Results are displayed as a styled table in chat (see design spec).

**Acceptance Criteria:**

- [ ] Text search matches skill names, descriptions, and tags.
- [ ] Filters narrow results by trust level, author, and compatibility.
- [ ] Sort options work correctly.
- [ ] Results are displayed as styled chat messages.

### 2.5 Dependency Resolution (CR-05)

| Field | Value |
|-------|-------|
| **ID** | CR-05 |
| **Priority** | P0 |
| **Requirement** | The installer resolves skill dependencies (other skills and system packages) and installs them in the correct order. |

**Details:**

- Skills can declare dependencies on other skills (e.g., `filesystem` skill required by `code-editor` skill).
- Skills can declare dependencies on system packages (via `dnf`, as in Module 11).
- The resolver performs topological sort to determine install order.
- Circular dependency detection — installation fails with a clear error.
- Version constraint support: `>=1.0.0, <2.0.0` syntax.
- Conflict detection: if two skills require incompatible versions of a dependency, installation fails with a clear message.
- Before installation, the full dependency tree is shown to the user for confirmation.

**Acceptance Criteria:**

- [ ] Skill dependencies on other skills are resolved and installed.
- [ ] System package dependencies are resolved and installed.
- [ ] Dependency tree is displayed before installation.
- [ ] Circular dependencies are detected and reported.
- [ ] Version conflicts are detected and reported.
- [ ] Installation proceeds in correct topological order.

### 2.6 Skill Ratings (CR-06)

| Field | Value |
|-------|-------|
| **ID** | CR-06 |
| **Priority** | P1 |
| **Requirement** | Users can rate installed skills 1-5 stars and submit the rating to the registry. |

**Details:**

- "Rate image-editor 4 stars" submits a rating.
- Only installed skills can be rated.
- Users can update their rating by submitting again.
- Ratings are aggregated into the `avg_rating` field.
- Rating submission requires a device identifier (anonymous but unique).

**Acceptance Criteria:**

- [ ] Users can rate installed skills via chat.
- [ ] Ratings are submitted to the registry.
- [ ] Average rating is updated and visible in search results.
- [ ] Only installed skills can be rated.

### 2.7 Version Compatibility (CR-07)

| Field | Value |
|-------|-------|
| **ID** | CR-07 |
| **Priority** | P1 |
| **Requirement** | Skills declare minimum and maximum compatible Levsha OS versions, and the installer checks compatibility before installation. |

**Details:**

- Skills declare `compatible_versions: { min: "0.2.0", max: "1.0.0" }` in their manifest.
- The installer reads the current OS version from `/etc/levsha/version`.
- If the OS version is outside the compatible range, installation is blocked with a clear message.
- Search results can be filtered to only show compatible skills.

**Acceptance Criteria:**

- [ ] Skills declare version compatibility in their manifest.
- [ ] Installer checks OS version before installation.
- [ ] Incompatible skills are blocked with a clear message.
- [ ] Search results can be filtered by compatibility.

### 2.8 Update Notifications (CR-08)

| Field | Value |
|-------|-------|
| **ID** | CR-08 |
| **Priority** | P1 |
| **Requirement** | The system periodically checks for skill updates and notifies the user when updates are available. |

**Details:**

- A background task checks the registry for newer versions of installed skills.
- Check interval is configurable (default: once per day).
- When updates are found, a notification is shown (via Module 21 Notification System).
- "Check for skill updates" triggers an immediate check.
- "Update all skills" installs all available updates.

**Acceptance Criteria:**

- [ ] Background task checks for updates at a configurable interval.
- [ ] Notification is shown when updates are available.
- [ ] Manual check is supported.
- [ ] Bulk update is supported.

### 2.9 Category Browsing (CR-09)

| Field | Value |
|-------|-------|
| **ID** | CR-09 |
| **Priority** | P1 |
| **Requirement** | Skills are organized into categories that users can browse. |

**Details:**

- Categories: System, Development, Media, Productivity, Networking, Utilities.
- "Browse skills" or "show skill categories" displays the category list.
- "Browse development skills" shows all skills in that category.
- Categories are displayed as a grid of cards in the split-view panel (see design spec).
- Skills can belong to multiple categories via tags.

**Acceptance Criteria:**

- [ ] Six predefined categories are available.
- [ ] Users can browse categories via chat.
- [ ] Category view displays skills within the selected category.
- [ ] Skills can appear in multiple categories.

### 2.10 Skill Publishing (CR-10)

| Field | Value |
|-------|-------|
| **ID** | CR-10 |
| **Priority** | P2 |
| **Requirement** | Users can validate, package, and publish skills to the community registry. |

**Details:**

- "Publish my skill" initiates the publish flow.
- Publishing steps:
  1. Validate the skill manifest (schema, required fields, tools).
  2. Validate tool JSON schemas.
  3. Package the skill directory into an archive.
  4. Upload to the registry with metadata.
- Requires a registry account (device-based authentication).
- Published skills start at `community` trust level.
- The publish flow shows a validation checklist in chat.

**Acceptance Criteria:**

- [ ] Skills can be validated and packaged for publishing.
- [ ] Published skills appear in the registry at community trust level.
- [ ] Validation errors are reported clearly.
- [ ] Publishing requires authentication.

---

## 3. Database Schema (Server-Side Reference)

The registry server maintains the following schema:

```sql
CREATE TABLE skills (
    id TEXT PRIMARY KEY,
    name TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    author TEXT NOT NULL,
    version TEXT NOT NULL,
    description TEXT NOT NULL,
    long_description TEXT,
    tags TEXT,                        -- JSON array
    trust_level TEXT NOT NULL,        -- 'official', 'verified', 'community'
    downloads INTEGER DEFAULT 0,
    avg_rating REAL DEFAULT 0.0,
    rating_count INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    manifest_url TEXT NOT NULL,
    compatible_versions TEXT,         -- JSON: { "min": "0.2.0", "max": "1.0.0" }
    dependencies TEXT                 -- JSON array of skill/package dependencies
);

CREATE TABLE skill_versions (
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL REFERENCES skills(id),
    version TEXT NOT NULL,
    changelog TEXT,
    published_at INTEGER NOT NULL,
    archive_url TEXT NOT NULL
);

CREATE TABLE ratings (
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL REFERENCES skills(id),
    device_id TEXT NOT NULL,
    rating INTEGER NOT NULL,          -- 1-5
    created_at INTEGER NOT NULL,
    UNIQUE(skill_id, device_id)
);
```

---

## 4. Configuration

New and modified config entries in `/etc/levsha/config.toml`:

```toml
[skills]
path = "/usr/share/levsha/skills"
installed_path = "/usr/share/levsha/skills/installed"

[skills.registry]
api_url = "https://registry.levsha.dev/api/v1"
legacy_git_url = "https://github.com/levsha-community/skill-registry"  # Backward compat
update_check_interval = 86400    # Seconds between update checks (default: 24h)
device_id_path = "/etc/levsha/device-id"
```

---

## 5. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Skill Repository (11) | Extends | Evolves the Git-based registry into a full community platform. |
| Skills System (04) | Upstream | Uses existing skill loading and manifest framework. |
| Intelligence Engine (03) | Modified | Registry HTTP client, dependency resolver, update checker. |
| Chat Shell (02) | Modified | Skill search results, detail views, category browser. |
| Notification System (21) | Consumer | Update notifications when new skill versions are available. |
| Package Manager Skill (05) | Sibling | Used to install system package dependencies. |

---

## 6. Out of Scope

- Paid skills or monetization.
- Per-skill sandboxing or capability restrictions.
- Automated security scanning of published skills.
- Skill CI/CD or build pipelines.
- Private registries or enterprise features.
- Skill download mirroring or CDN.
