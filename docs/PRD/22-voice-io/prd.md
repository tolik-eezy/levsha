# 22 — Voice Input/Output: Product Requirements

**Module:** Voice Input/Output (L2 + L3)
**Phase:** 3
**Status:** Draft

---

## 1. Overview

Phase 3 adds hands-free interaction via speech-to-text and text-to-speech. Users can speak to Levsha using a push-to-talk key, and Levsha can read responses aloud. The STT engine is `whisper.cpp` (running as a local server), and the TTS engine is `Piper` (also a local server). Both integrate with PipeWire for audio capture and playback.

Voice is an optional mode — the chat remains the primary interface. Voice augments it for accessibility and convenience.

---

## 2. Functional Requirements

### 2.1 Speech-to-Text (VI-01)

| Field | Value |
|-------|-------|
| **ID** | VI-01 |
| **Priority** | P0 |
| **Requirement** | The system transcribes spoken input to text using whisper.cpp. |

**Details:**

- **Runtime:** `whisper.cpp` compiled as a server (`whisper-server`) providing an HTTP API.
- **Bundled model:** Whisper base model (74 MB) is included in the ISO or downloaded on first use.
- **Model storage:** `/var/lib/levsha/models/whisper/`.
- **Server management:** `whisper-server` runs as a systemd service, started on demand when voice mode is enabled.
- **Transport:** HTTP over localhost (port 8178). Accepts audio POST, returns JSON transcription.
- **Audio format:** WAV (16kHz, mono, 16-bit) captured from PipeWire.

**Acceptance Criteria:**

- [ ] `whisper-server` starts and serves the whisper base model.
- [ ] Engine can send audio data and receive text transcriptions.
- [ ] Transcription latency is under 3 seconds for a 10-second utterance on CPU.
- [ ] Whisper model files are stored in `/var/lib/levsha/models/whisper/`.

### 2.2 Push-to-Talk (VI-02)

| Field | Value |
|-------|-------|
| **ID** | VI-02 |
| **Priority** | P0 |
| **Requirement** | A configurable keyboard shortcut activates audio recording for speech input. |

**Details:**

- Default key: `Ctrl+Space`. Configurable in `/etc/levsha/config.toml`.
- **Hold mode:** Hold the key to record, release to submit. Audio is captured while key is held.
- **Toggle mode:** Press once to start recording, press again to stop. Configurable in settings.
- While recording, a waveform visualization appears above the input bar.
- On release/stop, audio is sent to `whisper-server` for transcription.
- Transcribed text fills the input bar. User can edit before sending, or it auto-sends (configurable).
- If voice mode is disabled, the key does nothing.

**Acceptance Criteria:**

- [ ] Ctrl+Space activates recording.
- [ ] Audio is captured from PipeWire while key is held.
- [ ] Release triggers transcription.
- [ ] Transcribed text appears in the input bar.
- [ ] Push-to-talk key is configurable.

### 2.3 Text-to-Speech (VI-03)

| Field | Value |
|-------|-------|
| **ID** | VI-03 |
| **Priority** | P0 |
| **Requirement** | Piper TTS reads LLM responses aloud through PipeWire. |

**Details:**

- **Runtime:** `piper-server` compiled as a server providing an HTTP API.
- **Bundled voice:** One English voice model (~15 MB) included in the ISO.
- **Model storage:** `/var/lib/levsha/models/piper/`.
- **Server management:** `piper-server` runs as a systemd service, started on demand.
- **Transport:** HTTP over localhost (port 8179). Accepts text POST, returns streaming audio.
- **Playback:** Audio streamed to PipeWire for output.
- When TTS is active and an LLM response arrives, the response text is sent to Piper.
- TTS can be interrupted by the user (clicking the speaker icon, pressing Escape, or speaking a new input).

**Acceptance Criteria:**

- [ ] `piper-server` starts and serves a voice model.
- [ ] LLM responses are read aloud when TTS is enabled.
- [ ] Audio plays through PipeWire.
- [ ] User can interrupt TTS playback.

### 2.4 Voice Mode Toggle (VI-04)

| Field | Value |
|-------|-------|
| **ID** | VI-04 |
| **Priority** | P0 |
| **Requirement** | Voice features can be enabled or disabled via chat commands or configuration. |

**Details:**

