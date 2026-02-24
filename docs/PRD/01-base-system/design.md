# 01 — Base System: Design Document

**Module:** L0 (Kernel) + L1 (Base System)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Design Overview

The base system is a minimal Fedora installation stripped to the essentials required to boot, render a Wayland GUI, connect to the network, and run the Chat Shell. Every included package must justify its presence. The design goal is the smallest possible footprint that still delivers polished font rendering and a smooth 60 fps GUI.

---

## 2. Kernel (L0)

**Choice:** Fedora default kernel (`kernel-core` package).

No custom kernel compilation. The Fedora kernel ships with broad hardware support and a proven configuration. For the VM-only MVP, this is the correct trade-off: zero kernel maintenance with full VirtIO and VM guest support.

**Module loading:** The kernel loads only modules required by the virtualized hardware. Unnecessary modules (Bluetooth, USB storage, wireless) are not blacklisted but remain unloaded since the VM does not present the hardware.

**Key kernel features used:**

- VirtIO drivers (disk, network, GPU, console) for QEMU/KVM
- DRM/KMS for Wayland display
- cgroups v2 for systemd resource management
- inotify for filesystem monitoring (SQLite WAL, config changes)

---

## 3. Init System

**Choice:** systemd

systemd manages the entire boot sequence, service lifecycle, and user sessions. It provides:

- Parallel service startup for fast boot
- Socket activation for on-demand services
- Automatic restart policies for critical services
- User session management (`systemd --user`) for the compositor and Chat Shell
- Journal logging for all services

### Boot Target

The system boots to `graphical.target`, which depends on the Wayland compositor being ready. There is no `multi-user.target` fallback.

---

## 4. Wayland Compositor

**Choice:** cage (wlroots-based kiosk compositor)

`cage` is a Wayland kiosk compositor built on wlroots. It runs a single application in full-screen mode with no window decorations, no taskbar, and no window switching. This matches the Levsha OS requirement exactly: one full-screen Chat Shell, nothing else.

**Why cage:**

- Purpose-built for single-application kiosk use cases
- Based on wlroots (proven, actively maintained)
- No window management logic to disable or work around
- Tiny footprint (single binary, minimal dependencies)
- Handles input (keyboard, mouse) and passes it directly to the client

**Why not alternatives:**

| Alternative | Reason rejected |
|-------------|----------------|
| sway | Full tiling WM; excessive for single-app kiosk. Requires disabling features. |
| Mutter | Pulls in GNOME dependencies. Heavy. |
| labwc | More capable than needed; still requires configuration to lock to single-app mode. |
| weston | Reference compositor; less maintained, more complex for kiosk use. |

**Compositor configuration:**

- Launched via systemd user service
- Runs the Chat Shell binary as its sole client
- Virtual terminal: VT7 (standard for graphical sessions)
- Environment variables: `WLR_NO_HARDWARE_CURSORS=1` (for VM compatibility)

---

## 5. Network Stack

**Choice:** NetworkManager + DHCP

### Design

```
NetworkManager (systemd service)
  └── dhclient / internal DHCP
        └── Virtualized ethernet (eth0 / enp0s3)
              └── VirtIO-net or e1000 (VM NIC)
```

NetworkManager is the standard Fedora network manager. It detects the single ethernet interface, runs DHCP, and configures the IP address, gateway, and DNS automatically. No user interaction is required.

### Configuration

- **Connection profile:** A default "Wired connection" profile is auto-created by NetworkManager.
- **DHCP:** Uses NetworkManager's internal DHCP client (no external dhclient dependency).
- **DNS:** Resolved via DHCP-provided nameservers. systemd-resolved is not used; NetworkManager writes `/etc/resolv.conf` directly.
- **Wait for network:** `NetworkManager-wait-online.service` ensures the Intelligence Engine does not start until an IP address is assigned.

### Why not alternatives

| Alternative | Reason rejected |
|-------------|----------------|
| systemd-networkd | Less featureful for the future (Wi-Fi in Phase 2). NetworkManager has better desktop integration. |
| Manual ifconfig/ip | Not persistent, not service-managed, fragile. |
| connman | Less mainstream, less tested on Fedora. |

---

## 6. Audio Stack

**Choice:** PipeWire + WirePlumber

### Design

```
PipeWire (session manager: WirePlumber)
  └── PulseAudio compatibility layer (pipewire-pulse)
  └── ALSA compatibility layer (pipewire-alsa)
        └── Virtualized sound card (Intel HDA / AC97)
```

PipeWire is the modern Linux audio server, shipping as default on Fedora. It replaces PulseAudio and provides low-latency audio with automatic device detection.

**Why PipeWire:**

- Default on Fedora (zero custom configuration)
- Handles audio output for any future skill that produces sound
- PulseAudio-compatible API for application support
- Low resource usage

**MVP usage:** Audio is not actively used by the Chat Shell in the MVP. PipeWire is included to avoid errors from applications that expect an audio server and to be ready for Phase 2 skills (voice input/output).

---

## 7. Font Rendering

Font quality is critical. The Chat Shell's visual polish depends on beautiful text rendering.

### Configuration Stack

