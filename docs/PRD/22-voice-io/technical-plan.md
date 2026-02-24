# 22 — Voice Input/Output: Technical Plan

**Module:** Voice Input/Output (L2 + L3)
**Language:** Rust
**Phase:** 3

---

## 1. Crate Structure

```
engine/
  src/
    voice/
      mod.rs              # Voice subsystem entry point, mode management
      stt_client.rs       # HTTP client for whisper-server
      tts_client.rs       # HTTP client for piper-server, streaming audio
      vad.rs              # Voice activity detection (energy + zero-crossing)
      commands.rs         # Voice command parser (regex-based shortcuts)
      audio_capture.rs    # PipeWire audio capture, ring buffer, WAV encoding
      audio_playback.rs   # PipeWire audio playback for TTS
      devices.rs          # PipeWire device enumeration and selection

chat-shell/
  src/
    ui/
      voice/
        mod.rs            # Voice UI coordination
        waveform.rs       # Waveform visualization widget (Cairo DrawingArea)
        tts_indicator.rs  # TTS playback indicator in message bubbles
        device_selector.rs # Audio device selector (split-view panel)
    input/
      push_to_talk.rs     # Push-to-talk key handler

base/
  overlay/
    etc/
      systemd/system/
        whisper-server.service   # systemd on-demand service
        piper-server.service     # systemd on-demand service
```

### New Dependencies

```toml
# Added to engine/Cargo.toml
pipewire-rs = "0.8"           # PipeWire audio capture and playback
hound = "3.5"                 # WAV encoding for whisper input

# Added to chat-shell/Cargo.toml
# (no new crates — Cairo drawing already available via gtk4)
```

---

## 2. whisper-server systemd Service

```ini
# /etc/systemd/system/whisper-server.service
[Unit]
Description=Whisper STT Server (Levsha Voice)
After=network.target
ConditionPathExists=/var/lib/levsha/models/whisper/ggml-base.bin

[Service]
Type=simple
ExecStart=/usr/bin/whisper-server \
    --model /var/lib/levsha/models/whisper/ggml-base.bin \
    --host 127.0.0.1 \
    --port 8178 \
    --threads %N
Restart=on-failure
RestartSec=3
User=levsha
Group=levsha
EnvironmentFile=-/etc/levsha/whisper-server.env

[Install]
WantedBy=multi-user.target
```

The engine starts/stops this service on demand via `systemctl start/stop whisper-server`.

---

## 3. piper-server systemd Service

```ini
# /etc/systemd/system/piper-server.service
[Unit]
Description=Piper TTS Server (Levsha Voice)
After=network.target
ConditionPathExists=/var/lib/levsha/models/piper/en_US-lessac-medium.onnx

[Service]
Type=simple
ExecStart=/usr/bin/piper-server \
    --model /var/lib/levsha/models/piper/en_US-lessac-medium.onnx \
    --host 127.0.0.1 \
    --port 8179
Restart=on-failure
RestartSec=3
User=levsha
Group=levsha
EnvironmentFile=-/etc/levsha/piper-server.env

[Install]
WantedBy=multi-user.target
```

---

## 4. Audio Capture Module

