# 22 — Voice Input/Output: Design Specification

**Module:** Voice Input/Output (L2 + L3)
**Phase:** 3

---

## 1. Architecture Overview

```
┌───────────────────────────────────────────────────────────┐
│                    Chat Shell (L3)                          │
│                                                            │
│  ┌────────────────────────────────────────────────────┐   │
│  │  Status bar: voice mode indicator (mic icon)        │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  ┌────────────────────────────────────────────────────┐   │
│  │  Waveform bar (40px, above input, during recording) │   │
│  ├────────────────────────────────────────────────────┤   │
│  │  > _  [🎤]                                          │   │
│  └────────────────────────────────────────────────────┘   │
│                                                            │
│  Audio capture ──► PipeWire ──► whisper-server             │
│  TTS playback  ◄── PipeWire ◄── piper-server              │
│                                                            │
└───────────────────────────────────┬────────────────────────┘
                                    │ IPC
┌───────────────────────────────────▼────────────────────────┐
│                Intelligence Engine (L2)                      │
│                                                             │
│  STT Client ──► Voice Command Parser ──► LLM Dispatch      │
│  TTS Client ◄── Response text                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Voice Mode Indicator

The status bar displays a microphone icon whose color reflects the current voice state.

### Indicator States

| State | Icon | Color | Label |
|-------|------|-------|-------|
| Voice disabled | Lucide `mic-off` | `$text-tertiary` (#8C7E6E) | (none) |
| Idle (voice enabled) | Lucide `mic` | `$text-secondary` (#6B5D4F) | (none) |
| Listening (VAD active) | Lucide `mic` | `$accent-green` (#62B37B) | (none) |
| Recording (PTT held) | Lucide `mic` | `$accent-copper` (#C67A52) | `REC` |
| Transcribing | Lucide `mic` | `$accent-gold` (#D4A853) | `...` |
| TTS playing | Lucide `volume-2` | `$accent-copper` (#C67A52) | (none) |

### Status Bar Layout

```
▸ auto · 🎤 · connected · 14:32
          ↑ voice indicator
```

When recording:

```
▸ auto · 🎤 REC · connected · 14:32
```

### Indicator Styling

| Element | Style |
|---------|-------|
| Icon size | 14px |
| Label font | IBM Plex Sans 11px, weight 600 |
| REC label | `$accent-copper` (#C67A52) |
| Transcribing dots | `$accent-gold` (#D4A853), animated pulse (1s cycle) |

---

## 3. Push-to-Talk UX Flow

### Recording Flow

```
1. User holds Ctrl+Space
   ↓
2. Waveform bar appears above input (slide up, 150ms ease-out)
   ↓
3. Audio captured from PipeWire, waveform updates in real-time
   ↓
4. User releases Ctrl+Space
   ↓
5. Waveform freezes, "Transcribing..." label appears
   ↓
6. whisper-server returns text
   ↓
7. Waveform slides away (150ms ease-in), text fills input bar
   ↓
