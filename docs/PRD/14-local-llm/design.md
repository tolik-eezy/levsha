# 14 — Local LLM Support: Design Specification

**Module:** Local LLM + Intelligent Backend Routing
**Phase:** 2

---

## 1. Architecture Overview

```
┌───────────────────────────────────────────────────────────┐
│                    Chat Shell (L3)                         │
│                                                           │
│  Status bar shows active backend:                         │
│  ▸ claude-sonnet  |  ▸ llama-3-8b (local)  |  ▸ auto    │
│                                                           │
└──────────────────────────┬────────────────────────────────┘
                           │ IPC
┌──────────────────────────▼────────────────────────────────┐
│                Intelligence Engine (L2)                    │
│                                                           │
│  ┌────────────────────────────────────────────────────┐  │
│  │              Backend Router                         │  │
│  │                                                     │  │
│  │  Mode: auto | cloud | local                        │  │
│  │  Rules: message length, tool complexity,            │  │
│  │         self-improvement detection                  │  │
│  └────────┬──────────────────────┬────────────────────┘  │
│           │                      │                        │
│  ┌────────▼────────┐    ┌───────▼─────────┐             │
│  │  Cloud Client    │    │  Local Client   │             │
│  │  (Anthropic API) │    │  (OpenAI compat)│             │
│  └────────┬────────┘    └───────┬─────────┘             │
│           │                      │                        │
└───────────┼──────────────────────┼────────────────────────┘
            │                      │
            ▼                      ▼
      Anthropic API          llama-server
      (internet)             (localhost:8080)
```

---

## 2. Status Bar Backend Display

The status bar communicates which backend is active and the current routing mode.

### Backend Indicators

| Mode | Status Bar Display | Color |
|------|-------------------|-------|
| Cloud (active) | `▸ claude-sonnet` | `$text-secondary` (#6B5D4F) |
| Local (active) | `▸ llama-3-8b` | `$text-secondary` |
| Auto mode | `▸ auto` | `$text-secondary` |
| Offline fallback | `▸ llama-3-8b (offline)` | `$accent-gold` (#D4A853) |
| No backend available | `▸ disconnected` | `$accent-copper` (#C67A52) |

### Status Bar Layout

```
▸ auto · connected · 14:32
         ↑ mode   ↑ connection   ↑ clock
```

When a message is answered, the backend that answered is briefly shown:

```
▸ auto (→ local) · connected · 14:32
```

This fades back to `▸ auto` after 3 seconds.

---

## 3. Routing Transparency

Users can ask which backend answered their last query.

```
User: which model answered that?

Levsha: Your last message was answered by the local
        model (llama-3-8b, running on this machine).

        Current routing mode: auto
        • Simple queries → local model
        • Complex queries → cloud (Claude Sonnet)
        • Self-improvement → always cloud

        Say "use cloud only" or "use local only"
        to override automatic routing.
```

### Routing Info Card

```
┌─ Backend Routing ──────────────────────────────┐
│                                                  │
│  Mode: auto                                      │
│                                                  │
│  ┌─────────────┬────────────────────────────┐   │
│  │ Local       │ Simple queries, file ops,   │   │
│  │ llama-3-8b  │ system info, package search │   │
│  ├─────────────┼────────────────────────────┤   │
│  │ Cloud       │ Complex reasoning, multi-   │   │
│  │ claude-son  │ step tasks, self-improve    │   │
│  └─────────────┴────────────────────────────┘   │
│                                                  │
│  Last query: local (3ms)                         │
│                                                  │
└──────────────────────────────────────────────────┘
```

### Routing Card Styling

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Card cornerRadius | 8 |
| Section header | IBM Plex Sans 14px, weight 600, `$text-primary` |
| Table header | IBM Plex Sans 12px, weight 500, `$text-tertiary` |
| Table row | IBM Plex Sans 13px, `$text-primary`/`$text-secondary` |
| Table border | 1px `$border-primary` |
| "Local" label | IBM Plex Mono 12px, weight 500, `$accent-green` |
| "Cloud" label | IBM Plex Mono 12px, weight 500, `$accent-copper` |

---

## 4. Model Management Interface

Model management is entirely conversational.

### Download Progress

```
User: download llama 3 70b

Levsha: Downloading Llama 3 70B (Q4 quantization, ~38 GB)...

        ████████████░░░░░░░░ 60%
        23.1 GB / 38.5 GB — 12.4 MB/s — ~21 min remaining

        This will be saved to /var/lib/levsha/models/
```

### Model List

```
User: what models are available?

Levsha: Here are your local models:

  ┌─ Local Models ─────────────────────────────────────┐
  │                                                     │
  │  ✓ llama-3-8b-q4      4.3 GB   Active              │
  │  ○ llama-3-70b-q4    38.5 GB   Downloaded          │
  │                                                     │
  │  Available for download:                            │
  │  ─────────────────────────────────────────────────  │
  │  ○ mistral-7b-q4      3.8 GB                       │
  │  ○ codellama-13b-q4   7.4 GB                       │
  │  ○ phi-3-mini-q4      2.3 GB                       │
  │                                                     │
  │  Total disk used: 42.8 GB                           │
  │  Available: 78.2 GB                                 │
  │                                                     │
  └─────────────────────────────────────────────────────┘
```

### Model Card Styling

| Element | Style |
|---------|-------|
| Active model icon (✓) | `$accent-green` (#62B37B) |
| Downloaded icon (○) | `$text-tertiary` (#8C7E6E) |
| Model name | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Size | IBM Plex Mono 12px, `$text-tertiary` |
| Status | IBM Plex Sans 12px, weight 500 |
| Active status color | `$accent-green` |
| Download progress bar | `$accent-copper` fill, `$bg-secondary` track, 6px height |
| Speed/time estimate | IBM Plex Mono 12px, `$text-tertiary` |

---

## 5. Offline Mode Indicator

When network is unavailable and the system falls back to local inference.

### Offline Banner

```
┌──────────────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────────────┐  │
│  │  📡 Running locally — cloud API is unreachable.    │  │
│  │  I'll use the local model until connectivity       │  │
│  │  returns.                                           │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  [normal chat continues below]                           │
│                                                          │
```

### Offline Banner Styling

| Element | Style |
|---------|-------|
| Banner background | `$accent-gold` (#D4A853) at 10% opacity |
| Banner border | 1px `$accent-gold` |
| Banner cornerRadius | 8 |
| Icon | Lucide `wifi-off`, `$accent-gold` |
| Text | IBM Plex Sans 13px, `$text-primary` |

### No Model + No Network

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│                    ┌──────────┐                          │
│                    │  (flea   │                          │
│                    │  logo)   │                          │
│                    └──────────┘                          │
│                                                          │
│  I need either an internet connection or a local         │
│  model to function.                                      │
│                                                          │
│  • Check your network connection, or                     │
│  • Say "download a local model" when you're              │
│    back online to enable offline mode.                    │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## 6. Assets Used

| Asset | Usage |
|-------|-------|
| `flea logo.png` | No-backend-available state |
| `logo2.png` | Levsha avatar in routing info messages |
| Lucide `wifi-off` | Offline indicator |
| Lucide `wifi` | Online indicator |
| Lucide `cpu` | Local model indicator |
| Lucide `cloud` | Cloud model indicator |
| Lucide `download` | Model download |
| Lucide `trash-2` | Model delete |
| Lucide `refresh-cw` | Model switch |

---

## 7. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Status bar backend indicator, offline banner. |
| Intelligence Engine (03) | Modified | Router layer, dual API client. |
| API Key Config (13) | Sibling | Cloud requires key; local does not. |
