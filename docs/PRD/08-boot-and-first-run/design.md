# 08 — Boot and First Run: Design

> **Visual styling follows [theme.design.md](../theme.design.md).**

**Module:** Boot Sequence and First Boot Experience
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document details the design of the Levsha OS boot sequence — from firmware to interactive Chat Shell — and the first-run experience that greets new users. Every transition in the boot chain is designed for speed and visual polish, with zero user interaction required.

---

## 2. Boot Sequence Diagram

```
Power On
  |
  v
BIOS/UEFI (VM firmware)
  |
  v
GRUB (timeout=0, immediate boot)
  |
  v
Linux Kernel (Fedora default, quiet boot)
  |
  v
systemd (default.target)
  |-- NetworkManager.service (DHCP)
  |-- levsha-engine.service (Intelligence Engine)
  |
  v
getty@tty1 / greetd (auto-login as 'levsha')
  |
  v
User session starts (~/.bash_profile or systemd --user)
  |
  v
Wayland compositor (sway / cage)
  |
  v
Chat Shell (full-screen, single window)
  |
  v
[First boot?] --> Welcome message
[Subsequent boot?] --> Restore conversation history
```

Total target: under 30 seconds from GRUB to interactive Chat Shell.

---

## 3. Auto-Login Mechanism

### Option A: getty auto-login (Recommended)

Use systemd's `getty` with auto-login override:

```
# /etc/systemd/system/getty@tty1.service.d/autologin.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin levsha --noclear %I $TERM
```

After login, the user's shell profile starts the Wayland compositor.

### Option B: greetd

Use `greetd` with `agreety` configured for auto-login:

```toml
# /etc/greetd/config.toml
[terminal]
vt = 1

[default_session]
command = "sway"
user = "levsha"
```

**Decision:** Option A (getty auto-login) is simpler and has fewer dependencies. Use this for MVP.

---

## 4. Wayland Compositor Auto-Start

After auto-login, the compositor starts from the user's shell profile.

**~/.bash_profile:**

```bash
# Start Wayland compositor on tty1 only
if [ "$(tty)" = "/dev/tty1" ]; then
    exec sway
fi
```

**Compositor choice for MVP:**

| Compositor | Pros | Cons |
|------------|------|------|
| **sway** | Mature, wlroots-based, well-documented | Tiling WM features unnecessary |
| **cage** | Single-app kiosk mode (perfect fit) | Less mature, fewer features |

**Recommendation:** `cage` is ideal — it runs a single application full-screen with no window management overhead. Fallback to `sway` if `cage` has compatibility issues.

**Cage launch:**

```bash
if [ "$(tty)" = "/dev/tty1" ]; then
    exec cage levsha-chat-shell
fi
```

---

## 5. Chat Shell Auto-Launch

With `cage`, the Chat Shell is the compositor's sole managed application. It launches as the argument to `cage`. With `sway`, the Chat Shell is configured as the startup application:

```
# ~/.config/sway/config
exec levsha-chat-shell
```

The Chat Shell binary is installed at `/usr/bin/levsha-chat-shell`.

---

## 6. Plymouth Boot Splash Design

Plymouth provides a graphical splash screen during the kernel and systemd initialization phases, before the Wayland compositor starts.

### Theme Design

