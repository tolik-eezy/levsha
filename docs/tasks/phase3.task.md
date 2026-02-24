# Levsha OS — Phase 3 Implementation Plan

## Context

Phase 2 is complete: the OS can modify itself (self-improvement), install skills from Git repos, work offline with a local LLM, manage multiple conversation sessions, edit text/code, configure networks, and render content in a split-view panel. ~20,700 LOC of Rust across `chat-shell/` and `engine/`, 8 built-in skills with 72+ tools.

Phase 3 transforms Levsha OS from a keyboard-only tool into a **voice-enabled, community-powered platform**. Sessions become concurrent (background tasks), GPU acceleration unlocks performant local inference, and the community skill repository creates an ecosystem.

**Strategy:** Infrastructure first (concurrent sessions, notifications, GPU), then user-facing features (voice, community repo, skill wizard).

---

## Phase 3 Modules

| # | Module | PRD | Priority | Description |
|---|--------|-----|----------|-------------|
| 20 | Concurrent Sessions | `20-concurrent-sessions/` | P0 | Sessions run tasks in background; parallel execution. |
| 21 | Notification System | `21-notification-system/` | P0 | Alerts for background events; badges on sessions. |
| 22 | Voice Input/Output | `22-voice-io/` | P0 | whisper.cpp STT + TTS for hands-free interaction. |
| 23 | GPU-Accelerated Inference | `23-gpu-inference/` | P0 | GPU support for llama.cpp; faster local model. |
| 24 | Community Skill Repository | `24-community-skill-repo/` | P1 | Searchable skill registry with ratings and trust levels. |
| 25 | Skill Creation Wizard | `25-skill-creation-wizard/` | P1 | Guided skill building via chat conversation. |

---

## Dependency Graph

```
Phase 3.0 (Infrastructure)
    │
    ├── 23 — GPU-Accelerated Inference ─────────────┐
    │        (extends 14 — Local LLM)                │
    │                                                 │
    ├── 20 — Concurrent Sessions ───────────┐        │
    │        (extends 19 — Multi-Session)    │        │
    │                                        │        │
    └── 21 — Notification System ◄──────────┘        │
             (needs 20 for background events)         │
                                                      │
Phase 3.1 (User-Facing Features)                     │
    │                                                 │
    ├── 22 — Voice Input/Output ◄─────────────────────┘
    │        (benefits from 23 for local whisper)
    │
    ├── 24 — Community Skill Repository ◄──── (extends 11)
    │
    └── 25 — Skill Creation Wizard ◄─────────── (needs 24)
```

---

## Key Architectural Decisions

1. **Concurrent sessions via async task spawning** — Each session gets its own `tokio::task` for LLM requests and tool execution. A session can continue working while the user switches to another. Engine manages a task queue per session with cancellation support.

2. **Notification bus** — A pub/sub event bus (`tokio::broadcast`) inside the engine routes notifications from background sessions to the active Chat Shell view. Notifications are typed (task_complete, error, attention_needed) and persisted in SQLite for history.

3. **GPU detection at boot** — The engine probes for GPU devices (CUDA via `nvidia-smi`, ROCm via `rocm-smi`, Vulkan via `vulkaninfo`) at startup. llama.cpp is compiled with GPU backend flags. Config auto-selects the best available backend.

4. **whisper.cpp as systemd service** — Similar to the llama-server pattern: `whisper-server.service` runs on-demand, exposes a local HTTP endpoint. Chat Shell captures audio from PipeWire, streams to whisper-server, returns text. TTS uses `piper` for offline speech synthesis.

5. **Community registry as HTTP API** — Upgrade from Phase 2's Git-based `index.yaml` to a proper HTTP registry API. Skills have metadata (author, version, ratings, downloads, trust level). Registry server is a separate project; the client is built into the engine.

6. **Skill wizard as a meta-skill** — A built-in skill that uses the LLM to generate skill manifests, tool schemas, and prompt files interactively. The wizard validates output, offers to test tools, and can publish to the community registry.