```rust
use pipewire as pw;
use hound;
use std::sync::{Arc, Mutex};

/// Ring buffer for audio capture. Stores PCM samples (16kHz, mono, 16-bit).
pub struct AudioCapture {
    stream: Option<pw::stream::Stream>,
    buffer: Arc<Mutex<Vec<i16>>>,
    is_recording: Arc<Mutex<bool>>,
    sample_rate: u32,
    device_name: Option<String>,
}

impl AudioCapture {
    pub fn new(device_name: Option<&str>) -> Result<Self> {
        Ok(Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::with_capacity(16000 * 30))), // 30s max
            is_recording: Arc::new(Mutex::new(false)),
            sample_rate: 16000,
            device_name: device_name.map(|s| s.to_string()),
        })
    }

    /// Start capturing audio from PipeWire.
    pub fn start_recording(&mut self) -> Result<()> {
        let buffer = self.buffer.clone();
        let is_recording = self.is_recording.clone();
        *is_recording.lock().unwrap() = true;
        buffer.lock().unwrap().clear();

        // Set up PipeWire stream for audio capture
        // Target device selected via device_name or PipeWire default
        let props = pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Audio",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Communication",
        };

        // Stream setup with process callback that writes to ring buffer
        // (PipeWire stream creation omitted for brevity — uses pw::stream::Stream)
        Ok(())
    }

    /// Stop capturing and return the recorded audio as WAV bytes.
    pub fn stop_recording(&mut self) -> Result<Vec<u8>> {
        *self.is_recording.lock().unwrap() = false;

        let samples = self.buffer.lock().unwrap().clone();
        let wav_bytes = self.encode_wav(&samples)?;
        Ok(wav_bytes)
    }

    /// Get current audio level (0.0 - 1.0) for waveform visualization.
    pub fn current_level(&self) -> f32 {
        let buf = self.buffer.lock().unwrap();
        if buf.len() < 160 { return 0.0; }
        let recent = &buf[buf.len() - 160..]; // last 10ms at 16kHz
        let rms = (recent.iter().map(|&s| (s as f64).powi(2)).sum::<f64>()
            / recent.len() as f64).sqrt();
        (rms / 32768.0) as f32
    }

    fn encode_wav(&self, samples: &[i16]) -> Result<Vec<u8>> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()?;
        Ok(cursor.into_inner())
    }
}
```

---

## 5. STT Client

```rust
/// HTTP client for whisper-server.
pub struct SttClient {
    client: reqwest::Client,
    base_url: String,   // http://localhost:8178
}

#[derive(Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: u64,
}

impl SttClient {
    pub fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://localhost:{}", port),
        }
    }

    /// Send audio to whisper-server and receive transcription.
    pub async fn transcribe(
        &self,
        audio_wav: Vec<u8>,
        language: Option<&str>,
    ) -> Result<TranscriptionResult> {
        let mut form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(audio_wav)
                .file_name("audio.wav")
                .mime_str("audio/wav")?);

        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        }

        let response = self.client
            .post(format!("{}/inference", self.base_url))
            .multipart(form)
            .send()
            .await?;

        let result: TranscriptionResult = response.json().await?;
        Ok(result)
    }

    /// Check if whisper-server is healthy.
    pub async fn is_available(&self) -> bool {
        self.client.get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
```

---

## 6. TTS Client

```rust
use tokio::io::AsyncReadExt;

/// HTTP client for piper-server.
pub struct TtsClient {
    client: reqwest::Client,
    base_url: String,   // http://localhost:8179
}

pub struct TtsPlayback {
    /// Handle to cancel playback.
    cancel_tx: tokio::sync::oneshot::Sender<()>,
}

impl TtsClient {
    pub fn new(port: u16) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: format!("http://localhost:{}", port),
        }
    }

    /// Send text to piper-server and stream audio to PipeWire for playback.
    pub async fn speak(
        &self,
        text: &str,
        playback: &AudioPlayback,
    ) -> Result<TtsPlayback> {
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

        let response = self.client
            .post(format!("{}/synthesize", self.base_url))
            .json(&serde_json::json!({
                "text": text,
                "output_type": "raw",
                "sample_rate": 22050
            }))
            .send()
            .await?;

        let mut stream = response.bytes_stream();
        let playback = playback.clone();

        // Spawn task to stream audio to PipeWire
        tokio::spawn(async move {
            tokio::select! {
                _ = cancel_rx => {
                    // Playback cancelled
                    playback.stop();
                }
                _ = async {
                    while let Some(chunk) = stream.try_next().await.ok().flatten() {
                        playback.write(&chunk);
                    }
                    playback.drain();
                } => {}
            }
        });

        Ok(TtsPlayback { cancel_tx })
    }

    /// Check if piper-server is healthy.
    pub async fn is_available(&self) -> bool {
        self.client.get(format!("{}/health", self.base_url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

impl TtsPlayback {
    /// Cancel the current playback.
    pub fn stop(self) {
        let _ = self.cancel_tx.send(());
    }
}
```

---

## 7. Voice Activity Detection

