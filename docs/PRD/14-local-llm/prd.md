# 14 — Local LLM Support: Product Requirements

**Module:** Local LLM + Intelligent Backend Routing
**Phase:** 2
**Status:** Draft

---

## 1. Overview

Phase 2 adds local LLM inference alongside the existing cloud API backend. The system can run a bundled language model via `llama.cpp`, providing offline capability and reducing API costs. An intelligent routing layer decides whether to use the local model or the cloud API based on task complexity.

---

## 2. Functional Requirements

### 2.1 Local Model Runtime (LM-01)

| Field | Value |
|-------|-------|
| **ID** | LM-01 |
| **Priority** | P0 |
| **Requirement** | The system can run a local language model via llama.cpp for offline inference. |

**Details:**

- **Runtime:** `llama.cpp` compiled as a server (`llama-server`) providing an OpenAI-compatible API.
- **Bundled model:** A small, capable model (e.g., Llama 3 8B Q4 quantization, ~4.5 GB) is included in the ISO or downloaded on first use.
- **Model storage:** `/var/lib/levsha/models/`.
- **Server management:** `llama-server` runs as a systemd service, started on demand.
- **API compatibility:** llama.cpp's server exposes an OpenAI-compatible API. The engine's API client is extended to support both Anthropic and OpenAI-compatible endpoints.
- **Transport:** Phase 2 uses HTTP over localhost. Future optimization: `llama-server` supports Unix domain sockets (`--host /path/to/socket.sock`), which eliminates loopback network overhead while keeping the same OpenAI-compatible protocol.

**Acceptance Criteria:**

- [ ] `llama-server` starts and serves a local model.
- [ ] Engine can send requests to the local model and receive streamed responses.
- [ ] Local inference works without internet connectivity.
- [ ] Model files are stored in `/var/lib/levsha/models/`.

### 2.2 Intelligent Backend Routing (LM-02)

| Field | Value |
|-------|-------|
| **ID** | LM-02 |
| **Priority** | P0 |
| **Requirement** | The engine intelligently routes requests to either the local model or the cloud API based on task complexity. |

**Details:**

Routing heuristics:

| Route to | When |
|----------|------|
| **Local model** | Simple queries (system info, file listing, package search), short factual answers, command generation for well-understood tasks. |
| **Cloud API** | Complex reasoning, multi-step tool use, self-improvement tasks, ambiguous or creative requests, long-form responses. |
| **Cloud API (forced)** | User explicitly requests it ("use Claude for this"), self-improvement system always uses cloud. |
| **Local model (forced)** | User explicitly requests it ("use local model"), no internet available. |

Routing decision is made by a lightweight classifier (rule-based in Phase 2, potentially model-assisted later):

- Message length < 50 tokens → likely simple → local
- Contains tool-use patterns → check tool complexity → simple tools local, complex chains cloud
- Self-improvement context → always cloud
- User preference override → respected immediately

**Acceptance Criteria:**

- [ ] Simple queries are routed to the local model.
- [ ] Complex queries are routed to the cloud API.
- [ ] User can force a specific backend.
- [ ] Self-improvement always uses cloud API.
- [ ] Routing is transparent — user can ask "which model answered this?"

### 2.3 Backend Status and Switching (LM-03)

| Field | Value |
|-------|-------|
| **ID** | LM-03 |
| **Priority** | P0 |
| **Requirement** | Status bar shows which backend is active. User can switch backends via chat. |

**Details:**

- Status bar shows: `▸ claude-sonnet` (cloud) or `▸ llama-3-8b (local)` or `▸ auto`.
- "Use local model only" → all requests go to local model.
- "Use cloud only" → all requests go to cloud API.
- "Use auto routing" → intelligent routing decides.
- "What model are you using?" → explains current routing mode and which backend answered the last query.

**Acceptance Criteria:**

- [ ] Status bar shows the active backend/mode.
- [ ] User can switch between local/cloud/auto via chat.
- [ ] Backend preference persists across restarts.