7. **Split-view navigation for concurrent sessions** — The existing split-view panel gains a "session sidebar" mode: a narrow left panel showing active sessions with status indicators, replacing the overlay-only approach from Phase 2.

---

## Phase 3.0: Infrastructure (~5-7 hrs, 3 agents in parallel)

**Team:** `levsha-p3-infra`

These modules provide the foundation for all Phase 3 features.

### Track A — GPU-Accelerated Inference (`engine` + `base-system` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| GPU detection | `engine` | `engine/src/llm/gpu.rs` | Detect NVIDIA (CUDA), AMD (ROCm), Intel (Vulkan) GPUs at startup. Report capabilities and VRAM. |
| GPU-aware model loading | `engine` | `engine/src/llm/local_server.rs` | Pass GPU layers flag (`--n-gpu-layers`) to llama-server based on detected GPU and VRAM. |
| GPU config | `engine` | `engine/src/config.rs` | New `[gpu]` config section: `enabled = auto`, `backend = auto`, `layers = auto`, `vram_limit_mb`. |
| llama.cpp GPU builds | `base-system` | `infra/Containerfile`, kickstart | Ship llama.cpp compiled with CUDA + ROCm + Vulkan backends. Runtime selects correct one. |
| GPU status in sysinfo | `skills-dev` | `skills/built-in/sysinfo/tools/gpu_info.json` | New `gpu_info` tool: model, VRAM usage, temperature, driver version. |
| Performance monitoring | `engine` | `engine/src/llm/gpu.rs` | Track tokens/second with GPU vs CPU. Report in status bar tooltip. |
| Quantization tools | `engine` | `engine/src/llm/model_manager.rs` | Support loading quantized models (Q4_K_M, Q5_K_M, Q8_0). Recommend quantization based on VRAM. |
| Status bar GPU indicator | `chat-shell` | `chat-shell/src/status_bar.rs` | Show GPU icon when GPU inference active: `▸ llama-3-8b (GPU)`. |
| VM GPU passthrough docs | `docs` | `docs/technical/gpu-passthrough.md` | Guide for enabling GPU passthrough in QEMU/VirtualBox for testing. |

**Deliverable:** Local LLM inference runs on GPU when available. Auto-detection, no manual config needed. 3-10x speedup for local inference.

### Track B — Concurrent Sessions (`engine` + `chat-shell` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Session task runner | `engine` | `engine/src/session/runner.rs` | Per-session async task runner. Spawns `tokio::task` for LLM requests and tool execution. Supports cancellation. |
| Background execution | `engine` | `engine/src/session/runner.rs` | When user switches away from a session mid-task, the task continues in background. Results are queued. |
| Task queue | `engine` | `engine/src/session/queue.rs` | Per-session task queue. Sequential execution within a session, parallel across sessions. Priority levels (user-initiated > auto > maintenance). |
| Session state machine | `engine` | `engine/src/session/mod.rs` | Session states: `idle`, `processing`, `waiting_confirmation`, `background`. Transitions tracked. |
| Cancellation support | `engine` | `engine/src/session/runner.rs` | `Ctrl+C` or "cancel" in chat cancels the active task in current session. Background tasks can be cancelled from session list. |
| Session sidebar | `chat-shell` | `chat-shell/src/session_sidebar.rs` | Narrow left panel (200px) showing active sessions with status icons: idle (○), processing (◐), complete (✓), error (✗). |
| Sidebar toggle | `chat-shell` | `chat-shell/src/keybindings.rs` | `Ctrl+B` toggles session sidebar. Hidden by default when only 1 session. Auto-shows when 2+ sessions exist. |
| IPC extensions | both | `engine/src/protocol.rs`, `engine/src/types.rs` | New messages: `SessionStatus`, `TaskProgress`, `TaskComplete`, `TaskCancelled`. |
| Config update | `engine` | `engine/src/config.rs` | New `[sessions]` fields: `max_concurrent_tasks = 3`, `background_enabled = true`. |
| Resource limiting | `engine` | `engine/src/session/runner.rs` | Limit total concurrent LLM requests (cloud: 3, local: 1). Queue excess requests. |

