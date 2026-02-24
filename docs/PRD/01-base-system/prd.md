# 01 — Base System: Product Requirements

**Module:** L0 (Kernel) + L1 (Base System)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

The base system is the foundation layer of Levsha OS. It provides everything needed to boot the machine, initialize hardware, start a Wayland compositor, configure networking, and hand control to the Chat Shell (L3) via the Intelligence Engine (L2).

The base system is invisible to the user. There is no shell, no TTY, no login prompt. The only user-facing artifact is the boot splash, followed by the Chat Shell GUI filling the screen.

**Base distribution:** Fedora (minimal), chosen for excellent GTK4/libadwaita support, mature Wayland stack, and polished font rendering out of the box.

---

## 2. Scope

### In Scope (MVP)

- Linux kernel with minimal configuration (Fedora default kernel)
- systemd as init system and service manager
- Wayland compositor (wlroots-based) running as the sole display server
- Auto-configured networking via NetworkManager + DHCP on virtualized ethernet
- Audio subsystem via PipeWire
- Font rendering with subpixel antialiasing and HiDPI readiness
- Auto-login into the Chat Shell (no login screen, no user prompt)
- Automatic restart of the Chat Shell on crash (systemd watchdog)

### Out of Scope (MVP)

- Wi-Fi configuration (VM-only, ethernet assumed)
- Bare metal hardware support and driver management
- Multi-user support
- Firewall configuration beyond defaults
- Power management / suspend / hibernate
- GPU-accelerated rendering (software rendering acceptable in VM)

---

## 3. Functional Requirements

### 3.1 Boot and Initialization

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BS-01 | System boots from ISO in VirtualBox/QEMU to Chat Shell GUI in under 30 seconds from cold start. | P0 | BR-01 |
| BS-02 | No login screen. Single auto-login user (`levsha`) launches directly into the Chat Shell. | P0 | BR-02 |
| BS-03 | Boot splash displays Levsha OS logo with an animated progress indicator during startup. | P1 | BR-06 |
| BS-04 | systemd is the init system. All services are managed as systemd units. | P0 | — |
| BS-05 | The kernel is the Fedora default kernel with no custom compilation required. Minimal module set loaded at boot. | P0 | — |

### 3.2 Networking

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BS-10 | Network is auto-configured via DHCP on virtualized ethernet. No user interaction required. | P0 | BR-03 |
| BS-11 | NetworkManager runs as a systemd service, managing the single ethernet interface. | P0 | BR-03 |
| BS-12 | DNS resolution works immediately after boot (required for cloud API connectivity). | P0 | BR-03 |
| BS-13 | Network status (connected/disconnected, IP address) is queryable by the Intelligence Engine. | P0 | SY-05 |

### 3.3 Display and Compositor

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BS-20 | A wlroots-based Wayland compositor runs as the sole display server. No X11 or XWayland. | P0 | — |
| BS-21 | The compositor launches automatically on boot via a systemd user session. | P0 | — |
| BS-22 | The compositor supports a single full-screen surface (the Chat Shell). No window management, no decorations, no taskbar. | P0 | CS-01 |
| BS-23 | Minimum display resolution: 1280x720. Recommended: 1920x1080. | P0 | 8.2 |
| BS-24 | The compositor provides basic input handling (keyboard, mouse/touchpad). | P0 | — |
| BS-25 | Font rendering uses subpixel antialiasing (FreeType + fontconfig). HiDPI scale factors are detected automatically. | P0 | 9.1 |

### 3.4 Audio

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BS-30 | PipeWire runs as the audio server, replacing PulseAudio. | P0 | — |
| BS-31 | Audio output works with virtualized sound hardware (AC97/HDA) without user configuration. | P0 | — |
| BS-32 | Audio is not required for MVP functionality but must not cause errors if unavailable. | P0 | — |

### 3.5 Service Management

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BS-40 | The Chat Shell process is managed by systemd. If it crashes, systemd restarts it automatically within 2 seconds. | P0 | 8.4 |
| BS-41 | The Intelligence Engine is started as a systemd service, ready before the Chat Shell accepts input. | P0 | IE-01 |
| BS-42 | All critical services (compositor, Chat Shell, engine, NetworkManager) have explicit systemd dependency ordering. | P0 | — |
| BS-43 | systemd-journald provides centralized logging for all services. | P0 | — |

---

## 4. Non-Functional Requirements

### 4.1 Performance

| Metric | Target |
|--------|--------|
| Boot to interactive Chat Shell | < 30 seconds (cold start in VM) |
| Idle RAM (full system) | < 1 GB |
| Idle disk footprint | < 4 GB |
| GUI frame rate | 60 fps (scrolling, animations) |

### 4.2 Reference VM Environment

| Spec | Requirement |
|------|-------------|
| Hypervisor | VirtualBox 7+ or QEMU/KVM |
| vCPUs | 2+ |
| RAM | 2 GB |
| Disk | 8 GB virtual disk |
| Display | 1280x720 minimum, 1920x1080 recommended |
| Network | NAT or Bridged (ethernet, auto-DHCP) |
| Sound | Emulated (AC97 or Intel HDA) |

### 4.3 Reliability

- If the Chat Shell crashes, systemd restarts it. No manual recovery needed.
- If networking fails at boot, NetworkManager retries. The Chat Shell displays an offline indicator.
- The system must not drop to a TTY or emergency shell under any normal failure condition.

### 4.4 Security

- Single user (`levsha`) with sudo/root access for package management.
- No remote SSH access enabled by default.
- SELinux in permissive mode (Fedora default; not enforcing in MVP).
- No firewall rules beyond Fedora defaults.

---

## 5. Acceptance Criteria

1. **Boot test:** ISO boots in QEMU with 2 vCPU / 2 GB RAM / 8 GB disk and reaches the Chat Shell GUI within 30 seconds.
2. **Network test:** `curl https://api.anthropic.com` succeeds from within the VM immediately after boot.
3. **Display test:** The Chat Shell renders full-screen on a 1920x1080 virtual display with no visual artifacts.
4. **Crash recovery test:** Killing the Chat Shell process (`kill -9`) results in automatic restart within 2 seconds.
5. **Audio test:** PipeWire is running (`systemctl --user status pipewire`) with no errors.
6. **No TTY test:** Pressing Ctrl+Alt+F2 does not switch to a text console. The compositor holds the display.
7. **Idle resources test:** `free -m` shows total used memory below 1024 MB at idle.
8. **Font test:** Text in the Chat Shell renders with subpixel antialiasing (visual inspection).

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| L2 (Intelligence Engine) | Upstream | Base system provides systemd service management and network connectivity for the engine. |
| L3 (Chat Shell) | Upstream | Base system provides Wayland compositor, font rendering, and input handling for the GUI. |
| ISO Build (infra) | Downstream | ISO build consumes the base system package list and kickstart configuration. |

---

## 7. Traceability

| PRD Requirement | Base System Requirement |
|-----------------|------------------------|
| BR-01 (boot < 30s) | BS-01 |
| BR-02 (no login) | BS-02 |
| BR-03 (auto-network) | BS-10, BS-11, BS-12 |
| BR-06 (boot splash) | BS-03 |
| CS-01 (full-screen GUI) | BS-22 |
| SY-05 (network status) | BS-13 |
| 8.2 (VM specs) | BS-23, 4.2 |
| 8.4 (crash restart) | BS-40 |