8. User edits or presses Enter to send (or auto-sends if configured)
```

### Waveform Bar

```
┌──────────────────────────────────────────────────────────┐
│  ║ ▌║▌▐ ▌▐▌▐║▐║▌▌▐▌║▐║ ▌▐║▐▌▐▌║▐ ▐▌║▌▐ ▌║▐▌▐║▌ ▐▌▐ ▌│ 0:03
├──────────────────────────────────────────────────────────┤
│  > _                                                     │
└──────────────────────────────────────────────────────────┘
```

### Waveform Styling

| Element | Style |
|---------|-------|
| Bar height | 40px |
| Bar background | `$bg-secondary` (#F5F1EA) |
| Waveform bars | `$accent-copper` (#C67A52), 2px wide, 2px gap |
| Waveform bar height | Proportional to audio level (4px min, 32px max) |
| Border top | 1px `$border-primary` (#EBE6DC) |
| Border bottom | 1px `$border-primary` |
| Duration label | IBM Plex Mono 12px, `$text-tertiary` (#8C7E6E), right-aligned |
| Padding | 12px horizontal |

### Transcribing State

```
┌──────────────────────────────────────────────────────────┐
│  ║ ▌║▌▐ ▌▐▌▐║▐║▌▌▐▌║▐║ ▌▐║▐▌▐▌║▐ ▐▌║▌▐   Transcribing…│
├──────────────────────────────────────────────────────────┤
│  > _                                                     │
└──────────────────────────────────────────────────────────┘
```

### Transcribing Label Styling

| Element | Style |
|---------|-------|
| Font | IBM Plex Sans 12px, weight 500 |
| Color | `$accent-gold` (#D4A853) |
| Animation | Ellipsis pulse (opacity 1.0 → 0.3, 1s cycle) |

---

## 4. TTS Playback Indicator

When the system reads a response aloud, the message bubble shows a playback indicator.

### Active Playback

```
┌─ Levsha ──────────────────────────────────────────┐
│                                                     │
│  Here are the packages you have installed:          │
│  vim, git, curl, and 42 others.                     │
│                                                     │
│                                   🔊 Playing...  ⏹ │
└─────────────────────────────────────────────────────┘
```

### Playback Controls

| Element | Style |
|---------|-------|
| Speaker icon | Lucide `volume-2`, 16px, `$accent-copper` (#C67A52) |
| Speaker animation | Icon pulses (scale 1.0 → 1.15, 600ms ease-in-out, looping) |
| "Playing..." label | IBM Plex Sans 12px, `$text-secondary` (#6B5D4F) |
| Stop button | Lucide `square`, 16px, `$text-tertiary`, hover: `$accent-copper` |
| Position | Bottom-right of message bubble, 8px padding |

### Completed/Stopped

```
┌─ Levsha ──────────────────────────────────────────┐
│                                                     │
│  Here are the packages you have installed:          │
│  vim, git, curl, and 42 others.                     │
│                                                     │
│                                          🔊 ▶      │
└─────────────────────────────────────────────────────┘
```

| Element | Style |
|---------|-------|
| Speaker icon (idle) | Lucide `volume-2`, 16px, `$text-tertiary` (#8C7E6E) |
| Play button | Lucide `play`, 16px, `$text-tertiary`, hover: `$accent-copper` |
| Tooltip | "Read aloud" |

---

## 5. Voice Settings via Chat

Voice settings are configured conversationally.

### Enable/Disable

```
User: enable voice

Levsha: Voice mode enabled.

        • Speech-to-text: whisper (base model)
        • Text-to-speech: Piper (en_US-lessac)
        • Push-to-talk: Ctrl+Space

        Say "disable voice" to turn off.
```

### Change Voice Model

```
User: use whisper medium model

Levsha: Downloading whisper medium model (769 MB)...

        ████████████░░░░░░░░ 62%
        477 MB / 769 MB — 8.3 MB/s — ~35s remaining

        Whisper will restart with the medium model
        when the download completes.
```

### Voice Settings Card

```
User: voice settings

Levsha: Here are your voice settings:

  ┌─ Voice Settings ─────────────────────────────────┐
  │                                                    │
  │  Speech-to-Text                                    │
  │  ─────────────────────────────────────────────     │
  │  Status:    Enabled                                │
  │  Model:     whisper base (74 MB)                   │
  │  Language:  Auto-detect                            │
  │  Input:     push-to-talk (Ctrl+Space)              │
  │  Device:    Built-in Microphone                    │
  │                                                    │
  │  Text-to-Speech                                    │
  │  ─────────────────────────────────────────────     │
  │  Status:    Enabled                                │
  │  Voice:     en_US-lessac-medium                    │
  │  Speed:     1.0x                                   │
  │  Volume:    100%                                   │
  │  Device:    Built-in Speakers                      │
  │                                                    │
  └────────────────────────────────────────────────────┘