**Deliverable:** Sessions run tasks in the background. User can switch sessions while tasks execute. Session sidebar shows live status.

### Track C — Notification System (`engine` + `chat-shell` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Notification bus | `engine` | `engine/src/notifications/bus.rs` | `tokio::broadcast` channel for typed notifications. Publishers: session runner, skill installer, system events. Subscribers: Chat Shell, notification history. |
| Notification types | `engine` | `engine/src/notifications/types.rs` | Types: `TaskComplete`, `TaskError`, `AttentionNeeded`, `SkillInstalled`, `SystemAlert`, `SessionMessage`. Severity: info, warning, error. |
| Notification persistence | `engine` | `engine/src/notifications/store.rs` | SQLite `notifications` table: id, type, session_id, title, body, severity, read, created_at. |
| Toast renderer | `chat-shell` | `chat-shell/src/toast.rs` | Animated toast notifications (slide in from top-right, 3s auto-dismiss). Stacking for multiple. Click to navigate to source session. |
| Session badges | `chat-shell` | `chat-shell/src/session_sidebar.rs`, `chat-shell/src/session_list.rs` | Unread notification count badge on sessions in sidebar and list overlay. Clears on session focus. |
| Sound alerts | `chat-shell` | `chat-shell/src/audio.rs` | Optional notification sounds via PipeWire. Subtle chime for complete, alert for error. Mutable in config. |
| Notification history | `chat-shell` | `chat-shell/src/notification_panel.rs` | `Ctrl+Shift+N` opens notification history panel. List of all notifications with timestamps, read/unread. |
| L2→L3 notification messages | both | `engine/src/types.rs` | New messages: `NotificationPush`, `NotificationList`, `NotificationRead`, `NotificationClear`. |
| Config update | `engine` | `engine/src/config.rs` | New `[notifications]` section: `enabled = true`, `sound = true`, `toast_duration_ms = 3000`, `max_history = 500`. |
| Do Not Disturb mode | `engine` + `chat-shell` | `engine/src/notifications/bus.rs`, `chat-shell/src/status_bar.rs` | "Do not disturb" mode: suppresses toasts and sounds, still records. Toggle via chat or status bar. |

**Deliverable:** Background events trigger notifications. Toast popups, badges, and sound. Full notification history.

---

## Phase 3.1: User-Facing Features (~6-8 hrs, 3 agents in parallel)

**Team:** `levsha-p3-features`

Depends on Phase 3.0 completion (GPU for voice, concurrent sessions for background transcription).

### Track D — Voice Input/Output (`engine` + `chat-shell` + `base-system` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| whisper-server service | `base-system` | `base/overlay/etc/systemd/system/whisper-server.service` | Systemd service running whisper.cpp server. On-demand start. Exposes HTTP endpoint on localhost:8178. |
| Audio capture | `chat-shell` | `chat-shell/src/audio_capture.rs` | PipeWire audio capture. Push-to-talk (configurable key, default: `Ctrl+Space`) or voice activity detection (VAD). |
| STT client | `engine` | `engine/src/voice/stt.rs` | HTTP client for whisper-server. Streams audio chunks, returns transcribed text. Supports multiple languages. |
| TTS engine | `engine` | `engine/src/voice/tts.rs` | Piper TTS integration for offline speech synthesis. Streams audio output to PipeWire. |
| TTS service | `base-system` | `base/overlay/etc/systemd/system/piper-server.service` | Systemd service for Piper TTS. On-demand. HTTP endpoint on localhost:8179. |
| Voice mode toggle | `chat-shell` | `chat-shell/src/voice_mode.rs` | Voice mode UI: microphone indicator, waveform visualization during recording, speaker icon during TTS playback. |
| Voice commands | `engine` | `engine/src/voice/commands.rs` | Shortcut voice commands: "stop", "cancel", "new session", "switch session". Parsed before sending to LLM. |
| Audio device selection | `engine` | `engine/src/voice/devices.rs` | Enumerate PipeWire audio devices. Select input/output device. Persist in config. |
| Voice activity indicator | `chat-shell` | `chat-shell/src/status_bar.rs` | Microphone icon in status bar: gray (off), green (listening), red (recording), blue (speaking). |
| Whisper model download | `engine` | `engine/src/voice/stt.rs` | Download whisper models on first use (base, small, medium). GPU acceleration if available. |
| Config update | `engine` | `engine/src/config.rs` | New `[voice]` section: `enabled = false`, `stt_model = "base"`, `tts_voice = "en_US-lessac"`, `push_to_talk_key = "Ctrl+Space"`, `vad_enabled = false`, `auto_tts = false`. |
| Kickstart packages | `base-system` | `base/kickstart/levsha-os.ks` | Add whisper.cpp, piper, pipewire-devel. Whisper models downloaded separately. |

