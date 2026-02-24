# 01 — Base System: Technical Plan

**Module:** L0 (Kernel) + L1 (Base System)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Implementation Strategy

Start from a Fedora minimal install, strip unnecessary packages, add the required ones, configure systemd services, and overlay custom configuration files. The result is a rootfs that boots directly into the Wayland compositor running the Chat Shell.

---

## 2. Step-by-Step Implementation

### Step 1: Fedora Minimal Base

Start with `@core` package group from Fedora (installed via kickstart). This provides the kernel, systemd, basic filesystem utilities, and the RPM/dnf package manager.

```
# In kickstart:
%packages
@core
-plymouth          # Remove default boot splash (we use our own)
-firewalld         # Not needed in MVP VM
-sssd*             # No directory services
-abrt*             # No crash reporting
-kdump*            # No kernel dump
%end
```

### Step 2: Add Required Packages

Install the packages that form the base system. Organized by subsystem:

#### Kernel and Boot

| Package | Purpose |
|---------|---------|
| `kernel-core` | Linux kernel (included in @core) |
| `systemd` | Init system (included in @core) |
| `grub2-efi-x64` | UEFI bootloader |
| `shim-x64` | Secure boot shim |
| `dracut` | Initramfs generator |

#### Wayland Compositor

| Package | Purpose |
|---------|---------|
| `cage` | wlroots-based kiosk compositor |
| `mesa-dri-drivers` | OpenGL drivers (software + VirtIO GPU) |
| `libwayland-client` | Wayland client library (Chat Shell dependency) |
| `xorg-x11-server-Xwayland` | Only if any dependency requires X11 compat (prefer to omit) |

#### Networking

| Package | Purpose |
|---------|---------|
| `NetworkManager` | Network management daemon |
| `dhcp-client` | DHCP (bundled with NM, but explicit) |

#### Audio

| Package | Purpose |
|---------|---------|
| `pipewire` | Audio server |
| `pipewire-pulseaudio` | PulseAudio compatibility |
| `wireplumber` | PipeWire session manager |

#### Fonts

| Package | Purpose |
|---------|---------|
| `google-noto-sans-fonts` | UI proportional font |
| `google-noto-sans-mono-fonts` | Fallback monospace font |
| `jetbrains-mono-fonts` | Primary monospace font (code blocks) |
| `google-noto-emoji-color-fonts` | Emoji rendering |
| `fontconfig` | Font configuration framework |
| `freetype` | Font rasterization engine |

#### Utilities

| Package | Purpose |
|---------|---------|
| `dnf` | Package manager (included in @core) |
| `sqlite` | Database engine for conversation persistence |
| `curl` | HTTP client (engine API calls; also useful for health checks) |
| `bash` | Shell (emergency maintenance only) |
| `coreutils` | Basic file utilities (required by skills) |
| `procps-ng` | Process utilities: ps, top, free (system info skill) |
| `iproute` | Network utilities: ip, ss (system info skill) |
| `util-linux` | System utilities: lsblk, lscpu, etc. |
| `sudo` | Privilege escalation for package management |

#### VM Guest Support

| Package | Purpose |
|---------|---------|
| `qemu-guest-agent` | QEMU guest integration |
| `spice-vdagent` | Display resize, clipboard (QEMU/SPICE) |
| `virtualbox-guest-additions` | VirtualBox guest support (if targeting VBox) |

### Step 3: Strip Unnecessary Services

Disable or mask services not needed in the MVP:

```bash
# Disable unnecessary services
systemctl disable bluetooth.service
systemctl disable cups.service
systemctl disable avahi-daemon.service
systemctl disable ModemManager.service
systemctl mask systemd-resolved.service    # NM handles DNS directly
systemctl mask plymouth-quit-wait.service  # Custom boot splash
```

### Step 4: Configure Auto-Login

No display manager. The compositor launches directly from a systemd user service, triggered by auto-login on the `levsha` user.

**`/etc/systemd/system/getty@tty7.service.d/autologin.conf`:**

```ini
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin levsha --noclear %I $TERM
```

Alternatively, a dedicated systemd service bypasses getty entirely:

**`/etc/systemd/system/levsha-compositor.service`:**

```ini
[Unit]
Description=Levsha OS Compositor (cage)
After=network-online.target systemd-user-sessions.service
Wants=network-online.target

[Service]
Type=simple
User=levsha
PAMName=login
Environment=XDG_SESSION_TYPE=wayland
Environment=WLR_NO_HARDWARE_CURSORS=1
Environment=WLR_RENDERER=pixman
ExecStart=/usr/bin/cage -- /usr/bin/levsha-chat
Restart=on-failure
RestartSec=1

[Install]
WantedBy=graphical.target
```

