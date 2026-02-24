# 23 — GPU-Accelerated Inference: Design Specification

**Module:** GPU-Accelerated Inference (L1 + L2)
**Phase:** 3

---

## 1. Architecture Overview

```
┌───────────────────────────────────────────────────────────┐
│                    Chat Shell (L3)                          │
│                                                            │
│  Status bar: ▸ llama-3-8b (GPU) · connected · 14:32       │
│                            ↑ GPU/CPU indicator              │
│                                                            │
│  Performance: "12.4 tokens/s" after each local response    │
│                                                            │
└──────────────────────────────┬─────────────────────────────┘
                               │ IPC
┌──────────────────────────────▼─────────────────────────────┐
│                Intelligence Engine (L2)                      │
│                                                             │
│  GPU Manager → Auto-Configurator → llama-server (GPU)      │
│             → VRAM Monitor → Status/Warnings                │
│             → GPU Info → sysinfo skill                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Status Bar GPU Indicator

The status bar backend display (from module 14) is extended to show GPU/CPU mode.

### Backend Indicators with GPU Info

| State | Status Bar Display | Color |
|-------|-------------------|-------|
| GPU inference (local) | `▸ llama-3-8b (GPU)` | `$text-secondary` (#6B5D4F) |
| CPU inference (local) | `▸ llama-3-8b (CPU)` | `$text-secondary` |
| Cloud active | `▸ claude-sonnet` | `$text-secondary` |
| Auto mode, GPU available | `▸ auto (GPU)` | `$text-secondary` |
| GPU error / fallback | `▸ llama-3-8b (CPU ⚠)` | `$accent-gold` (#D4A853) |

### GPU Indicator Styling

| Element | Style |
|---------|-------|
| GPU label | IBM Plex Sans 12px, weight 400, `$text-secondary` |
| GPU label parentheses | `$text-tertiary` (#8C7E6E) |
| Warning icon (⚠) | `$accent-gold` (#D4A853) |

---

## 3. GPU Info Display

When the user asks "what GPU do I have?" or "show GPU info", the sysinfo skill returns a GPU info card.

### GPU Info Card

```
User: what GPU do I have?

Levsha: Here's your GPU information:

  ┌─ GPU Info ───────────────────────────────────────┐
  │                                                    │
  │  NVIDIA GeForce RTX 3060                           │
  │                                                    │
  │  VRAM         12 GB (8.2 GB used, 3.8 GB free)    │
  │  Driver       535.183.01                           │
  │  CUDA         12.2                                 │
  │  Temperature  62°C                                 │
  │  Utilization  45%                                  │
  │  Power        120W / 170W                          │
  │                                                    │
  │  ████████████████░░░░░░░░ 68% VRAM                │
  │                                                    │
  │  LLM: llama-3-8b (Q4_K_M, 33 layers on GPU)      │
  │  Speed: ~32 tokens/s                               │
  │                                                    │
  └────────────────────────────────────────────────────┘
```

### GPU Info Card Styling

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Card cornerRadius | 8 |
| GPU name | IBM Plex Sans 16px, weight 600, `$text-primary` (#3A3228) |
| Label column | IBM Plex Sans 13px, `$text-tertiary` (#8C7E6E), 120px wide |
| Value column | IBM Plex Mono 13px, weight 500, `$text-primary` |
| VRAM bar (used) | `$accent-copper` (#C67A52), 6px height |
| VRAM bar (free) | `$bg-secondary` (#F5F1EA), 6px height |
| VRAM bar cornerRadius | 3 |
| VRAM label | IBM Plex Mono 12px, `$text-tertiary` |
| LLM info | IBM Plex Sans 12px, `$text-secondary` (#6B5D4F) |
| Speed value | IBM Plex Mono 13px, weight 500, `$accent-green` (#62B37B) |

---

## 4. Performance Metrics Display

After each locally-generated response, the performance metrics are shown below the message.

### Performance Line

```
┌─ Levsha ──────────────────────────────────────────┐
│                                                     │
│  Here are the installed packages:                   │
│  vim 9.0, git 2.43, curl 8.5...                    │
│                                                     │
│                    llama-3-8b · Q4_K_M · 31.2 tk/s │
└─────────────────────────────────────────────────────┘
```

### Performance Line Styling

| Element | Style |
|---------|-------|
| Container | Right-aligned, 4px padding-top |
| Model name | IBM Plex Mono 11px, `$text-tertiary` (#8C7E6E) |
| Separator (·) | `$text-tertiary`, 6px horizontal margin |
| Quantization | IBM Plex Mono 11px, `$text-tertiary` |
| Speed | IBM Plex Mono 11px, weight 500, `$accent-green` (#62B37B) |
| Speed (slow, <10 tk/s) | IBM Plex Mono 11px, weight 500, `$accent-gold` (#D4A853) |
| Speed (very slow, <3 tk/s) | IBM Plex Mono 11px, weight 500, `$accent-copper` (#C67A52) |

Performance line only appears for local model responses, not cloud responses.

---

## 5. GPU Configuration via Chat

### Set GPU Layers

```
User: set GPU layers to 32