**Deliverable:** Hands-free voice interaction. Push-to-talk for input, TTS for responses. Works offline with local models.

### Track E — Community Skill Repository (`engine` + `infra` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Registry API client | `engine` | `engine/src/skill_registry.rs` | HTTP client for community registry API. Search, browse, fetch metadata, download skill packages. |
| Skill metadata model | `engine` | `engine/src/skills/metadata.rs` | Extended metadata: author, version, description, tags, ratings, download_count, trust_level, dependencies, compatibility. |
| Trust levels | `engine` | `engine/src/skills/trust.rs` | Trust tiers: `official` (Levsha team), `verified` (reviewed), `community` (unreviewed). Display trust badge in skill info. |
| Skill search with filters | `engine` | `engine/src/skill_registry.rs` | Search by name, tags, author. Filter by trust level, compatibility. Sort by downloads, rating, newest. |
| Skill ratings | `engine` | `engine/src/skill_registry.rs` | Rate installed skills 1-5 stars. Submit rating to registry API. |
| Dependency resolution | `engine` | `engine/src/skills/resolver.rs` | Resolve skill dependencies (other skills + system packages). Install dependency chain. Detect conflicts. |
| Version compatibility | `engine` | `engine/src/skills/resolver.rs` | Skills declare min/max Levsha OS version. Installer checks compatibility before install. |
| Skill update notifications | `engine` | `engine/src/skill_installer.rs` | Periodic check for skill updates (configurable interval). Notification when updates available. |
| Browse categories | `engine` | `engine/src/skill_registry.rs` | Skills organized by category: system, development, media, productivity, networking, utilities. |
| Skill detail view | `chat-shell` | `chat-shell/src/content_panel/skill_detail.rs` | Skill info rendered in split-view: description, screenshots, reviews, version history, install button. |
| Publish flow | `engine` | `engine/src/skill_registry.rs` | `skill_publish` tool: validate manifest, package skill, upload to registry. Requires account. |
| Registry skill update | `skills-dev` | `skills/built-in/skill-manager/` | Update skill-manager with new tools: `skill_browse`, `skill_rate`, `skill_publish`, `skill_check_updates`. |
| Config update | `engine` | `engine/src/config.rs` | Updated `[skills]` section: `registry_url = "https://registry.levsha.dev/api/v1"`, `update_check_interval_hours = 24`, `trust_minimum = "community"`. |

**Deliverable:** Searchable community skill repository with ratings, trust levels, dependency resolution, and publishing.