### Step 5: Configure User Session Services

PipeWire and WirePlumber run as systemd user services (Fedora default behavior):

```bash
# These are typically enabled by default in Fedora user sessions
systemctl --user --global enable pipewire.socket
systemctl --user --global enable pipewire-pulse.socket
systemctl --user --global enable wireplumber.service
```

### Step 6: Configure Font Rendering

**`/etc/fonts/local.conf`:**

```xml
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig>
  <!-- Enable antialiasing -->
  <match target="font">
    <edit name="antialias" mode="assign"><bool>true</bool></edit>
  </match>

  <!-- Slight hinting for balanced rendering -->
  <match target="font">
    <edit name="hinting" mode="assign"><bool>true</bool></edit>
    <edit name="hintstyle" mode="assign"><const>hintslight</const></edit>
  </match>

  <!-- Subpixel rendering (RGB for standard LCD) -->
  <match target="font">
    <edit name="rgba" mode="assign"><const>rgb</const></edit>
  </match>

  <!-- LCD filter -->
  <match target="font">
    <edit name="lcdfilter" mode="assign"><const>lcddefault</const></edit>
  </match>

  <!-- Default sans-serif: Noto Sans -->
  <alias>
    <family>sans-serif</family>
    <prefer><family>Noto Sans</family></prefer>
  </alias>

  <!-- Default monospace: JetBrains Mono -->
  <alias>
    <family>monospace</family>
    <prefer><family>JetBrains Mono</family></prefer>
  </alias>
</fontconfig>
```

### Step 7: Configure NetworkManager

**`/etc/NetworkManager/conf.d/levsha.conf`:**

```ini
[main]
dns=default
no-auto-default=

[connection]
ipv6.method=auto
```

No further configuration needed. NetworkManager auto-detects the single ethernet interface and runs DHCP.

---

## 3. Systemd Unit Configuration

### System Services (root)

| Unit | Type | WantedBy | Description |
|------|------|----------|-------------|
| `NetworkManager.service` | system | multi-user.target | Network management |
| `NetworkManager-wait-online.service` | system | network-online.target | Blocks until network is ready |
| `systemd-journald.service` | system | (early boot) | Centralized logging |
| `levsha-compositor.service` | system | graphical.target | Cage compositor + Chat Shell |

### User Services (levsha)

| Unit | Type | Description |
|------|------|-------------|
| `pipewire.socket` | user | PipeWire audio socket activation |
| `pipewire-pulse.socket` | user | PulseAudio compat socket |
| `wireplumber.service` | user | PipeWire session manager |
| `levsha-engine.service` | user | Intelligence Engine (if running as separate service) |

---

## 4. Filesystem Layout

```
/
├── boot/
│   ├── vmlinuz-*                       # Kernel
│   ├── initramfs-*                     # Initial ramdisk
│   └── efi/                            # UEFI boot partition
├── etc/
│   ├── fonts/local.conf                # Font rendering config
│   ├── hostname                        # "levsha"
│   ├── NetworkManager/
│   │   └── conf.d/levsha.conf          # NM configuration
│   ├── systemd/
│   │   └── system/
│   │       └── levsha-compositor.service
│   └── levsha/
│       └── config.toml                 # API key, model config
├── home/levsha/
│   ├── .local/share/levsha/
│   │   └── history.db                  # SQLite conversation history
│   └── .config/systemd/user/           # User service overrides
├── usr/
│   ├── bin/
│   │   ├── levsha-chat                 # Chat Shell binary (Rust)
│   │   └── levsha-engine               # Intelligence Engine binary (Rust)
│   └── share/levsha/
│       └── skills/
│           ├── package-manager/        # Built-in skill: pkg mgmt
│           │   ├── skill.yaml
│           │   ├── prompt.md
│           │   └── tools/
│           └── system-info/            # Built-in skill: sysinfo
│               ├── skill.yaml
│               ├── prompt.md
│               └── tools/
└── var/
    └── log/journal/                    # Persistent journal logs
```

---

## 5. Kickstart Skeleton

The Fedora kickstart file defines the base image. Full kickstart lives in `base/kickstart/levsha.ks`.

