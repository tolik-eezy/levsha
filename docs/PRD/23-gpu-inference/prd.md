# 23 — GPU-Accelerated Inference: Product Requirements

**Module:** GPU-Accelerated Inference (L1 + L2)
**Phase:** 3
**Status:** Draft

---

## 1. Overview

Phase 2 introduced local LLM inference via llama.cpp, running on CPU only. Phase 3 adds GPU acceleration for dramatically faster local inference. The system auto-detects available GPUs (NVIDIA, AMD, Intel), configures llama.cpp to offload model layers to GPU, and selects optimal quantization based on available VRAM. GPU acceleration also benefits whisper.cpp for faster speech-to-text (module 22).

---

## 2. Functional Requirements

### 2.1 GPU Auto-Detection (GI-01)

| Field | Value |
|-------|-------|
| **ID** | GI-01 |
| **Priority** | P0 |
| **Requirement** | The system detects available GPUs and their capabilities at startup. |

**Details:**

- **NVIDIA:** Detected via `nvidia-smi`. Extracts model name, VRAM, driver version, CUDA compute capability.
- **AMD:** Detected via `rocm-smi`. Extracts model name, VRAM, driver version.
- **Intel:** Detected via `vulkaninfo`. Extracts model name, VRAM (shared memory), Vulkan version.
- Detection runs once at boot and on user request ("detect GPUs").
- Results cached in engine state.
- If no GPU detected, system continues with CPU-only inference (Phase 2 behavior).

**Acceptance Criteria:**

- [ ] NVIDIA GPUs are detected with model, VRAM, and driver info.
- [ ] AMD GPUs are detected with model, VRAM, and driver info.
- [ ] Intel GPUs are detected with model and Vulkan info.
- [ ] System works correctly when no GPU is present (CPU fallback).

### 2.2 GPU-Accelerated llama.cpp (GI-02)

| Field | Value |
|-------|-------|
| **ID** | GI-02 |
| **Priority** | P0 |
| **Requirement** | llama-server uses GPU acceleration when a compatible GPU is available. |

**Details:**

- llama-server is started with `--n-gpu-layers N` where N depends on available VRAM.
- The appropriate llama.cpp backend is selected at runtime:
  - NVIDIA: CUDA backend.
  - AMD: ROCm backend (HIP).
  - Intel: Vulkan backend.
- Multiple backend binaries are shipped in the ISO; the correct one is selected based on detected GPU.
- If GPU initialization fails, falls back to CPU-only with a warning.

**Acceptance Criteria:**

- [ ] llama-server starts with GPU acceleration on supported hardware.
- [ ] Correct backend is selected for the detected GPU vendor.
- [ ] GPU acceleration failure falls back to CPU gracefully.
- [ ] Token generation speed improves significantly over CPU-only.

### 2.3 Auto-Configuration (GI-03)

| Field | Value |
|-------|-------|
| **ID** | GI-03 |
| **Priority** | P0 |
| **Requirement** | The system automatically selects optimal GPU layers and quantization based on available VRAM. |

**Details:**

VRAM-based auto-configuration:

| VRAM | Recommended Model | Quantization | GPU Layers |
|------|-------------------|-------------|------------|
| 4 GB | 7-8B parameter | Q4_K_M | 28-33 (all) |
| 6 GB | 7-8B parameter | Q5_K_M | 28-33 (all) |
| 8 GB | 13B parameter | Q4_K_M | ~28 |
| 12 GB | 13B parameter | Q5_K_M | 40 (all) |
| 16 GB | 30B parameter | Q4_K_M | ~40 |
| 24 GB+ | 70B parameter | Q4_K_M | ~60 |

- Auto-configuration runs on first GPU detection and when models are switched.
- User can override: "set GPU layers to 32" or "use Q5_K_M quantization".
- System reserves 500 MB of VRAM for other uses (compositor, whisper).

**Acceptance Criteria:**

- [ ] Optimal GPU layers are calculated from available VRAM.
- [ ] Recommended quantization matches VRAM capacity.
- [ ] User can override automatic settings.
- [ ] VRAM reservation prevents over-allocation.

### 2.4 Status Reporting (GI-04)

| Field | Value |
|-------|-------|
| **ID** | GI-04 |
| **Priority** | P0 |
| **Requirement** | GPU information is visible in the sysinfo skill and status bar. |

**Details:**