```
Application (Chat Shell / GTK4)
  └── Pango (text layout)
        └── FreeType (glyph rasterization)
              └── fontconfig (font selection + rendering hints)
                    └── Font files (.ttf / .otf)
```

### Rendering Settings

Applied via `/etc/fonts/local.conf`:

| Setting | Value | Rationale |
|---------|-------|-----------|
| Antialiasing | enabled | Smooth glyph edges |
| Hinting | slight | Balance between sharpness and fidelity |
| Subpixel rendering | RGB | Standard for LCD/LED panels (including VM virtual displays) |
| LCD filter | default | FreeType's built-in filter for subpixel fringing |
| DPI | auto-detected | Respects VM display DPI; defaults to 96 |

### Font Selection

| Purpose | Font | Package |
|---------|------|---------|
| UI / proportional text | Noto Sans | `google-noto-sans-fonts` |
| Code / monospace | JetBrains Mono or Noto Sans Mono | `jetbrains-mono-fonts` or `google-noto-sans-mono-fonts` |
| Emoji | Noto Color Emoji | `google-noto-emoji-color-fonts` |
| Fallback (CJK, symbols) | Noto Sans CJK | `google-noto-sans-cjk-ttc-fonts` (optional, MVP may omit) |

### HiDPI

Wayland handles HiDPI natively via scale factors. The compositor reads the display's EDID data and applies the appropriate scale. For VMs, the default is 1x. Users with HiDPI host displays who set the VM to a high-resolution virtual display get automatic 2x scaling.

---

## 8. Service Dependency Graph

```
kernel
  └── systemd (PID 1)
        ├── systemd-journald          (logging, early)
        ├── systemd-udevd             (device manager)
        ├── NetworkManager             (networking)
        │     └── NetworkManager-wait-online  (blocks until IP assigned)
        ├── pipewire (user)            (audio)
        │     └── wireplumber (user)   (session manager)
        └── graphical.target
              └── cage (user)          (Wayland compositor)
                    └── levsha-chat    (Chat Shell, L3)
                          └── levsha-engine (Intelligence Engine, L2)
```

### Startup Order

1. **systemd** initializes, starts journald and udevd.
2. **NetworkManager** starts, acquires DHCP lease.
3. **NetworkManager-wait-online** blocks until network is ready.
4. **graphical.target** is reached.
5. **cage** (compositor) starts as a systemd user service on the `levsha` user session.
6. **cage** launches `levsha-chat` as its sole Wayland client.
7. **levsha-chat** starts `levsha-engine` (or engine starts as a separate systemd user service).
8. **pipewire** starts in the user session (non-blocking, parallel with cage).

### Restart Policies

| Service | Restart | Delay |
|---------|---------|-------|
| cage | on-failure | 1s |
| levsha-chat | always | 1s |
| levsha-engine | on-failure | 2s |
| NetworkManager | on-failure | 5s |
| pipewire | on-failure | 1s |

---

## 9. User Account

A single user account `levsha` is created during image build:

- **UID:** 1000
- **Groups:** `wheel` (sudo access), `audio`, `video`, `input`
- **Shell:** `/usr/bin/bash` (not user-accessible in normal operation, but available for emergency maintenance)
- **Auto-login:** Configured via systemd `getty` override or direct compositor launch without a display manager
- **Home directory:** `/home/levsha` (stores SQLite database, engine config, conversation history)
- **Password:** Disabled (no login prompt exists)

---

## 10. Filesystem Layout

```
/
├── boot/                    # Kernel, initramfs, bootloader
├── etc/
│   ├── fonts/local.conf     # Font rendering configuration
│   ├── NetworkManager/      # Network configuration
│   ├── systemd/             # System-level service overrides
│   └── levsha/
│       └── config.toml      # System config (API key, model, paths)
├── home/levsha/
│   ├── .local/share/levsha/
│   │   └── history.db       # SQLite conversation database
│   └── .config/
│       └── systemd/user/    # User-level service units (cage, chat, engine, pipewire)
├── usr/
│   ├── bin/
│   │   ├── levsha-chat      # Chat Shell binary
│   │   └── levsha-engine    # Intelligence Engine binary
│   └── share/
│       └── levsha/
│           └── skills/      # Built-in skill definitions
└── var/log/journal/         # Persistent systemd journal logs
```

---

## 11. Design Decisions Log

| Decision | Choice | Alternatives Considered | Rationale |
|----------|--------|------------------------|-----------|
| Base distro | Fedora | Arch, Ubuntu, Alpine | Best GTK4/Wayland/font support out of the box |
| Compositor | cage | sway, labwc, weston, Mutter | Purpose-built kiosk compositor; minimal footprint |
| Network manager | NetworkManager | systemd-networkd, connman | Fedora default; best path to Wi-Fi support in Phase 2 |
| Audio server | PipeWire | PulseAudio, ALSA-only | Fedora default; modern, low-latency, future-ready |
| Font hinting | slight | full, none | Best balance of sharpness and glyph fidelity |
| Init system | systemd | — | Only viable choice for Fedora; excellent service management |
| Display manager | None | GDM, SDDM | Auto-login with no DM avoids unnecessary complexity and boot time |