### Track F — Skill Creation Wizard (`engine` + `skills-dev` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Wizard skill manifest | `skills-dev` | `skills/built-in/skill-wizard/skill.yaml` | Built-in meta-skill for guided skill creation. |
| Conversational flow | `skills-dev` | `skills/built-in/skill-wizard/prompts/skill-wizard.md` | LLM prompt guiding users through skill creation: purpose, tools needed, dependencies, testing. |
| Template generator | `engine` | `engine/src/skills/wizard.rs` | Generate skill scaffold: `skill.yaml`, tool JSON schemas, prompt template, directory structure. |
| Tool schema builder | `engine` | `engine/src/skills/wizard.rs` | Interactive tool definition: name, description, parameters (type, required, description), return type. Validates against Anthropic tool-use schema. |
| Prompt engineering | `engine` | `engine/src/skills/wizard.rs` | Generate effective skill prompts from user description. Include safety guidelines, tool usage instructions, example interactions. |
| Test runner | `engine` | `engine/src/skills/wizard.rs` | Test generated tools by simulating tool calls. Verify tool schemas parse correctly. Dry-run a conversation with the skill active. |
| Publish integration | `engine` | `engine/src/skills/wizard.rs` | After creation and testing, offer to publish to community repository (module 24). |
| Wizard tools | `skills-dev` | `skills/built-in/skill-wizard/tools/` | Tools: `wizard_start`, `wizard_add_tool`, `wizard_set_prompt`, `wizard_preview`, `wizard_test`, `wizard_finalize`, `wizard_publish`. |

**Deliverable:** Users can create new skills entirely through chat conversation. Guided flow from idea to published skill.

---

## Phase 3.2: Integration & Wiring (~3-4 hrs, mostly sequential)

**Team:** `levsha-p3-integration`

Depends on Phase 3.0 + 3.1 completion.

### Wiring Checks

| Check | Components | Verification |
|-------|-----------|--------------|
| GPU detection | 23 ↔ engine ↔ base-system | Boot with GPU passthrough → GPU detected → llama-server uses GPU → tokens/s improved → status bar shows `(GPU)`. |
| GPU fallback | 23 ↔ engine | No GPU available → graceful fallback to CPU inference → no error, just slower. |
| Quantized model | 23 ↔ engine | Load Q4_K_M model → fits in 4GB VRAM → inference works → quality acceptable. |
| Background task | 20 ↔ engine ↔ chat-shell | Start long task in session A → switch to session B → type in B → session A completes in background. |
| Task cancellation | 20 ↔ engine ↔ chat-shell | Start task → Ctrl+C → task cancelled → "Cancelled" message in chat. |
| Session sidebar | 20 ↔ chat-shell | 2+ sessions → sidebar auto-shows → status icons update in real-time → click switches session. |
| Concurrent limit | 20 ↔ engine | Start 4 concurrent cloud requests → 4th queued → 1st completes → 4th starts. |
| Toast notification | 21 ↔ chat-shell | Background task completes → toast slides in → shows session name and result → auto-dismisses. |
| Notification badge | 21 ↔ chat-shell | Background session has activity → badge count appears on session in sidebar → clears on focus. |
| Notification history | 21 ↔ chat-shell | Ctrl+Shift+N → notification panel opens → all notifications listed → mark as read. |
| Sound alerts | 21 ↔ chat-shell | Task completes → chime plays → error → alert sound. DND mode → no sound. |
| Voice input | 22 ↔ engine ↔ chat-shell | Ctrl+Space → microphone active → speak → text appears in input → send → LLM responds. |
| Voice output | 22 ↔ engine ↔ chat-shell | Enable auto-TTS → LLM responds → TTS reads response aloud → speaker icon animates. |
| Voice offline | 22 ↔ engine | No internet → whisper (local) + piper (local) → voice works fully offline. |
| VAD mode | 22 ↔ chat-shell | Enable VAD → system listens continuously → detects speech → auto-records → auto-sends. |
| Skill search | 24 ↔ engine | "Find a skill for PDF editing" → searches community registry → results with ratings and trust badges. |
| Skill install from registry | 24 ↔ engine | "Install the pdf-tools skill" → downloads from registry → resolves dependencies → installs → available. |
| Skill rating | 24 ↔ engine | "Rate pdf-tools 4 stars" → rating submitted to registry. |
| Skill trust display | 24 ↔ chat-shell | Skill info shows trust badge: official (green), verified (blue), community (gray). |
| Dependency resolution | 24 ↔ engine | Install skill with dependency → dependency auto-installed first → both available. |
| Skill creation | 25 ↔ engine | "Help me build a skill for..." → wizard starts → guided through tools, prompt, testing → skill created. |
| Wizard test | 25 ↔ engine | Wizard generates skill → test run → simulated conversation → validates tool schemas. |
| Wizard publish | 25 ↔ engine ↔ 24 | After creation → "Publish to community?" → published to registry. |