- Sysinfo skill (`sys_gpu` tool) reports: GPU model, VRAM total/used/free, driver version, temperature, utilization.
- Status bar shows whether the active model is running on GPU or CPU.
- "What GPU do I have?" → detailed GPU info card.
- Performance metrics (tokens/s) shown after each local model response.

**Acceptance Criteria:**

- [ ] `sys_gpu` tool returns GPU information.
- [ ] Status bar shows GPU/CPU indicator for the active model.
- [ ] Performance metrics (tokens/s) are displayed.

### 2.5 Quantization Support (GI-05)

| Field | Value |
|-------|-------|
| **ID** | GI-05 |
| **Priority** | P1 |
| **Requirement** | Users can select between different quantization levels for models. |

**Details:**

Supported quantizations:

| Quantization | Bits | Size (8B model) | Quality |
|-------------|------|-----------------|---------|
| Q4_K_M | 4-bit | ~4.5 GB | Good — best speed/quality balance |
| Q5_K_M | 5-bit | ~5.5 GB | Better — slightly more accurate |
| Q8_0 | 8-bit | ~8.5 GB | Best — near full-precision |

- "Use Q5 quantization for this model" → downloads the Q5_K_M variant if not present.
- "What quantization am I using?" → reports current quantization.
- Model download UI shows available quantization options.

**Acceptance Criteria:**

- [ ] Multiple quantization variants can be downloaded.
- [ ] User can switch quantization via chat.
- [ ] Current quantization is reported.

### 2.6 VRAM Monitoring (GI-06)

| Field | Value |
|-------|-------|
| **ID** | GI-06 |
| **Priority** | P1 |
| **Requirement** | The system monitors VRAM usage and warns when nearing the limit. |

**Details:**

- VRAM usage polled every 5 seconds while a GPU model is loaded.
- Warning at 90% VRAM utilization: toast notification in chat.
- If VRAM exhausted and llama-server crashes, engine automatically restarts with fewer GPU layers.
- "How much VRAM is free?" → reports current VRAM usage.

**Acceptance Criteria:**

- [ ] VRAM usage is monitored periodically.
- [ ] Warning shown at 90% VRAM utilization.
- [ ] Automatic recovery from VRAM exhaustion.
- [ ] VRAM usage queryable via chat.

### 2.7 Multi-GPU Support (GI-07)

| Field | Value |
|-------|-------|
| **ID** | GI-07 |
| **Priority** | P1 |
| **Requirement** | Model layers can be distributed across multiple GPUs. |

**Details:**

- When multiple GPUs are detected, llama-server is started with `--tensor-split` and `--main-gpu` flags.
- Default split: proportional to VRAM (e.g., 16 GB + 8 GB = 2:1 ratio).
- User override: "split model 60/40 across GPUs".
- "Show GPU allocation" → reports which layers are on which GPU.

**Acceptance Criteria:**

- [ ] Multiple GPUs are detected and reported.
- [ ] Model layers are distributed across GPUs.
- [ ] Split ratio is based on VRAM proportions.
- [ ] User can override the split.

### 2.8 Whisper GPU Acceleration (GI-08)

| Field | Value |
|-------|-------|
| **ID** | GI-08 |
| **Priority** | P2 |
| **Requirement** | whisper.cpp can use GPU acceleration for faster speech-to-text. |

**Details:**

- whisper-server started with GPU flags when a GPU is available and voice mode is active.
- GPU whisper uses the same backend selection as llama.cpp (CUDA/ROCm/Vulkan).
- VRAM budget shared: whisper reserves a small portion (100-200 MB), rest for LLM.
- GPU whisper is optional — if VRAM is insufficient, whisper runs on CPU.

**Acceptance Criteria:**

- [ ] whisper-server starts with GPU acceleration when available.
- [ ] Transcription speed improves with GPU.
- [ ] VRAM budget is shared with llama-server.
- [ ] Falls back to CPU whisper if VRAM insufficient.

---

## 3. Architecture