```rust
/// Energy-based voice activity detection with zero-crossing rate.
pub struct VoiceActivityDetector {
    /// Energy threshold (calibrated on enable).
    energy_threshold: f32,
    /// Minimum speech duration to trigger (ms).
    min_speech_ms: u64,
    /// Silence duration to end speech (ms).
    silence_timeout_ms: u64,
    /// Current state.
    state: VadState,
    /// Samples since last state change.
    samples_since_change: usize,
    sample_rate: u32,
}

#[derive(Clone, Copy, PartialEq)]
pub enum VadState {
    Silence,
    MaybeSpeech,
    Speech,
    MaybeSilence,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Sensitivity {
    Low,
    Medium,
    High,
}

impl VoiceActivityDetector {
    pub fn new(sensitivity: Sensitivity) -> Self {
        let (energy_threshold, silence_timeout_ms) = match sensitivity {
            Sensitivity::Low => (0.015, 1200),
            Sensitivity::Medium => (0.008, 800),
            Sensitivity::High => (0.004, 500),
        };

        Self {
            energy_threshold,
            min_speech_ms: 500,
            silence_timeout_ms,
            state: VadState::Silence,
            samples_since_change: 0,
            sample_rate: 16000,
        }
    }

    /// Calibrate threshold from ambient noise.
    /// Records 1 second of ambient audio and sets threshold above it.
    pub fn calibrate(&mut self, ambient_samples: &[i16]) {
        let rms = (ambient_samples.iter()
            .map(|&s| (s as f64).powi(2))
            .sum::<f64>() / ambient_samples.len() as f64)
            .sqrt() / 32768.0;
        self.energy_threshold = (rms as f32 * 3.0).max(0.003);
    }

    /// Feed audio frame (10ms chunks) and return state transitions.
    pub fn process_frame(&mut self, frame: &[i16]) -> Option<VadEvent> {
        let energy = self.frame_energy(frame);
        let zcr = self.zero_crossing_rate(frame);
        let is_speech = energy > self.energy_threshold && zcr < 0.5;

        self.samples_since_change += frame.len();
        let elapsed_ms = (self.samples_since_change * 1000) / self.sample_rate as usize;

        match (self.state, is_speech) {
            (VadState::Silence, true) => {
                self.state = VadState::MaybeSpeech;
                self.samples_since_change = 0;
                None
            }
            (VadState::MaybeSpeech, true) if elapsed_ms >= 200 => {
                self.state = VadState::Speech;
                self.samples_since_change = 0;
                Some(VadEvent::SpeechStart)
            }
            (VadState::MaybeSpeech, false) => {
                self.state = VadState::Silence;
                self.samples_since_change = 0;
                None
            }
            (VadState::Speech, false) => {
                self.state = VadState::MaybeSilence;
                self.samples_since_change = 0;
                None
            }
            (VadState::MaybeSilence, true) => {
                self.state = VadState::Speech;
                self.samples_since_change = 0;
                None
            }
            (VadState::MaybeSilence, false) if elapsed_ms >= self.silence_timeout_ms as usize => {
                self.state = VadState::Silence;
                self.samples_since_change = 0;
                Some(VadEvent::SpeechEnd)
            }
            _ => None,
        }
    }

    fn frame_energy(&self, frame: &[i16]) -> f32 {
        let rms = (frame.iter()
            .map(|&s| (s as f64).powi(2))
            .sum::<f64>() / frame.len() as f64)
            .sqrt();
        (rms / 32768.0) as f32
    }

    fn zero_crossing_rate(&self, frame: &[i16]) -> f32 {
        let crossings = frame.windows(2)
            .filter(|w| (w[0] >= 0) != (w[1] >= 0))
            .count();
        crossings as f32 / frame.len() as f32
    }
}

pub enum VadEvent {
    SpeechStart,
    SpeechEnd,
}
```

---

## 8. Voice Command Parser