```kickstart
# Levsha OS Kickstart — MVP Base System
lang en_US.UTF-8
keyboard us
timezone UTC --utc
rootpw --lock
user --name=levsha --groups=wheel --password=levsha --plaintext

# Partitioning
clearpart --all
autopart --type=plain --nohome

# Bootloader
bootloader --location=mbr

# Network
network --bootproto=dhcp --device=link --activate --hostname=levsha

# Package selection
%packages
@core
cage
mesa-dri-drivers
NetworkManager
pipewire
pipewire-pulseaudio
wireplumber
google-noto-sans-fonts
google-noto-sans-mono-fonts
jetbrains-mono-fonts
google-noto-emoji-color-fonts
fontconfig
freetype
sqlite
curl
sudo
procps-ng
iproute
qemu-guest-agent
-plymouth
-firewalld
-sssd-common
-abrt
%end

# Post-install configuration
%post
# Enable compositor service
systemctl enable levsha-compositor.service
systemctl set-default graphical.target

# Disable unnecessary services
systemctl disable bluetooth.service 2>/dev/null || true
systemctl disable cups.service 2>/dev/null || true
systemctl disable avahi-daemon.service 2>/dev/null || true

# Configure sudo for levsha (no password)
echo "levsha ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/levsha

# Set hostname
hostnamectl set-hostname levsha
%end
```

---

## 6. Integration Points with L2/L3

### Intelligence Engine (L2) Integration

The base system provides:

- **Network connectivity** — Engine requires outbound HTTPS to `api.anthropic.com`
- **systemd service management** — Engine runs as a user service or is spawned by the Chat Shell
- **Configuration file** — `/etc/levsha/config.toml` contains API key and model selection
- **System commands** — Engine executes commands (`dnf`, `free`, `ps`, `ip`) with full root access via sudo

### Chat Shell (L3) Integration

The base system provides:

- **Wayland compositor** — cage provides the Wayland socket; Chat Shell connects as a client
- **Font rendering** — fontconfig + FreeType deliver antialiased text to the GTK4/Iced rendering pipeline
- **Input handling** — cage passes keyboard/mouse events to the Chat Shell via the Wayland protocol
- **Process management** — systemd restarts the Chat Shell on crash
- **Display resolution** — cage reports display dimensions; Chat Shell adapts its layout

### Shared Interface Contract

```
Base System → Chat Shell:
  - Wayland socket: $XDG_RUNTIME_DIR/wayland-0
  - Font config: /etc/fonts/local.conf
  - Display dimensions: via Wayland protocol (wl_output)

Base System → Intelligence Engine:
  - Config: /etc/levsha/config.toml
  - Network: outbound HTTPS on port 443
  - Commands: /usr/bin/dnf, /usr/bin/free, /usr/bin/ps, /usr/bin/ip, etc.
  - Sudo: passwordless via /etc/sudoers.d/levsha

Chat Shell → Intelligence Engine:
  - IPC: Unix socket or stdin/stdout pipe (defined by L2/L3 design)
  - History DB: /home/levsha/.local/share/levsha/history.db
```

---

## 7. Verification Checklist

| # | Check | Command / Method |
|---|-------|-----------------|
| 1 | Kernel boots | VM starts, `uname -r` returns Fedora kernel version |
| 2 | systemd is PID 1 | `ps -p 1 -o comm=` returns `systemd` |
| 3 | Network has IP | `ip addr show` shows DHCP-assigned address |
| 4 | DNS resolves | `curl -s https://api.anthropic.com` does not fail on DNS |
| 5 | Compositor running | `systemctl status levsha-compositor` is active |
| 6 | Chat Shell rendering | Visual: full-screen GUI with welcome message |
| 7 | PipeWire running | `systemctl --user status pipewire` is active |
| 8 | Font rendering | Visual: text is antialiased, no bitmap/jagged edges |
| 9 | Crash recovery | `kill -9 $(pidof levsha-chat)` restarts within 2s |
| 10 | Memory < 1 GB | `free -m` total used < 1024 at idle |
| 11 | Boot < 30s | Timed from VM power-on to Chat Shell input cursor |
| 12 | No TTY escape | Ctrl+Alt+F2 does not switch away from compositor |

---

## 8. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `cage` not packaged in Fedora repos | Cannot install via dnf | Build from source in kickstart `%post`, or use COPR repo. cage is a small C project with few dependencies. |
| VirtIO GPU not supported by cage | No display output | Fall back to `WLR_RENDERER=pixman` (software rendering). Acceptable for MVP. |
| Boot time exceeds 30s | Fails acceptance criteria | Profile with `systemd-analyze blame`, disable slow services, consider `dracut` optimizations. |
| Mesa drivers missing for VM GPU | Black screen or crash | Include `mesa-dri-drivers` which covers swrast, virtio, and vmware. |
| Font packages bloat disk | Exceeds 4 GB disk target | Use only Latin subset of Noto if needed; omit CJK fonts in MVP. |