- "Enable voice" starts whisper-server and piper-server, activates push-to-talk and TTS.
- "Disable voice" stops both servers, deactivates all voice features.
- "Enable speech to text" / "Enable text to speech" — individual toggles.
- Voice mode state persists across restarts.
- Status bar shows voice mode indicator (mic icon with color state).

**Acceptance Criteria:**

- [ ] "Enable voice" starts both voice services.
- [ ] "Disable voice" stops both voice services.
- [ ] STT and TTS can be toggled independently.
- [ ] Voice mode state persists across restarts.
- [ ] Status bar shows voice mode indicator.

### 2.5 Voice Activity Detection (VI-05)

| Field | Value |
|-------|-------|
| **ID** | VI-05 |
| **Priority** | P1 |
| **Requirement** | The system auto-detects speech start and end without requiring push-to-talk. |

**Details:**

- VAD mode is an alternative to push-to-talk (user chooses one).
- Energy-based detection: audio level exceeds threshold for 200ms to start, drops below for 800ms to stop.
- Adjustable sensitivity: "set voice sensitivity to high/medium/low".
- When VAD detects speech end, audio is sent to whisper-server automatically.
- False-positive mitigation: minimum utterance length (500ms), background noise calibration on enable.

**Acceptance Criteria:**

- [ ] VAD detects speech start and end.
- [ ] Automatic transcription on speech end.
- [ ] Sensitivity is adjustable.
- [ ] Background noise calibration runs on enable.

### 2.6 Audio Device Selection (VI-06)

| Field | Value |
|-------|-------|
| **ID** | VI-06 |
| **Priority** | P1 |
| **Requirement** | Users can select input and output audio devices from available PipeWire devices. |

**Details:**

- "Select audio device" or "change microphone" opens an audio device selector.
- Lists available PipeWire input devices (microphones) and output devices (speakers/headphones).
- Device selector rendered in split-view panel.
- Selected devices are saved to config and persist across restarts.
- Default: system default PipeWire devices.

**Acceptance Criteria:**

- [ ] Available PipeWire devices are listed.
- [ ] User can select input and output devices.
- [ ] Device selection persists.
- [ ] Default is system PipeWire default.

### 2.7 Voice Commands (VI-07)

| Field | Value |
|-------|-------|
| **ID** | VI-07 |
| **Priority** | P1 |
| **Requirement** | Short voice commands are parsed locally before LLM dispatch for immediate action. |

**Details:**

Recognized voice commands (case-insensitive, fuzzy-matched):

| Command | Action |
|---------|--------|
| "Stop" | Stop TTS playback |
| "Cancel" | Cancel current operation |
| "New session" | Create a new chat session |
| "Scroll up/down" | Scroll the chat view |
| "Read that again" | Re-read the last response |
| "Louder" / "Quieter" | Adjust TTS volume |

- Commands are matched against the transcription text before sending to the LLM.
- If matched, the action is executed immediately without LLM round-trip.
- If not matched, the text is sent to the LLM as a normal message.

**Acceptance Criteria:**

- [ ] Voice commands are recognized and executed without LLM dispatch.
- [ ] Unrecognized speech is sent to the LLM normally.
- [ ] Command list is extensible.

### 2.8 Whisper Model Selection (VI-08)

| Field | Value |
|-------|-------|
| **ID** | VI-08 |
| **Priority** | P1 |
| **Requirement** | Users can select between different whisper model sizes for accuracy vs. speed tradeoff. |

**Details:**

| Model | Size | Speed (CPU) | Accuracy |
|-------|------|-------------|----------|
| base | 74 MB | Fast (~1x real-time) | Good for clear speech |
| small | 244 MB | Medium (~2x real-time) | Better for accented/noisy |
| medium | 769 MB | Slow (~5x real-time) | Best accuracy |

- "Use whisper base/small/medium model" switches the model.
- Model download if not already present.
- whisper-server restarts with the new model.
- Current model shown in voice settings.

**Acceptance Criteria:**

- [ ] Users can switch whisper models via chat.
- [ ] Models are downloaded on demand.
- [ ] whisper-server restarts with the selected model.

### 2.9 Multi-Language Support (VI-09)

| Field | Value |
|-------|-------|
| **ID** | VI-09 |
| **Priority** | P2 |
| **Requirement** | Users can select their preferred language for speech recognition. |

**Details:**

- Whisper supports 99 languages natively.
- "Set voice language to [language]" configures the preferred language.
- When language is set, whisper-server receives a language hint for better accuracy.
- Default: auto-detect (whisper determines language from audio).
- Language setting persists.