```rust
use regex::Regex;

/// Parses transcribed text for known voice commands before LLM dispatch.
pub struct VoiceCommandParser {
    commands: Vec<VoiceCommand>,
}

pub struct VoiceCommand {
    pattern: Regex,
    action: VoiceAction,
}

#[derive(Clone, Debug)]
pub enum VoiceAction {
    StopTts,
    Cancel,
    NewSession,
    ScrollUp,
    ScrollDown,
    ReadAgain,
    VolumeUp,
    VolumeDown,
}

impl VoiceCommandParser {
    pub fn new() -> Self {
        let commands = vec![
            VoiceCommand {
                pattern: Regex::new(r"(?i)^stop$").unwrap(),
                action: VoiceAction::StopTts,
            },
            VoiceCommand {
                pattern: Regex::new(r"(?i)^cancel$").unwrap(),
                action: VoiceAction::Cancel,
            },
            VoiceCommand {
                pattern: Regex::new(r"(?i)^new\s+session$").unwrap(),
                action: VoiceAction::NewSession,
            },
            VoiceCommand {
                pattern: Regex::new(r"(?i)^scroll\s+up$").unwrap(),
                action: VoiceAction::ScrollUp,
            },
            VoiceCommand {
                pattern: Regex::new(r"(?i)^scroll\s+down$").unwrap(),
                action: VoiceAction::ScrollDown,
            },
            VoiceCommand {
                pattern: Regex::new(r"(?i)^read\s+(that\s+)?again$").unwrap(),
                action: VoiceAction::ReadAgain,
            },
            VoiceCommand {
                pattern: Regex::new(r"(?i)^(louder|volume\s+up)$").unwrap(),
                action: VoiceAction::VolumeUp,
            },
            VoiceCommand {
                pattern: Regex::new(r"(?i)^(quieter|volume\s+down)$").unwrap(),
                action: VoiceAction::VolumeDown,
            },
        ];

        Self { commands }
    }

    /// Check transcribed text against known commands.
    /// Returns the action if matched, None if the text should go to LLM.
    pub fn parse(&self, text: &str) -> Option<VoiceAction> {
        let trimmed = text.trim();
        for cmd in &self.commands {
            if cmd.pattern.is_match(trimmed) {
                return Some(cmd.action.clone());
            }
        }
        None
    }
}
```

---

## 9. Waveform Widget

```rust
use gtk4::prelude::*;
use gtk4::cairo;

/// Custom DrawingArea widget that displays a real-time audio waveform.
pub struct WaveformWidget {
    drawing_area: gtk4::DrawingArea,
    levels: Arc<Mutex<VecDeque<f32>>>,  // Recent audio levels (ring buffer)
    max_bars: usize,
}

impl WaveformWidget {
    pub fn new() -> Self {
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_height_request(40);
        drawing_area.set_hexpand(true);

        let levels = Arc::new(Mutex::new(VecDeque::with_capacity(200)));
        let levels_clone = levels.clone();

        drawing_area.set_draw_func(move |_, cr, width, height| {
            let levels = levels_clone.lock().unwrap();
            Self::draw(cr, width, height, &levels);
        });

        Self {
            drawing_area,
            levels,
            max_bars: 200,
        }
    }

    /// Push a new audio level sample (called at ~60fps).
    pub fn push_level(&self, level: f32) {
        let mut levels = self.levels.lock().unwrap();
        if levels.len() >= self.max_bars {
            levels.pop_front();
        }
        levels.push_back(level.clamp(0.0, 1.0));
        self.drawing_area.queue_draw();
    }

    fn draw(cr: &cairo::Context, width: i32, height: i32, levels: &VecDeque<f32>) {
        let w = width as f64;
        let h = height as f64;

        // Background: $bg-secondary (#F5F1EA)
        cr.set_source_rgb(0.96, 0.945, 0.918);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        // Waveform bars: $accent-copper (#C67A52)
        cr.set_source_rgb(0.776, 0.478, 0.322);

        let bar_width = 2.0;
        let bar_gap = 2.0;
        let min_height = 4.0;
        let max_height = h - 8.0; // 4px padding top and bottom
        let total_bar = bar_width + bar_gap;

        let start_x = w - (levels.len() as f64 * total_bar);

        for (i, &level) in levels.iter().enumerate() {
            let x = start_x + (i as f64 * total_bar);
            let bar_h = min_height + (level as f64 * (max_height - min_height));
            let y = (h - bar_h) / 2.0;

            cr.rectangle(x, y, bar_width, bar_h);
            let _ = cr.fill();
        }
    }

    pub fn widget(&self) -> &gtk4::DrawingArea {
        &self.drawing_area
    }

    pub fn clear(&self) {
        self.levels.lock().unwrap().clear();
        self.drawing_area.queue_draw();
    }
}
```

---

## 10. IPC Protocol Extensions

New message types for voice communication between L2 and L3:

