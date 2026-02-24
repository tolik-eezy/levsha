# 11 — Skill Repository: Design Specification

**Module:** Skill Install/Remove from Repositories
**Phase:** 2

---

## 1. Skill Management Interaction Design

All skill management happens through natural conversation. There is no dedicated UI for skill browsing — the LLM formats skill information as styled chat messages.

### Install Flow

```
User: install the image editor skill

Levsha: I found the image-editor skill in the repository:

  ┌─ Skill: image-editor ──────────────────────────┐
  │                                                  │
  │  Edit, resize, crop, and filter images           │
  │                                                  │
  │  Author: levsha-community                        │
  │  Version: 1.2.0                                  │
  │  Tags: image, media, editing                     │
  │                                                  │
  │  System packages required:                       │
  │    • ImageMagick                                 │
  │    • libjpeg-turbo                               │
  │                                                  │
  │  ┌───────────┐  ┌──────────┐                    │
  │  │  Install  │  │  Cancel  │                    │
  │  └───────────┘  └──────────┘                    │
  │                                                  │
  └──────────────────────────────────────────────────┘
```

### List Skills

```
User: list skills

Levsha: Here are your installed skills:

  ┌─ Installed Skills ─────────────────────────────────────┐
  │                                                         │
  │  Built-in                                               │
  │  ─────────────────────────────────────────────────────  │
  │  📦 package-manager   v0.1.0   Manage system packages  │
  │  ℹ️  sysinfo           v0.1.0   System information      │
  │                                                         │
  │  Installed                                              │
  │  ─────────────────────────────────────────────────────  │
  │  🖼  image-editor      v1.2.0   Edit and resize images  │
  │  📁 filesystem         v0.1.0   File management         │
  │                                                         │
  └─────────────────────────────────────────────────────────┘
```

---

## 2. Skill Card Styling

Skill information is displayed as styled cards within the chat message flow.

### Card Layout

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Card cornerRadius | 8 |
| Card shadow | 0 1px 3px rgba(58,50,40,0.06) |
| Card padding | 16px |
| Skill name header | IBM Plex Sans 15px, weight 600, `$text-primary` (#3A3228) |
| Description | IBM Plex Sans 14px, weight 400, `$text-secondary` (#6B5D4F) |
| Metadata labels | IBM Plex Sans 12px, weight 500, `$text-tertiary` (#8C7E6E) |
| Metadata values | IBM Plex Sans 12px, weight 400, `$text-primary` |
| Tags | IBM Plex Sans 11px, `$bg-secondary` (#F5F1EA) pill, `$text-secondary` text |
| Install button | `$accent-copper` (#C67A52) bg, white text, cornerRadius 6 |
| Cancel button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |
| Remove button | `$danger-bg` (#FFF5F0) bg, `$accent-copper` text, cornerRadius 6 |

### Skill List Table

| Element | Style |
|---------|-------|
| Section header | IBM Plex Sans 13px, weight 600, `$text-tertiary`, uppercase |
| Skill name | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Version | IBM Plex Mono 12px, `$text-tertiary` |
| Description | IBM Plex Sans 13px, `$text-secondary` |
| Row separator | 1px `$border-primary` (#EBE6DC) |
| Built-in badge | `$accent-green` (#62B37B) dot indicator |

---

## 3. Installation Progress

During skill installation, progress is shown inline in the chat.

```
Levsha: Installing image-editor...

  ✓ Cloning repository
  ✓ Validating manifest
  ▶ Installing dependencies (ImageMagick)
  ○ Copying skill files
  ○ Loading skill

  ████████████░░░░░░░░ 60%
```

### Progress Styling

| Element | Style |
|---------|-------|
| Completed step (✓) | `$accent-green` (#62B37B), Lucide `check` |
| Active step (▶) | `$accent-copper` (#C67A52), Lucide `loader` (animated) |
| Pending step (○) | `$text-tertiary` (#8C7E6E) |
| Step text | IBM Plex Sans 13px, `$text-primary` |
| Progress bar fill | `$accent-copper` |
| Progress bar track | `$bg-secondary` |

---

## 4. Error States

### Invalid Manifest

```
Levsha: I couldn't install that skill — the manifest
is invalid:

  ┌─ Error ────────────────────────────────────────┐
  │  ⚠ Missing required field: "description"        │
  │                                                  │
  │  The skill at github.com/user/bad-skill has     │
  │  an invalid skill.yaml manifest.                 │
  └──────────────────────────────────────────────────┘
```

### Network Error

```
Levsha: I couldn't reach the skill repository.
Check your network connection and try again.
```

### Error Card Styling

| Element | Style |
|---------|-------|
| Error container | `$danger-bg` (#FFF5F0) bg, 1px `$accent-copper` border |
| Warning icon | Lucide `alert-circle`, `$accent-copper` |
| Error title | IBM Plex Sans 14px, weight 600, `$accent-copper` |
| Error body | IBM Plex Sans 13px, `$text-secondary` |

---

## 5. Search Results

```
User: search skills for network

Levsha: I found 3 skills matching "network":

  ┌─ Available Skills ─────────────────────────────────────────┐
  │                                                             │
  │  network-config     v0.1.0   Configure Wi-Fi, IP, DNS      │
  │                     levsha   [network] [wifi] [core]        │
  │                                                             │
  │  network-monitor    v0.3.1   Monitor bandwidth and traffic  │
  │                     community [network] [monitoring]        │
  │                                                             │
  │  firewall           v0.2.0   Manage iptables/nftables rules │
  │                     community [network] [security]          │
  │                                                             │
  └─────────────────────────────────────────────────────────────┘

  Say "install <name>" to install any of these.
```

---

## 6. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in skill management messages |
| Lucide `package` | Skill card icon |
| Lucide `download` | Install action icon |
| Lucide `trash-2` | Remove action icon |
| Lucide `refresh-cw` | Update action icon |
| Lucide `search` | Search action icon |
| Lucide `check` | Completed installation step |
| Lucide `alert-circle` | Error states |

---

## 7. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Skill cards, progress indicators, confirmation dialogs render in chat. |
| Intelligence Engine (03) | Modified | Skill management tools dispatched by the engine. |
| Skills System (04) | Upstream | Uses existing skill loading framework. |
