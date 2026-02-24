# 09 — ISO Build: Product Requirements

**Module:** ISO Generation and VM Testing
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

The Levsha OS ISO is the sole distribution artifact of the MVP. It is a bootable live image that runs in VirtualBox or QEMU, containing the complete system: kernel, base system, Wayland compositor, Intelligence Engine, Chat Shell, built-in skills, and all dependencies. There is no network install — everything needed to run the system is on the ISO.

The ISO build pipeline takes the Fedora minimal base, applies Levsha OS customizations via a kickstart file and filesystem overlay, and produces a ready-to-boot image.

---

## 2. Scope

### In Scope (MVP)

- Bootable live ISO image for VirtualBox 7+ and QEMU/KVM
- Based on Fedora minimal with Levsha OS customizations
- All Levsha OS components pre-installed (no post-boot setup)
- ISO includes all packages, no network dependency during boot
- Reference environment: 2+ vCPUs, 2 GB RAM, 8 GB disk, 1280x720 minimum display
- Build pipeline using Fedora's lorax/livemedia-creator tooling
- QEMU test scripts for automated boot validation

### Out of Scope (MVP)

- Installer (the ISO is a live image, not an installation medium)
- Persistent storage across reboots on live media (VM disk image is separate)
- Bare metal hardware support
- ARM64 or other non-x86_64 architectures
- Signed/Secure Boot images
- OTA updates or delta builds
- Container-based deployment

---

## 3. Functional Requirements

### 3.1 ISO Contents

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| IB-01 | The ISO contains a complete, bootable Levsha OS based on Fedora minimal. | P0 | 6.1 |
| IB-02 | All Levsha OS components are pre-installed: Chat Shell binary, Intelligence Engine binary, built-in skills, configuration files, Plymouth theme. | P0 | 6.2 |
| IB-03 | All system dependencies (Wayland compositor, fonts, SQLite, network tools) are included. No package downloads required at boot. | P0 | 6.1 |
| IB-04 | The API key is embedded in the configuration file on the ISO. | P0 | BR-04 |
| IB-05 | The ISO boots without requiring network access (network is used only for API calls after boot). | P0 | 6.1 |

### 3.2 VM Compatibility

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| IB-10 | The ISO boots successfully in VirtualBox 7+ with default VM settings (2 vCPU, 2 GB RAM, 8 GB disk). | P0 | 8.2 |
| IB-11 | The ISO boots successfully in QEMU/KVM with equivalent settings. | P0 | 8.2 |
| IB-12 | The VM display works at 1280x720 minimum and 1920x1080 recommended resolution. | P0 | 8.2 |
| IB-13 | VM guest drivers for VirtIO (disk, network) are included in the ISO. | P0 | — |
| IB-14 | VirtualBox Guest Additions or SPICE/QXL drivers are included for display scaling. | P1 | — |

### 3.3 Build Pipeline

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| IB-20 | The ISO is built using Fedora's lorax/livemedia-creator tooling with a kickstart file. | P0 | 9 |
| IB-21 | The build is reproducible: same inputs produce functionally identical ISOs. | P0 | — |
| IB-22 | The build can run on a Fedora host or in a Fedora container (for CI). | P0 | — |
| IB-23 | The build completes within 30 minutes on a modern machine (8+ cores, 16 GB RAM, SSD). | P1 | — |

---

## 4. Non-Functional Requirements

| Requirement | Target |
|-------------|--------|
| ISO size | < 1.5 GB |
| Installed disk footprint | < 4 GB |
| Boot to interactive chat (VM) | < 30 seconds |
| Idle RAM usage | < 1 GB |
| Build time | < 30 minutes |
| Supported hypervisors | VirtualBox 7+, QEMU/KVM |
| Architecture | x86_64 only |

---

## 5. Acceptance Criteria

1. **VirtualBox boot:** ISO boots to interactive Chat Shell in VirtualBox with 2 vCPU, 2 GB RAM, 8 GB disk within 30 seconds.
2. **QEMU boot:** ISO boots to interactive Chat Shell in QEMU with equivalent settings within 30 seconds.
3. **Display resolution:** Chat Shell renders correctly at 1280x720 and 1920x1080.
4. **No network install:** Disconnect network before boot; system reaches Chat Shell (with "disconnected" status).
5. **End-to-end flow:** User types a message, receives a response from Claude API (network connected).
6. **ISO size:** Generated ISO is under 1.5 GB.
7. **Idle resources:** `free -m` shows total used memory under 1024 MB at idle after boot.
8. **Build reproducibility:** Two consecutive builds from the same commit produce ISOs with identical package sets.
9. **Automated test:** QEMU boot test script confirms boot success without manual intervention.

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| L1 (Base System) | Input | Package list, systemd configuration, auto-login setup. |
| L2 (Intelligence Engine) | Input | Engine binary, configuration file with API key. |
| L3 (Chat Shell) | Input | Chat Shell binary, assets (fonts, icons). |
| 07-Persistence | Input | Database initialization and schema. |
| 08-Boot and First Run | Input | Boot sequence configuration, Plymouth theme, welcome message. |
| All built-in skills | Input | Skill manifests and tool definitions. |

---

## 7. Traceability

| PRD Requirement | ISO Build Requirement |
|-----------------|----------------------|
| 6.1 (VM development environment) | IB-01, IB-05, IB-10, IB-11 |
| 6.2 (bootable ISO) | IB-01, IB-02, IB-03 |
| 8.2 (reference environment) | IB-10, IB-11, IB-12 |
| 9 (lorax/livemedia-creator) | IB-20, IB-22 |
| BR-04 (hardcoded API key) | IB-04 |