Levsha: GPU layers updated to 32.
        Restarting llama-server with new configuration...

        ▸ llama-3-8b (GPU, 32/33 layers)
        Previous: auto (33 layers)
```

### Force CPU Mode

```
User: use CPU only for inference

Levsha: Switched to CPU-only inference.
        GPU acceleration is disabled.

        ▸ llama-3-8b (CPU)

        Say "use GPU for inference" to re-enable.
```

### GPU Recommendation

```
User: what model should I use?

Levsha: Based on your GPU (RTX 3060, 12 GB VRAM):

  ┌─ Recommended Models ─────────────────────────────┐
  │                                                    │
  │  ✓ llama-3-8b (Q4_K_M)     4.5 GB   Fastest      │
  │    All 33 layers on GPU · ~32 tokens/s             │
  │                                                    │
  │  ◎ llama-3-8b (Q5_K_M)     5.5 GB   Best balance │
  │    All 33 layers on GPU · ~28 tokens/s             │
  │                                                    │
  │  ○ llama-3-13b (Q4_K_M)    7.4 GB   Smarter      │
  │    28/40 layers on GPU · ~15 tokens/s              │
  │                                                    │
  │  △ llama-3-13b (Q5_K_M)    9.0 GB   Tight fit    │
  │    22/40 layers on GPU · ~10 tokens/s              │
  │                                                    │
  │  Available VRAM: 11.5 GB (500 MB reserved)         │
  │                                                    │
  └────────────────────────────────────────────────────┘
```

### Recommendation Card Styling

| Element | Style |
|---------|-------|
| Active model icon (✓) | `$accent-green` (#62B37B) |
| Recommended icon (◎) | `$accent-copper` (#C67A52) |
| Available icon (○) | `$text-tertiary` (#8C7E6E) |
| Tight fit icon (△) | `$accent-gold` (#D4A853) |
| Model name | IBM Plex Mono 13px, weight 500, `$text-primary` (#3A3228) |
| Size | IBM Plex Mono 12px, `$text-tertiary` |
| Label (Fastest, etc.) | IBM Plex Sans 12px, weight 500 |
| GPU layer info | IBM Plex Sans 12px, `$text-secondary` (#6B5D4F) |
| Speed estimate | IBM Plex Mono 12px, `$text-secondary` |
| VRAM note | IBM Plex Sans 12px, `$text-tertiary` |

---

## 6. VRAM Warning Toast

When VRAM utilization exceeds the configured threshold (default 90%).

### Warning Toast

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  [chat messages above]                                   │
│                                                          │
│  ┌────────────────────────────────────────────────────┐  │
│  │  ⚠ VRAM usage is high (92%). Performance may       │  │
│  │  degrade. Consider using a smaller model or         │  │
│  │  reducing GPU layers.                               │  │
│  └────────────────────────────────────────────────────┘  │
│                                                          │
│  [chat continues below]                                  │
│                                                          │
```

### Warning Toast Styling

| Element | Style |
|---------|-------|
| Toast background | `$accent-gold` (#D4A853) at 10% opacity |
| Toast border | 1px `$accent-gold` |
| Toast cornerRadius | 8 |
| Warning icon (⚠) | Lucide `alert-triangle`, `$accent-gold` |
| Text | IBM Plex Sans 13px, `$text-primary` (#3A3228) |
| Auto-dismiss | 10 seconds (or user dismisses) |

---

## 7. Multi-GPU Display

When multiple GPUs are detected:

```
User: show GPU allocation

Levsha: Model layers are distributed across 2 GPUs:

  ┌─ GPU Allocation ─────────────────────────────────┐
  │                                                    │
  │  GPU 0: NVIDIA RTX 3060 (12 GB)                   │
  │  ████████████████████████░░░ 80% · Layers 0–26   │
  │  9.6 GB / 12 GB VRAM                              │
  │                                                    │
  │  GPU 1: NVIDIA RTX 2060 (6 GB)                    │
  │  ██████████████░░░░░░░░░░░░ 55% · Layers 27–40   │
  │  3.3 GB / 6 GB VRAM                               │
  │                                                    │
  │  Total: 40/40 layers on GPU                        │
  │  Tensor split: 0.67, 0.33                          │
  │                                                    │
  └────────────────────────────────────────────────────┘
```

---

## 8. Assets Used

| Asset | Usage |
|-------|-------|
| Lucide `cpu` | CPU inference indicator |
| Lucide `zap` | GPU inference indicator |
| Lucide `alert-triangle` | VRAM warning toast |
| Lucide `activity` | Performance metrics |
| Lucide `thermometer` | GPU temperature in sysinfo |
| Lucide `hard-drive` | VRAM usage indicator |

---

## 9. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | GPU/CPU status bar indicator, performance metrics, VRAM warning toast. |
| Intelligence Engine (03) | Modified | GPU manager state, auto-configuration results. |
| System Info Skill (06) | Modified | `sys_gpu` tool output rendering. |
| Local LLM (14) | Extended | GPU-aware status bar display. |