All visual properties below are defined in [theme.design.md, Section 7 — Boot Splash](../theme.design.md#7-boot-splash).

- **Background:** Warm parchment `bg-primary` (`#FBF8F3`).
- **Logo:** `◆ Levsha OS` — diamond in `accent-copper` (`#C67A52`), text in `text-primary` warm brown (`#3A3228`), IBM Plex Sans 28px weight 600, centered vertically and horizontally.
- **Progress indicator:** 3 small dots (6px, 12px gap), sequential fade animation (1200ms cycle, 400ms per dot), `accent-copper` (`#C67A52`), centered 48px below logo.
- **Text:** None. No boot messages, no status text. Pure logo and dots.
- **Overall feel:** Warm parchment background, copper accents. Minimal, centered, calm. No loading bars, no percentages.

### Theme Implementation

```
/usr/share/plymouth/themes/levsha/
  levsha.plymouth          # Theme descriptor
  levsha.script            # Plymouth script for animation
  logo.png                 # Logo image (centered)
  spinner.png              # Spinner frames (if sprite-based)
```

**levsha.plymouth:**

```ini
[Plymouth Theme]
Name=Levsha OS
Description=Levsha OS boot splash
ModuleName=script

[script]
ImageDir=/usr/share/plymouth/themes/levsha
ScriptFile=/usr/share/plymouth/themes/levsha/levsha.script
```

### Smooth Transition

- Plymouth runs on the kernel framebuffer (KMS).
- When the Wayland compositor starts, Plymouth deactivates. The compositor takes over the display.
- To avoid flicker: both Plymouth and the compositor use the same warm cream background (`bg-primary` `#FBF8F3`). The transition is a seamless fade from splash to Chat Shell.
- Kernel parameters: `quiet splash plymouth.enable=1 vt.global_cursor_default=0` to suppress text output during boot.

---

## 7. Welcome Message Content

Visual styling for the welcome screen is defined in [theme.design.md, Section 8 — Welcome Screen](../theme.design.md#8-welcome-screen). Background is `bg-primary` (`#FBF8F3`), title uses `text-xl` with `accent-copper` (`#C67A52`) diamond and `text-primary` (`#3A3228`) warm brown text in IBM Plex Sans, body uses `text-base` at `text-primary`.

Displayed as a system/assistant message on first boot:

```
Welcome. I'm your operating system.
Everything you need — just ask.

I can manage your packages and tell you about your system.
More skills are coming soon.
```

This matches the PRD section 6.4 mockup exactly. The message is:
- Injected by the Intelligence Engine as a `system` role message.
- Stored in the persistence layer (survives reboot if history is not cleared).
- Not repeated on subsequent boots — the engine checks if the conversation has existing messages.

---

## 8. First-Run Detection

The Intelligence Engine determines whether this is a first run by checking the persistence layer:

```
On startup:
  1. Open SQLite database
  2. Query for active conversation
  3. If no conversation exists OR conversation has zero messages:
       -> This is a first run. Insert welcome message.
  4. If conversation has messages:
       -> Not first run. Load history. Skip welcome.
```

No external flag file or registry is needed. The database state is the single source of truth.

---

## 9. API Connectivity Check

On startup, the Intelligence Engine performs a lightweight connectivity check:

1. Read API key from `/etc/levsha/config.toml`.
2. If key is missing: display error in Chat Shell status bar ("no API key").
3. If key exists: make a minimal API request (or HTTPS HEAD to `api.anthropic.com`) to verify connectivity.
4. Update Chat Shell status bar: "connected" or "disconnected".
5. If disconnected: the Chat Shell remains usable for scrolling history, but new messages show an error prompting retry.

The connectivity check runs asynchronously and does not block the Chat Shell from appearing. The user sees the Chat Shell immediately; the status bar updates once the check completes.

---

## 10. Service Dependency Order

```
multi-user.target
  |
  +-- NetworkManager.service (networking)
  |
  +-- levsha-engine.service (Intelligence Engine)
  |     Requires: NetworkManager-wait-online.service (optional, soft dep)
  |     After: NetworkManager.service
  |
  +-- getty@tty1.service (auto-login)
        After: levsha-engine.service
        |
        +-- User session: cage + levsha-chat-shell
```

The Intelligence Engine starts as a system service before the user session. By the time the Chat Shell launches, the engine is already running and the API connection check is in progress.

---

## 11. Configuration File

```toml
# /etc/levsha/config.toml
[api]
provider = "anthropic"
key = "sk-ant-..."
model = "claude-sonnet-4-20250514"
base_url = "https://api.anthropic.com"

[persistence]
database_path = "/var/lib/levsha/history.db"

[ui]
theme = "light"  # Light theme (see theme.design.md)
```

The config file is baked into the ISO during build. In MVP, it is not user-editable through the chat interface.