**Acceptance Criteria:**

- [ ] Language can be set via chat.
- [ ] Language hint is passed to whisper-server.
- [ ] Auto-detect is the default.

---

## 3. Architecture

```
┌──────────────────────────────────────────────────────────┐
│                    Chat Shell (L3)                         │
│                                                           │
│  Audio capture (PipeWire)  ──►  whisper-server  ──►  Text │
│  Waveform visualization                                   │
│  TTS playback (PipeWire)   ◄──  piper-server   ◄──  Text │
│  Voice mode indicator                                     │
│                                                           │
└────────────────────────────────┬──────────────────────────┘
                                 │ IPC
┌────────────────────────────────▼──────────────────────────┐
│                Intelligence Engine (L2)                     │
│                                                            │
│  ┌─────────────────┐   ┌──────────────┐                   │
│  │ Voice Command   │   │ STT Client   │                   │
│  │ Parser          │   │ (HTTP)       │                   │
│  └────────┬────────┘   └──────┬───────┘                   │
│           │ match?             │ transcribed text           │
│           ▼                    ▼                            │
│  Execute immediately    Normal LLM dispatch                │
│                                                            │
│  ┌──────────────┐                                         │
│  │ TTS Client   │ ──► piper-server ──► PipeWire           │
│  │ (HTTP)       │                                         │
│  └──────────────┘                                         │
└────────────────────────────────────────────────────────────┘
           │                      │
           ▼                      ▼
     whisper-server          piper-server
     (localhost:8178)        (localhost:8179)
```

---

## 4. New System Requirements

| Package | Purpose | Size |
|---------|---------|------|
| `whisper.cpp` (compiled) | Speech-to-text server | ~5 MB |
| Whisper base model | Default STT model | 74 MB |
| `piper` (compiled) | Text-to-speech server | ~3 MB |
| Piper English voice | Default TTS voice | ~15 MB |
| `pipewire-rs` (crate) | PipeWire audio integration | Build dep |

**Impact:** ISO size increases by ~100 MB (whisper base model + piper voice). VM RAM requirement: no increase for base model; medium model needs +512 MB during inference.

---

## 5. Configuration

New config entries in `/etc/levsha/config.toml`:

```toml
[voice]
enabled = false                      # Master voice toggle
stt_enabled = true                   # Speech-to-text (when voice enabled)
tts_enabled = true                   # Text-to-speech (when voice enabled)

[voice.stt]
server_binary = "/usr/bin/whisper-server"
model_path = "/var/lib/levsha/models/whisper/ggml-base.bin"
models_dir = "/var/lib/levsha/models/whisper"
port = 8178
model_size = "base"                  # "base", "small", "medium"
language = "auto"                    # ISO 639-1 code or "auto"

[voice.tts]
server_binary = "/usr/bin/piper-server"
model_path = "/var/lib/levsha/models/piper/en_US-lessac-medium.onnx"
models_dir = "/var/lib/levsha/models/piper"
port = 8179
voice = "en_US-lessac-medium"
speed = 1.0                          # Playback speed multiplier

[voice.input]
mode = "push_to_talk"                # "push_to_talk" or "vad"
push_to_talk_key = "Ctrl+Space"
auto_send = false                    # Auto-send transcription without editing
vad_sensitivity = "medium"           # "low", "medium", "high"

[voice.output]
device_input = ""                    # PipeWire device name (empty = default)
device_output = ""                   # PipeWire device name (empty = default)
volume = 1.0                         # TTS volume (0.0 - 1.0)
```

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Waveform widget, voice mode indicator, push-to-talk key handling, TTS playback controls. |
| Intelligence Engine (03) | Modified | STT/TTS client modules, voice command parser, voice mode management. |
| Base System (01) | Modified | whisper-server and piper-server systemd services, model storage, PipeWire configuration. |
| Split-View (12) | Consumer | Audio device selector rendered in split-view panel. |
| GPU Inference (23) | Optional | GPU acceleration for whisper.cpp inference. |

---

## 7. Out of Scope

- Wake word detection ("Hey Levsha").
- Continuous dictation mode (always-on transcription).
- Voice cloning or custom voice training.
- Real-time translation (transcribe one language, respond in another).
- Phone/telephony integration.
- Voice biometrics or speaker identification.
- Lip-sync or avatar animation.