### Build & Release

| Task | Agent | Description |
|------|-------|-------------|
| Update kickstart | `base-system` | Add GPU drivers (NVIDIA, AMD), whisper.cpp, piper, pipewire-devel. |
| Update overlay | `base-system` | New systemd services (whisper-server, piper-server), updated config.toml. |
| Update Makefile | `infra` | New build targets for GPU variants, voice model download. |
| ISO size check | `infra` | Verify ISO < 4 GB (increased from 3 GB due to GPU drivers + voice models). VM disk: 30 GB. |
| Rebuild ISO | `infra` | Full rebuild with all Phase 3 components. |
| Boot test | `qa` | Boot → voice input → background task → notification → GPU inference. |

---

## Verification

### After Phase 3.0 — Infrastructure

- [ ] GPU detected automatically on supported hardware
- [ ] llama-server uses GPU when available
- [ ] GPU inference 3x+ faster than CPU for same model
- [ ] Graceful fallback to CPU when no GPU
- [ ] Quantized model loading works (Q4_K_M, Q5_K_M)
- [ ] Status bar shows GPU indicator
- [ ] Background tasks execute while user is in another session
- [ ] Session sidebar shows live status (idle/processing/complete/error)
- [ ] Ctrl+C cancels active task in current session
- [ ] Max concurrent task limit enforced
- [ ] Task results queued and displayed when user returns to session
- [ ] Toast notifications appear for background events
- [ ] Notification badges on sessions with unread activity
- [ ] Notification history accessible via Ctrl+Shift+N
- [ ] Sound alerts play for task complete/error
- [ ] DND mode suppresses toasts and sounds
- [ ] Notifications persist across sessions

### After Phase 3.1 — Features

- [ ] Push-to-talk (Ctrl+Space) captures audio and transcribes
- [ ] Transcribed text appears in input field
- [ ] TTS reads LLM responses aloud (when enabled)
- [ ] Voice works offline (local whisper + piper)
- [ ] Voice activity detection (VAD) mode works
- [ ] Audio device selection works
- [ ] Microphone status indicator in status bar
- [ ] Community registry search returns results with metadata
- [ ] Skill install from registry with dependency resolution
- [ ] Trust levels displayed correctly (official/verified/community)
- [ ] Skill rating submission works
- [ ] Skill update notifications appear
- [ ] Category browsing works
- [ ] Skill creation wizard guides through full process
- [ ] Wizard generates valid skill.yaml + tool JSONs + prompt
- [ ] Wizard test validates generated skill
- [ ] Wizard can publish to community registry

### Performance & Size

- [ ] GPU inference: > 20 tokens/s for 8B model with decent GPU
- [ ] CPU inference: unchanged from Phase 2
- [ ] Whisper transcription: < 2s latency for 5s audio clip (GPU)
- [ ] TTS latency: < 500ms to first audio
- [ ] ISO size < 4 GB (GPU drivers + voice models add ~800 MB)
- [ ] VM disk: 30 GB sufficient for OS + models
- [ ] Idle RAM < 1.5 GB (cloud-only, no voice)
- [ ] Idle RAM < 3 GB (local model + GPU + voice active)
- [ ] All Phase 1 + Phase 2 functionality still works

---

