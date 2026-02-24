# 24 — Community Skill Repository: Design Specification

**Module:** Community Skill Repository
**Phase:** 3

---

## 1. Skill Search Results

Search results are displayed as a styled table in the chat message flow.

```
User: search skills for image editing

Levsha: I found 4 skills matching "image editing":

  ┌─ Available Skills ─────────────────────────────────────────────────┐
  │                                                                     │
  │  🛡 image-editor      v1.2.0   Edit, resize, crop images           │
  │     levsha            ★★★★★ (127)   ↓ 3,241                       │
  │                                                                     │
  │  ✓ image-optimizer    v0.8.1   Compress and optimize images         │
  │     verified-dev      ★★★★☆ (43)    ↓ 892                         │
  │                                                                     │
  │  ○ image-converter    v0.3.0   Convert between image formats        │
  │     community-user    ★★★☆☆ (12)    ↓ 234                         │
  │                                                                     │
  │  ○ screenshot-tool    v0.1.0   Capture and annotate screenshots     │
  │     another-user      ☆☆☆☆☆ (0)     ↓ 18                          │
  │                                                                     │
  └─────────────────────────────────────────────────────────────────────┘

  Say "install <name>" or "skill info <name>" for details.
```

### Search Results Styling

| Element | Style |
|---------|-------|
| Table background | `$bg-surface` (#FDFBF7) |
| Table border | 1px `$border-primary` (#EBE6DC) |
| Table cornerRadius | 8 |
| Table shadow | 0 1px 3px rgba(58,50,40,0.06) |
| Skill name | IBM Plex Mono 13px, weight 500, `$text-primary` (#3A3228) |
| Version | IBM Plex Mono 12px, `$text-tertiary` (#8C7E6E) |
| Description | IBM Plex Sans 13px, `$text-secondary` (#6B5D4F) |
| Author | IBM Plex Sans 12px, `$text-tertiary` |
| Rating stars (filled) | `$accent-gold` (#D4A853) |
| Rating stars (hollow) | `$text-tertiary` (#8C7E6E) |
| Rating count | IBM Plex Sans 11px, `$text-tertiary` |
| Download count | IBM Plex Sans 11px, `$text-tertiary`, Lucide `download` icon |
| Row separator | 1px `$border-primary` (#EBE6DC) |
| Row padding | 12px vertical, 16px horizontal |

---

## 2. Trust Badges

Trust badges appear inline next to the skill name in all views.

| Trust Level | Badge | Color | Icon |
|-------------|-------|-------|------|
| Official | Green shield | `$accent-green` (#62B37B) | Lucide `shield-check` |
| Verified | Blue checkmark | `$accent-blue` (#5B8DBE) | Lucide `check-circle` |
| Community | Gray circle | `$text-tertiary` (#8C7E6E) | Lucide `circle` |

### Badge Styling

| Element | Style |
|---------|-------|
| Badge icon size | 14px |
| Badge spacing | 4px after icon, before skill name |
| Official tooltip | "Official Levsha skill" |
| Verified tooltip | "Reviewed and verified" |
| Community tooltip | "Community-contributed, unreviewed" |

---

## 3. Skill Detail View (Split-View Panel)

Clicking "skill info image-editor" opens the full detail view in the split-view content panel.

```
┌─ image-editor ─────────────────────────────────────────── [×] ┐
│                                                                 │
│  🛡 Image Editor                                    v1.2.0     │
│  by levsha · Official                                           │
│                                                                 │
│  ★★★★★  4.8 (127 ratings) · 3,241 downloads                   │
│                                                                 │
│  ────────────────────────────────────────────────────────────   │
│                                                                 │
│  Edit, resize, crop, and filter images directly from            │
│  your Levsha chat. Supports JPEG, PNG, WebP, and SVG.          │
│                                                                 │
│  Features:                                                      │
│  • Resize and crop with natural language                        │
│  • Apply filters (blur, sharpen, grayscale)                     │
│  • Format conversion                                            │
│  • Batch processing                                             │
│                                                                 │
│  ────────────────────────────────────────────────────────────   │
│                                                                 │
│  Tags: [image] [media] [editing] [graphics]                     │
│                                                                 │
│  Dependencies:                                                  │
│    System: ImageMagick, libjpeg-turbo                           │
│    Skills: filesystem                                           │
│                                                                 │
│  Compatible: Levsha OS 0.2.0 – 1.0.0                           │
│                                                                 │
│  ────────────────────────────────────────────────────────────   │
│                                                                 │
│  Version History                                                │
│  ─────────────────────────────────────────────────────          │
│  v1.2.0  (2026-01-15)  Added batch processing                  │
│  v1.1.0  (2025-11-20)  WebP support                            │
│  v1.0.0  (2025-09-01)  Initial release                         │
│                                                                 │
│  ────────────────────────────────────────────────────────────   │
│                                                                 │
│  ┌───────────┐  ┌──────────────┐                               │
│  │  Install  │  │  Rate ★★★★★ │                               │
│  └───────────┘  └──────────────┘                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Detail Panel Styling

| Element | Style |
|---------|-------|
| Panel header background | `$bg-secondary` (#F5F1EA) |
| Panel header height | 36px |
| Panel header title | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Close button | 24x24, Lucide `x`, `$text-tertiary`, hover: `$text-primary` |
| Skill display name | IBM Plex Sans 20px, weight 600, `$text-primary` |
| Version badge | IBM Plex Mono 13px, `$bg-secondary` pill, `$text-secondary` |
| Author line | IBM Plex Sans 13px, `$text-secondary` |
| Rating stars (filled) | 16px, `$accent-gold` (#D4A853) |
| Rating stats | IBM Plex Sans 13px, `$text-secondary` |
| Section separator | 1px `$border-primary` (#EBE6DC), 16px vertical margin |
| Long description | IBM Plex Sans 14px, `$text-primary`, line-height 1.6 |
| Feature bullets | IBM Plex Sans 14px, `$text-secondary`, 8px left indent |
| Tags | IBM Plex Sans 11px, `$bg-secondary` pill, `$text-secondary` text, cornerRadius 4 |
| Dependency labels | IBM Plex Sans 12px, weight 500, `$text-tertiary`, uppercase |
| Dependency values | IBM Plex Sans 13px, `$text-primary` |
| Version history row | IBM Plex Mono 12px for version, IBM Plex Sans 12px for date and changelog |
| Install button | `$accent-copper` (#C67A52) bg, white text, cornerRadius 6 |
| Rate button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |

---

## 4. Rating Display

### Star Rating

Stars use filled (★) and hollow (☆) characters with `$accent-gold` color.

| Rating | Display |
|--------|---------|
| 5.0 | ★★★★★ |
| 4.0-4.9 | ★★★★☆ |
| 3.0-3.9 | ★★★☆☆ |
| 2.0-2.9 | ★★☆☆☆ |
| 1.0-1.9 | ★☆☆☆☆ |
| 0 (unrated) | ☆☆☆☆☆ |

### Rating Submission

```
User: rate image-editor 4 stars

Levsha: Thanks! I've submitted your rating for image-editor:

  ★★★★☆  4 stars

  Your rating helps the community discover great skills.
```

---

## 5. Update Available Indicator

When skill updates are available, a notification appears in the chat.

```
Levsha: Skill updates available:

  ┌─ Updates ──────────────────────────────────────────────────────┐
  │                                                                 │
  │  image-editor     v1.2.0 → v1.3.0   New filter presets         │
  │  filesystem       v0.1.0 → v0.2.0   Symlink support            │
  │                                                                 │
  │  ┌──────────────┐  ┌──────────────┐                            │
  │  │  Update All  │  │   Dismiss    │                            │
  │  └──────────────┘  └──────────────┘                            │
  │                                                                 │
  └─────────────────────────────────────────────────────────────────┘
```

### Update Notification Styling

| Element | Style |
|---------|-------|
| Container | `$bg-surface` (#FDFBF7), 1px `$accent-copper` (#C67A52) left border (3px), cornerRadius 8 |
| Skill name | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Version range | IBM Plex Mono 12px, `$accent-green` (#62B37B) |
| Changelog preview | IBM Plex Sans 12px, `$text-secondary` |
| Update All button | `$accent-copper` bg, white text, cornerRadius 6 |
| Dismiss button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |

---

## 6. Category Browser (Split-View Panel)

"Browse skills" opens a category grid in the split-view panel.

```
┌─ Skill Categories ────────────────────────────────────── [×] ┐
│                                                                │
│  ┌──────────────────┐  ┌──────────────────┐                   │
│  │  ⚙  System       │  │  </> Development │                   │
│  │  12 skills        │  │  8 skills        │                   │
│  └──────────────────┘  └──────────────────┘                   │
│                                                                │
│  ┌──────────────────┐  ┌──────────────────┐                   │
│  │  🎨 Media        │  │  📋 Productivity │                   │
│  │  6 skills         │  │  5 skills        │                   │
│  └──────────────────┘  └──────────────────┘                   │
│                                                                │
│  ┌──────────────────┐  ┌──────────────────┐                   │
│  │  🌐 Networking   │  │  🔧 Utilities    │                   │
│  │  7 skills         │  │  11 skills       │                   │
│  └──────────────────┘  └──────────────────┘                   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### Category Card Styling

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Card cornerRadius | 8 |
| Card padding | 16px |
| Card hover | `$bg-secondary` (#F5F1EA) background |
| Card shadow | 0 1px 3px rgba(58,50,40,0.06) |
| Grid layout | 2 columns, 12px gap |
| Category icon | 20px, `$accent-copper` (#C67A52) |
| Category name | IBM Plex Sans 14px, weight 600, `$text-primary` |
| Skill count | IBM Plex Sans 12px, `$text-tertiary` |

### Category Icons (Lucide)

| Category | Icon |
|----------|------|
| System | `settings` |
| Development | `code-2` |
| Media | `palette` |
| Productivity | `clipboard-list` |
| Networking | `globe` |
| Utilities | `wrench` |

---

## 7. Dependency Tree Display

Before installing a skill with dependencies, the full tree is shown.

```
User: install code-editor

Levsha: The code-editor skill requires 2 additional dependencies:

  ┌─ Dependency Tree ──────────────────────────────────────────────┐
  │                                                                 │
  │  code-editor v0.5.0                                             │
  │  ├── filesystem v0.2.0 (skill, will be installed)               │
  │  ├── tree-sitter v0.20.0 (system package)                      │
  │  └── git v2.43.0 (system package, already installed)            │
  │                                                                 │
  │  Total: 1 skill + 1 system package to install                   │
  │                                                                 │
  │  ┌───────────┐  ┌──────────┐                                   │
  │  │  Install  │  │  Cancel  │                                   │
  │  └───────────┘  └──────────┘                                   │
  │                                                                 │
  └─────────────────────────────────────────────────────────────────┘
```

### Dependency Tree Styling

| Element | Style |
|---------|-------|
| Container | `$bg-surface` (#FDFBF7), 1px `$border-primary` border, cornerRadius 8 |
| Root skill | IBM Plex Mono 13px, weight 600, `$text-primary` |
| Tree lines (├── └──) | `$text-tertiary` (#8C7E6E) |
| Dependency name | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Dependency version | IBM Plex Mono 12px, `$text-tertiary` |
| Status annotation (will be installed) | IBM Plex Sans 12px, `$accent-copper` (#C67A52) |
| Status annotation (already installed) | IBM Plex Sans 12px, `$accent-green` (#62B37B) |
| Summary line | IBM Plex Sans 13px, weight 500, `$text-secondary` |

---

## 8. Publish Flow

Publishing is shown as a validation checklist in chat.

```
User: publish my skill

Levsha: Publishing your skill to the community registry...

  ✓ Manifest valid (skill.yaml)
  ✓ Tool schemas valid (3 tools)
  ✓ Prompt file exists
  ▶ Packaging skill archive
  ○ Uploading to registry
  ○ Registering metadata

  ████████████░░░░░░░░ 60%
```

After completion:

```
Levsha: Your skill has been published!

  ┌─ Published ────────────────────────────────────────────┐
  │                                                         │
  │  ○ my-custom-skill v0.1.0                               │
  │                                                         │
  │  Trust level: Community (gray badge)                    │
  │  Visible in registry search immediately.                │
  │                                                         │
  │  To request verification, contact the Levsha team.      │
  │                                                         │
  └─────────────────────────────────────────────────────────┘
```

### Publish Progress Styling

| Element | Style |
|---------|-------|
| Completed step (✓) | `$accent-green` (#62B37B), Lucide `check` |
| Active step (▶) | `$accent-copper` (#C67A52), Lucide `loader` (animated) |
| Pending step (○) | `$text-tertiary` (#8C7E6E) |
| Step text | IBM Plex Sans 13px, `$text-primary` |
| Progress bar fill | `$accent-copper` |
| Progress bar track | `$bg-secondary` |

---

## 9. Community Skill Warning

When installing a community (unreviewed) skill:

```
Levsha:

  ┌─ ⚠ Unreviewed Skill ──────────────────────────────────┐
  │                                                         │
  │  screenshot-tool is a community-contributed skill       │
  │  that has not been reviewed by the Levsha team.         │
  │                                                         │
  │  Community skills have full system access and may       │
  │  execute arbitrary commands.                            │
  │                                                         │
  │  ┌───────────────────────┐  ┌──────────┐               │
  │  │  Install Anyway       │  │  Cancel  │               │
  │  └───────────────────────┘  └──────────┘               │
  │                                                         │
  └─────────────────────────────────────────────────────────┘
```

### Warning Styling

| Element | Style |
|---------|-------|
| Container | `$danger-bg` (#FFF5F0), 2px `$accent-copper` border, cornerRadius 8 |
| Warning icon | Lucide `alert-triangle`, `$accent-gold` (#D4A853) |
| Title | IBM Plex Sans 15px, weight 600, `$accent-copper` |
| Body | IBM Plex Sans 13px, `$text-secondary` |
| Emphasis text | IBM Plex Sans 13px, weight 500, `$text-primary` |
| Install button | `$accent-copper` bg, white text |
| Cancel button | `$bg-secondary` bg, `$text-primary` text |

---

## 10. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in skill management messages |
| Lucide `shield-check` | Official trust badge |
| Lucide `check-circle` | Verified trust badge |
| Lucide `circle` | Community trust badge |
| Lucide `star` | Rating stars |
| Lucide `download` | Download count icon |
| Lucide `search` | Search action |
| Lucide `package` | Skill card icon |
| Lucide `upload` | Publish action |
| Lucide `refresh-cw` | Update action |
| Lucide `alert-triangle` | Community skill warning |
| Lucide `check` | Completed publish step |
| Lucide `loader` | Active publish step |
| Lucide `settings` | System category icon |
| Lucide `code-2` | Development category icon |
| Lucide `palette` | Media category icon |
| Lucide `clipboard-list` | Productivity category icon |
| Lucide `globe` | Networking category icon |
| Lucide `wrench` | Utilities category icon |

---

## 11. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Search results table, skill detail panel, category browser, dependency tree, publish progress. |
| Intelligence Engine (03) | Modified | Registry HTTP client, search dispatch, dependency resolution display. |
| Split-View (12) | Consumer | Skill detail view and category browser render in the content panel. |
| Skills System (04) | Upstream | Uses existing skill loading and manifest framework. |
| Skill Repository (11) | Extends | Replaces simple registry with full community platform. |