```rust
// Engine -> Chat Shell
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VoiceMessage {
    #[serde(rename = "voice_state")]
    State {
        stt_enabled: bool,
        tts_enabled: bool,
        recording: bool,
        tts_playing: bool,
    },
    #[serde(rename = "voice_transcription")]
    Transcription {
        text: String,
        language: Option<String>,
        duration_ms: u64,
    },
    #[serde(rename = "voice_tts_start")]
    TtsStart {
        message_id: String,
    },
    #[serde(rename = "voice_tts_stop")]
    TtsStop {
        message_id: String,
    },
    #[serde(rename = "voice_audio_level")]
    AudioLevel {
        level: f32,
    },
    #[serde(rename = "voice_devices")]
    Devices {
        inputs: Vec<AudioDevice>,
        outputs: Vec<AudioDevice>,
    },
}

// Chat Shell -> Engine
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VoiceAction {
    #[serde(rename = "voice_record_start")]
    RecordStart,
    #[serde(rename = "voice_record_stop")]
    RecordStop,
    #[serde(rename = "voice_tts_cancel")]
    TtsCancel,
    #[serde(rename = "voice_tts_replay")]
    TtsReplay { message_id: String },
    #[serde(rename = "voice_select_device")]
    SelectDevice { device_type: String, device_name: String },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub is_default: bool,
    pub is_active: bool,
}
```

---

## 11. Implementation Stages

**Stage 1 -- whisper-server Integration (2-3 days)**
1. systemd service file for whisper-server.
2. `SttClient` HTTP client.
3. Service lifecycle management (start on enable, stop on disable).
4. Test: record audio, send to server, receive transcription.

**Stage 2 -- Audio Capture (2-3 days)**
1. PipeWire stream setup for audio capture via `pipewire-rs`.
2. Ring buffer for audio samples.
3. WAV encoding with `hound`.
4. Audio level metering for waveform visualization.
5. Test: capture audio, verify WAV output, verify level readings.

**Stage 3 -- Push-to-Talk and Waveform UI (2 days)**
1. Key binding handler for Ctrl+Space (hold/release detection).
2. `WaveformWidget` with Cairo drawing.
3. Slide-up/slide-down animation.
4. Transcribing state display.
5. Transcription result fills input bar.

**Stage 4 -- piper-server Integration (2 days)**
1. systemd service file for piper-server.
2. `TtsClient` HTTP client with streaming audio.
3. PipeWire playback stream.
4. TTS indicator in message bubbles (speaker icon, stop button).
5. TTS cancellation.

**Stage 5 -- Voice Activity Detection (1-2 days)**
1. `VoiceActivityDetector` with energy + zero-crossing.
2. Background noise calibration.
3. Sensitivity configuration.
4. Integration with audio capture (auto-trigger transcription on speech end).

**Stage 6 -- Voice Commands and Settings (1 day)**
1. `VoiceCommandParser` with regex matching.
2. Voice settings card rendering.
3. Chat command handling ("enable voice", "use whisper medium", etc.).
4. Voice mode indicator in status bar.

**Stage 7 -- Audio Device Selection (1 day)**
1. PipeWire device enumeration.
2. Device selector panel in split-view.
3. Device switching.
4. Audio test playback.

---

## 12. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `stt_client.rs` | HTTP request format, response parsing |
| `tts_client.rs` | HTTP request format, streaming handling |
| `vad.rs` | State transitions, threshold calibration, edge cases |
| `commands.rs` | Command matching, case insensitivity, no false positives |
| `audio_capture.rs` | WAV encoding, ring buffer overflow, level calculation |
| `waveform.rs` | Level buffer management, draw dimensions |

### Integration Tests

| Test | Method |
|------|--------|
| STT round-trip | Record audio, send to whisper-server, verify transcription |
| TTS round-trip | Send text to piper-server, verify audio output |
| Push-to-talk flow | Simulate key hold, verify recording starts/stops |
| VAD detection | Feed speech + silence audio, verify SpeechStart/SpeechEnd events |
| Voice command | Transcribe "stop", verify TTS stops without LLM dispatch |
| Device enumeration | List PipeWire devices, verify at least one input and output |
| Voice toggle | Enable/disable voice, verify servers start/stop |
| Model switching | Switch whisper model, verify server restarts with new model |