### 2.4 Model Management (LM-04)

| Field | Value |
|-------|-------|
| **ID** | LM-04 |
| **Priority** | P1 |
| **Requirement** | Users can download, list, and switch between local models via chat. |

**Details:**

- "What models are available?" → lists downloadable models.
- "Download Llama 3 70B" → downloads model to `/var/lib/levsha/models/`.
- "Switch to Llama 3 70B" → restarts llama-server with the new model.
- "How much space do my models use?" → shows disk usage.
- "Delete the 70B model" → removes model file.

**Acceptance Criteria:**

- [ ] Users can download models via chat.
- [ ] Users can switch between downloaded models.
- [ ] Model disk usage is reported.
- [ ] Model deletion works.

### 2.5 Offline Mode (LM-05)

| Field | Value |
|-------|-------|
| **ID** | LM-05 |
| **Priority** | P1 |
| **Requirement** | The system degrades gracefully when offline, falling back to the local model. |

**Details:**

- When network is unavailable:
  - Cloud API requests fail → engine automatically falls back to local model.
  - Status bar shows: `▸ llama-3-8b (offline)`.
  - User is informed: "I'm running locally — cloud API is unreachable."
  - When network returns, auto-routing resumes.
- If no local model is installed and network is down:
  - Chat Shell shows: "I need either an internet connection or a local model to function. Want me to help set up a local model when you're back online?"

**Acceptance Criteria:**

- [ ] Network loss triggers automatic fallback to local model.
- [ ] User is informed of the fallback.
- [ ] When network returns, normal routing resumes.
- [ ] No local model + no network produces a helpful message.

---

## 3. Architecture

```
┌─────────────────────────┐
│    Intelligence Engine   │
│                          │
│  ┌──────────────────┐   │
│  │  Backend Router   │   │
│  │  (rule-based)     │   │
│  └───────┬──────┬────┘   │
│          │      │        │
│     ┌────▼──┐ ┌─▼─────┐ │
│     │ Cloud │ │ Local  │ │
│     │ API   │ │ Model  │ │
│     │Client │ │ Client │ │
│     └───────┘ └────────┘ │
└─────────────────────────┘
         │           │
         ▼           ▼
   Anthropic    llama-server
   Claude API   (localhost:8080)
```

---

## 4. New System Requirements

| Package | Purpose | Size |
|---------|---------|------|
| `llama.cpp` (compiled) | Local inference server | ~10 MB |
| Bundled model (Llama 3 8B Q4) | Default local model | ~4.5 GB |
| `curl` | Model download | Already installed |

**Impact:** VM storage requirement increases to 20 GB minimum (16 GB + model files). VM RAM requirement increases to 4 GB minimum for local inference (8B model).

---

## 5. Configuration

New config entries:

```toml
[local_model]
enabled = true
server_binary = "/usr/bin/llama-server"
model_path = "/var/lib/levsha/models/llama-3-8b-q4.gguf"
models_dir = "/var/lib/levsha/models"
port = 8080                 # Ignored if socket_path is set
socket_path = ""            # Unix domain socket path (e.g., "/run/levsha/llama.sock"); if set, uses UDS instead of TCP
context_size = 4096
gpu_layers = 0          # CPU-only in Phase 2

[routing]
mode = "auto"            # "auto", "cloud", "local"
```

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Intelligence Engine (03) | Modified | Router layer, dual API client, backend switching. |
| API Key Config (13) | Sibling | Cloud API requires key; local model does not. |
| Chat Shell (02) | Modified | Status bar shows backend info. |
| Base System (01) | Modified | llama-server systemd service, model storage. |

---

## 7. Out of Scope

- GPU acceleration (Phase 3: requires GPU drivers and CUDA/ROCm).
- Model fine-tuning on local data (Phase 4).
- Multiple concurrent local model instances.
- Non-llama.cpp inference backends.
- Model training or distillation.