## Risk Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| GPU driver diversity (NVIDIA vs AMD vs Intel) | Complex build, large ISO | Ship modular GPU backends. Only load detected GPU's driver. Vulkan as universal fallback. |
| GPU passthrough in VM is complex | Testing difficulty | Document passthrough setup. Provide CPU-only ISO variant. Test on bare metal when possible. |
| Concurrent sessions race conditions | Corrupted state, crashes | Session-level locks on shared resources (SQLite, config). Extensive concurrency testing. |
| Voice model size (whisper + piper) | Large ISO, slow first use | Download models on first voice use, not at install. Ship only "base" whisper model (74 MB). |
| Voice latency | Frustrating UX | Stream audio to whisper in chunks. Use GPU for whisper when available. Keep TTS pipeline warm. |
| Community registry abuse | Malicious skills, spam | Trust levels + review process. Official skills signed. Community skills show warning. |
| Skill dependency hell | Install failures, conflicts | Strict semver. Solver rejects conflicting dependencies. Rollback on failed install chain. |

---

## Estimated Effort

| Phase | Parallelism | Estimated Time |
|-------|-------------|----------------|
| 3.0 — Infrastructure | 3 agents | 5-7 hours |
| 3.1 — Features | 3 agents | 6-8 hours |
| 3.2 — Integration | Sequential | 3-4 hours |
| **Total** | | **14-19 hours agent time** |

---

## Appendix: Module ↔ File Map

| Module | New Files | Modified Files |
|--------|-----------|----------------|
| 20 — Concurrent Sessions | `engine/src/session/runner.rs`, `engine/src/session/queue.rs`, `chat-shell/src/session_sidebar.rs` | `engine/src/session/mod.rs`, `engine/src/types.rs`, `engine/src/config.rs`, `chat-shell/src/window.rs`, `chat-shell/src/keybindings.rs` |
| 21 — Notification System | `engine/src/notifications/bus.rs`, `engine/src/notifications/types.rs`, `engine/src/notifications/store.rs`, `chat-shell/src/toast.rs`, `chat-shell/src/audio.rs`, `chat-shell/src/notification_panel.rs` | `engine/src/types.rs`, `engine/src/config.rs`, `chat-shell/src/session_sidebar.rs`, `chat-shell/src/session_list.rs`, `chat-shell/src/status_bar.rs` |
| 22 — Voice I/O | `engine/src/voice/stt.rs`, `engine/src/voice/tts.rs`, `engine/src/voice/commands.rs`, `engine/src/voice/devices.rs`, `chat-shell/src/audio_capture.rs`, `chat-shell/src/voice_mode.rs`, `base/overlay/etc/systemd/system/whisper-server.service`, `base/overlay/etc/systemd/system/piper-server.service` | `engine/src/config.rs`, `chat-shell/src/status_bar.rs`, `chat-shell/src/keybindings.rs`, kickstart |
| 23 — GPU Inference | `engine/src/llm/gpu.rs`, `docs/technical/gpu-passthrough.md` | `engine/src/llm/local_server.rs`, `engine/src/llm/model_manager.rs`, `engine/src/config.rs`, `chat-shell/src/status_bar.rs`, `infra/Containerfile`, kickstart |
| 24 — Community Skill Repo | `engine/src/skills/metadata.rs`, `engine/src/skills/trust.rs`, `engine/src/skills/resolver.rs`, `chat-shell/src/content_panel/skill_detail.rs` | `engine/src/skill_registry.rs`, `engine/src/skill_installer.rs`, `engine/src/config.rs`, `skills/built-in/skill-manager/` |
| 25 — Skill Creation Wizard | `engine/src/skills/wizard.rs`, `skills/built-in/skill-wizard/skill.yaml`, `skills/built-in/skill-wizard/prompts/skill-wizard.md`, `skills/built-in/skill-wizard/tools/*.json` | `engine/src/lib.rs` |

**Total new files:** ~45
**Total modified files:** ~25