```
┌───────────────────────────────────────────────────────────┐
│                Intelligence Engine (L2)                     │
│                                                            │
│  ┌──────────────────────────────────────────────────┐     │
│  │            GPU Manager                             │     │
│  │                                                    │     │
│  │  ┌─────────────┐  ┌──────────────┐               │     │
│  │  │ GPU Detector │  │ VRAM Monitor │               │     │
│  │  └──────┬──────┘  └──────┬───────┘               │     │
│  │         │                 │                        │     │
│  │  ┌──────▼─────────────────▼───────┐               │     │
│  │  │      Auto-Configurator         │               │     │
│  │  │  (VRAM → layers + quant)       │               │     │
│  │  └──────┬─────────────────────────┘               │     │
│  │         │                                          │     │
│  └─────────┼──────────────────────────────────────────┘     │
│            │                                                │
│  ┌─────────▼──────────┐   ┌────────────────────────┐      │
│  │  Local Client       │   │  STT Client            │      │
│  │  (llama-server)     │   │  (whisper-server)      │      │
│  └─────────┬──────────┘   └────────────┬───────────┘      │
│            │                            │                   │
└────────────┼────────────────────────────┼───────────────────┘
             │                            │
             ▼                            ▼
       llama-server                 whisper-server
       (--n-gpu-layers N)          (--gpu)
       CUDA | ROCm | Vulkan        CUDA | ROCm | Vulkan
```

---

## 4. Performance Targets

| Model | Quantization | Hardware | Target (tokens/s) |
|-------|-------------|----------|-------------------|
| 8B | Q4_K_M | RTX 3060 (12 GB) | >30 |
| 8B | Q4_K_M | RX 6700 XT (12 GB) | >25 |
| 8B | Q4_K_M | CPU-only (8 cores) | ~5 |
| 13B | Q4_K_M | RTX 3060 (12 GB) | >15 |
| 13B | Q4_K_M | RTX 4090 (24 GB) | >40 |
| 70B | Q4_K_M | RTX 4090 (24 GB) | >10 |
| Whisper base | -- | RTX 3060 | <0.5s for 10s audio |
| Whisper base | -- | CPU-only | ~3s for 10s audio |

---

## 5. New System Requirements

| Package | Purpose | Size |
|---------|---------|------|
| `llama.cpp` (CUDA build) | NVIDIA GPU inference | ~15 MB |
| `llama.cpp` (ROCm build) | AMD GPU inference | ~15 MB |
| `llama.cpp` (Vulkan build) | Intel/fallback GPU inference | ~12 MB |
| NVIDIA driver (user-installed) | NVIDIA GPU support | ~300 MB |
| ROCm runtime (user-installed) | AMD GPU support | ~500 MB |
| Vulkan driver (system) | Intel/generic GPU support | ~50 MB |
| `nvidia-smi` | NVIDIA GPU detection | Included with driver |
| `rocm-smi` | AMD GPU detection | Included with ROCm |
| `vulkaninfo` | Vulkan GPU detection | ~1 MB |

**Impact:** ISO size increases by ~45 MB (llama.cpp multi-backend builds + vulkaninfo). GPU drivers are not bundled — installed on demand via package manager skill or user instruction.

---

## 6. Configuration

Modified config entries in `/etc/levsha/config.toml`:

```toml
[local_model]
# Existing fields from module 14...
gpu_layers = -1             # -1 = auto (calculate from VRAM), 0 = CPU-only, N = specific count
gpu_backend = "auto"        # "auto", "cuda", "rocm", "vulkan", "cpu"

[gpu]
enabled = true              # Master GPU toggle
auto_configure = true       # Auto-select layers and quantization
vram_reserve_mb = 500       # VRAM reserved for system (compositor, whisper)
monitor_interval_s = 5      # VRAM monitoring poll interval
warn_threshold = 0.90       # VRAM usage warning threshold (0.0 - 1.0)
main_gpu = 0                # Primary GPU index (for multi-GPU)
tensor_split = ""           # Custom split ratio (e.g., "0.67,0.33")
```

---

## 7. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Local LLM (14) | Extended | GPU flags added to llama-server startup, backend selection logic. |
| Intelligence Engine (03) | Modified | GPU manager module, VRAM monitoring, auto-configuration. |
| Base System (01) | Modified | GPU driver packages, llama.cpp multi-backend builds, vulkaninfo. |
| System Info Skill (06) | Modified | New `sys_gpu` tool for GPU information reporting. |
| Chat Shell (02) | Modified | GPU/CPU indicator in status bar, performance metrics display. |
| Voice I/O (22) | Optional | GPU acceleration for whisper-server. |

---

## 8. Out of Scope

- Model training or fine-tuning on GPU.
- GPU compute for non-LLM tasks (image generation, video encoding, etc.).
- Cloud GPU offloading (running inference on remote GPU servers).
- GPU overclocking or power management.
- Non-x86 GPU support (ARM Mali, Apple Silicon — Levsha targets x86 Linux).
- Automatic GPU driver installation (drivers are installed via package manager skill or manually).