```

### Voice Settings Card Styling

| Element | Style |
|---------|-------|
| Card background | `$bg-surface` (#FDFBF7) |
| Card border | 1px `$border-primary` (#EBE6DC) |
| Card cornerRadius | 8 |
| Section header | IBM Plex Sans 14px, weight 600, `$text-primary` (#3A3228) |
| Section divider | 1px `$border-primary` |
| Label | IBM Plex Sans 13px, `$text-tertiary` (#8C7E6E) |
| Value | IBM Plex Sans 13px, weight 500, `$text-primary` |
| Enabled status | `$accent-green` (#62B37B) |
| Disabled status | `$text-tertiary` |

---

## 6. Audio Device Selector

Rendered in the split-view panel when the user requests "select audio device" or "change microphone".

### Device Selector Panel

```
┌─ Audio Devices ──────────────────────────────── [×] ┐
│                                                       │
│  Input Devices (Microphone)                           │
│  ──────────────────────────────────────────────       │
│  ● Built-in Microphone                    Active      │
│  ○ USB Headset Microphone                             │
│  ○ Bluetooth Earbuds                                  │
│                                                       │
│  Output Devices (Speaker)                             │
│  ──────────────────────────────────────────────       │
│  ● Built-in Speakers                      Active      │
│  ○ USB Headset                                        │
│  ○ HDMI Audio                                         │
│  ○ Bluetooth Earbuds                                  │
│                                                       │
│  ┌──────────────────────────────────────────────┐    │
│  │  🔊 Test: The quick brown fox jumps over...   │    │
│  └──────────────────────────────────────────────┘    │
│                                                       │
└───────────────────────────────────────────────────────┘
```

### Device Selector Styling

| Element | Style |
|---------|-------|
| Active device icon (●) | `$accent-green` (#62B37B) |
| Inactive device icon (○) | `$text-tertiary` (#8C7E6E) |
| Device name | IBM Plex Sans 14px, `$text-primary` (#3A3228) |
| Active label | IBM Plex Sans 12px, weight 500, `$accent-green` |
| Section header | IBM Plex Sans 13px, weight 600, `$text-secondary` (#6B5D4F) |
| Test button background | `$bg-secondary` (#F5F1EA) |
| Test button border | 1px `$border-primary` (#EBE6DC) |
| Test button cornerRadius | 6 |
| Device row height | 36px |
| Device row hover | `$bg-secondary` |
| Device row click | selects device, updates ● indicator |

---

## 7. Animation Specifications

| Animation | Duration | Easing | Trigger |
|-----------|----------|--------|---------|
| Waveform bar slide up | 150ms | ease-out | Push-to-talk key pressed |
| Waveform bar slide down | 150ms | ease-in | Transcription complete |
| Waveform bar levels | 16ms (60fps) | linear | Real-time audio input |
| Transcribing ellipsis pulse | 1000ms | ease-in-out | Transcription in progress |
| TTS speaker icon pulse | 600ms | ease-in-out | TTS playback active |
| Voice indicator state change | 200ms | ease-out | Voice state transition |
| Recording dot blink | 1000ms | step | Recording active |

---

## 8. Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl+Space | Push-to-talk (hold to record, release to transcribe) |
| Escape | Stop TTS playback / Cancel recording |
| Ctrl+Shift+V | Toggle voice mode on/off |
| Ctrl+Shift+M | Mute/unmute microphone |

---

## 9. Assets Used

| Asset | Usage |
|-------|-------|
| Lucide `mic` | Voice enabled indicator |
| Lucide `mic-off` | Voice disabled indicator |
| Lucide `volume-2` | TTS active / speaker icon |
| Lucide `volume-x` | TTS muted |
| Lucide `play` | Replay TTS |
| Lucide `square` | Stop TTS playback |
| Lucide `settings` | Voice settings |
| Lucide `radio` | Audio device selector |

---

## 10. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Waveform widget, voice indicator, TTS controls, push-to-talk key binding. |
| Intelligence Engine (03) | Modified | STT/TTS client, voice command parser, voice mode state. |
| Split-View (12) | Consumer | Audio device selector rendered in content panel. |
| Base System (01) | Modified | PipeWire audio routing configuration. |
