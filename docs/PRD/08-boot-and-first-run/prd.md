# 08 — Boot and First Run: Product Requirements

**Module:** Boot Sequence and First Boot Experience
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

The boot sequence is the user's first impression of Levsha OS. From the moment power is applied to the moment the chat cursor blinks, every step must be fast, silent, and polished. There is no login screen, no setup wizard, no configuration dialog. The system boots directly into the Chat Shell with a welcome message.

This module covers the full path from BIOS/UEFI through GRUB, kernel, systemd, Wayland compositor, and finally the Chat Shell GUI — plus the first-run experience that greets the user on initial boot.

---

## 2. Scope

### In Scope (MVP)

- Boot to interactive Chat Shell in under 30 seconds (VM cold start)
- Auto-login to the `levsha` user account with no login prompt
- Network auto-configuration via DHCP (no user interaction)
- Hardcoded API key in system configuration
- Welcome message displayed on first launch
- Boot splash with Levsha OS logo and animated progress indicator (P1)

### Out of Scope (MVP)

- User account creation or selection
- API key entry or configuration
- Language or locale selection
- Wi-Fi setup (VM-only, ethernet assumed)
- Bare metal boot support
- Secure Boot / TPM
- Dual-boot with other operating systems

---

## 3. Functional Requirements

### 3.1 Boot Performance

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BF-01 | System boots from ISO in VirtualBox/QEMU to an interactive Chat Shell GUI in under 30 seconds from cold start. | P0 | BR-01 |
| BF-02 | Boot time is measured from GRUB handoff to kernel until the Chat Shell input field is responsive to keystrokes. | P0 | BR-01 |
| BF-03 | GRUB timeout is set to 0 seconds (no menu displayed, immediate boot). | P0 | — |

### 3.2 Auto-Login

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BF-10 | No login screen is presented. The system auto-logs in as the `levsha` user. | P0 | BR-02 |
| BF-11 | The `levsha` user is created during ISO build with a pre-set password (or no password). | P0 | BR-02 |
| BF-12 | The Wayland compositor starts automatically as part of the `levsha` user session. | P0 | BR-02 |
| BF-13 | The Chat Shell launches automatically as the sole application in the compositor. | P0 | BR-02 |

### 3.3 Network

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BF-20 | NetworkManager starts on boot and auto-configures the first ethernet interface via DHCP. | P0 | BR-03 |
| BF-21 | DNS resolution is functional by the time the Chat Shell sends its first API request. | P0 | BR-03 |
| BF-22 | If DHCP fails, the Chat Shell displays a "no network" status indicator. No error dialog or popup. | P0 | CS-07 |

### 3.4 API Key

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BF-30 | The Anthropic API key is stored in a configuration file at a fixed path (`/etc/levsha/config.toml`). | P0 | BR-04 |
| BF-31 | The Intelligence Engine reads the API key on startup. No user prompt. | P0 | BR-04 |
| BF-32 | If the API key is missing or invalid, the Chat Shell displays a clear error message. | P0 | CS-07 |

### 3.5 Welcome Message

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BF-40 | On first launch, a welcome message is displayed as the first assistant message in the chat. | P0 | BR-05 |
| BF-41 | The welcome message introduces Levsha OS and lists current capabilities (package management, system info). | P0 | BR-05 |
| BF-42 | The welcome message is stored in the conversation history (persistence layer). | P0 | PM-01 |
| BF-43 | On subsequent boots, the previous conversation is restored. No welcome message is repeated. | P0 | PM-01 |
| BF-44 | After history clear, the welcome message is shown again as if it were a fresh boot. | P0 | PM-02 |

### 3.6 Boot Splash

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| BF-50 | A Plymouth boot splash displays the Levsha OS logo during kernel and systemd initialization. | P1 | BR-06 |
| BF-51 | The splash includes an animated progress indicator (spinner or progress bar) consistent with the Chat Shell's visual language. | P1 | BR-06 |
| BF-52 | The splash transitions smoothly to the Wayland compositor and Chat Shell (no flickering or mode switching visible). | P1 | BR-06 |

---

## 4. Non-Functional Requirements

| Requirement | Target |
|-------------|--------|
| Cold boot to interactive chat (VM) | < 30 seconds |
| GRUB to kernel handoff | < 1 second |
| Kernel to systemd target reached | < 10 seconds |
| Compositor + Chat Shell launch | < 5 seconds |
| Network ready (DHCP complete) | < 10 seconds after boot |
| Welcome message displayed | < 1 second after Chat Shell ready |

---

## 5. Acceptance Criteria

1. **30-second boot:** Fresh ISO boots in QEMU (2 vCPU, 2 GB RAM, 8 GB disk) to interactive Chat Shell within 30 seconds from cold start. Measured with `systemd-analyze`.
2. **No login screen:** No login prompt, password field, or user selection screen is visible at any point during boot.
3. **Network ready:** `curl https://api.anthropic.com` succeeds within 15 seconds of Chat Shell appearing.
4. **API key loaded:** The Intelligence Engine connects to the Claude API on first user message without any key prompt.
5. **Welcome message:** First boot shows the welcome message. Second boot shows the previous conversation with no repeated welcome.
6. **Clear and welcome:** Clearing history and rebooting shows the welcome message again.
7. **Boot splash (P1):** Levsha OS logo with animation is visible during boot, transitions to Chat Shell without flicker.
8. **Offline boot:** Booting without network shows "disconnected" in status bar but does not crash or show a raw error.

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| L1 (Base System) | Foundation | Provides systemd, auto-login, compositor, NetworkManager. |
| L2 (Intelligence Engine) | Launched at boot | Must be running before Chat Shell accepts input. |
| L3 (Chat Shell) | Launched at boot | The target of the entire boot sequence. |
| 07-Persistence | Data | Welcome message and conversation restore depend on the persistence layer. |
| 09-ISO Build | Build | Boot configuration is baked into the ISO via kickstart. |

---

## 7. Traceability

| PRD Requirement | Boot Requirement |
|-----------------|-----------------|
| BR-01 (boot < 30s) | BF-01, BF-02, BF-03 |
| BR-02 (no login) | BF-10, BF-11, BF-12, BF-13 |
| BR-03 (auto network) | BF-20, BF-21, BF-22 |
| BR-04 (hardcoded key) | BF-30, BF-31, BF-32 |
| BR-05 (welcome message) | BF-40, BF-41, BF-42, BF-43, BF-44 |
| BR-06 (boot splash) | BF-50, BF-51, BF-52 |
