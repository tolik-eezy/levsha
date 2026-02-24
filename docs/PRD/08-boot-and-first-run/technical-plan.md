# 08 — Boot and First Run: Technical Plan

**Module:** Boot Sequence and First Boot Experience
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document provides the step-by-step implementation plan for the Levsha OS boot sequence, from systemd target configuration through to the welcome message display. Each section maps to a specific component that must be built, configured, or integrated.

---

## 2. systemd Target Configuration

The default boot target is `graphical.target`, but Levsha OS strips it down to the minimum needed services.

**Disabled services (not needed in MVP):**

```bash
systemctl disable bluetooth.service
systemctl disable cups.service
systemctl disable firewalld.service
systemctl disable gdm.service          # No display manager
systemctl disable sshd.service         # No remote access
systemctl mask emergency.service       # No emergency shell
systemctl mask rescue.service          # No rescue shell
systemctl mask debug-shell.service     # No debug TTY
```

**Enabled services:**

```bash
systemctl enable NetworkManager.service
systemctl enable levsha-engine.service
systemctl enable getty@tty1.service     # With auto-login override
```

---

## 3. Auto-Login Setup

### getty override

Create the drop-in directory and configuration:

```bash
mkdir -p /etc/systemd/system/getty@tty1.service.d/
```

```ini
# /etc/systemd/system/getty@tty1.service.d/autologin.conf
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin levsha --noclear %I $TERM
Type=idle
```

### User account

Created in the kickstart file during ISO build:

```
user --name=levsha --groups=wheel --homedir=/home/levsha --shell=/bin/bash
```

No password is set (passwordless login). The user has `wheel` group membership for `sudo` access without password (configured via `/etc/sudoers.d/levsha`):

```
levsha ALL=(ALL) NOPASSWD: ALL
```

---

## 4. Wayland Compositor Auto-Start

### Shell profile

```bash
# /home/levsha/.bash_profile
if [ "$(tty)" = "/dev/tty1" ] && [ -z "$WAYLAND_DISPLAY" ]; then
    export XDG_SESSION_TYPE=wayland
    export XDG_CURRENT_DESKTOP=sway
    export MOZ_ENABLE_WAYLAND=1
    exec cage -- levsha-chat-shell 2>/dev/null
fi
```

### cage installation

cage is available in Fedora repositories:

```bash
dnf install cage
```

If cage is unavailable or unstable, fallback to sway with a minimal config:

```
# /home/levsha/.config/sway/config
# Minimal sway config for kiosk mode
output * bg #0D0D0D solid_color
default_border none
gaps inner 0
gaps outer 0
exec levsha-chat-shell
```

---

## 5. Chat Shell Auto-Launch

The Chat Shell binary is installed at `/usr/bin/levsha-chat-shell`. When launched by cage, it:

1. Connects to the Wayland compositor (via `wayland-client` / GTK4 / Iced).
2. Creates a single full-screen surface.
3. Connects to the Intelligence Engine via local IPC (Unix socket at `/run/levsha/engine.sock`).
4. Loads conversation history from the persistence layer.
5. Renders the chat interface with history (or welcome message on first boot).
6. Activates the input field — the user can type immediately.

If the Intelligence Engine is not yet ready, the Chat Shell displays "Connecting..." in the status bar and retries the IPC connection every 500ms.

---

## 6. Intelligence Engine Service

### systemd unit

```ini
# /etc/systemd/system/levsha-engine.service
[Unit]
Description=Levsha OS Intelligence Engine
After=NetworkManager.service
Wants=NetworkManager-wait-online.service

[Service]
Type=notify
User=levsha
Group=levsha
ExecStart=/usr/bin/levsha-engine
Restart=always
RestartSec=2
Environment=LEVSHA_CONFIG=/etc/levsha/config.toml

[Install]
WantedBy=multi-user.target
```

The engine uses `sd_notify` to signal readiness to systemd after completing:
- Configuration file load
- Database initialization (persistence layer)
- API connectivity check (non-blocking; engine is "ready" before API confirms)

---

## 7. Plymouth Theme Creation

### Theme files

```
base/overlay/usr/share/plymouth/themes/levsha/
  levsha.plymouth
  levsha.script
  logo.png
```

### levsha.plymouth

```ini
[Plymouth Theme]
Name=Levsha OS
Description=Levsha OS minimal boot splash
ModuleName=script

[script]
ImageDir=/usr/share/plymouth/themes/levsha
ScriptFile=/usr/share/plymouth/themes/levsha/levsha.script
```

### levsha.script (minimal spinner)

```
Window.SetBackgroundTopColor(0.05, 0.05, 0.05);
Window.SetBackgroundBottomColor(0.05, 0.05, 0.05);

logo = Image("logo.png");
logo_sprite = Sprite(logo);
logo_sprite.SetX(Window.GetWidth() / 2 - logo.GetWidth() / 2);
logo_sprite.SetY(Window.GetHeight() / 2 - logo.GetHeight() / 2 - 40);

// Simple pulsing effect
progress = 0;
fun refresh_callback() {
    progress += 0.02;
    opacity = Math.Abs(Math.Sin(progress));
    logo_sprite.SetOpacity(0.5 + opacity * 0.5);
}
Plymouth.SetRefreshFunction(refresh_callback);
```

### Installation

```bash
plymouth-set-default-theme levsha
dracut -f  # Rebuild initramfs with Plymouth theme
```

### Kernel parameters

Added to GRUB config:

```
GRUB_CMDLINE_LINUX="quiet splash plymouth.enable=1 vt.global_cursor_default=0 rd.udev.log_level=3 systemd.show_status=false"
```

---

## 8. Welcome Message Implementation

The Intelligence Engine handles first-run detection and welcome message insertion.

```rust
pub fn ensure_welcome_message(store: &HistoryStore) -> Result<()> {
    let conv = store.active_conversation()?;
    let messages = store.load_all_messages(conv.id)?;
    if messages.is_empty() {
        let welcome = "\
Welcome. I'm your operating system.\n\
Everything you need — just ask.\n\
\n\
I can manage your packages and tell you about your system.\n\
More skills are coming soon.";
        store.insert_message(conv.id, Role::System, welcome, None)?;
    }
    Ok(())
}
```

Called once during engine startup, after the persistence layer is initialized.

---

## 9. Boot Time Optimization

### Measurement

```bash
systemd-analyze                         # Total boot time
systemd-analyze blame                   # Per-service breakdown
systemd-analyze critical-chain          # Critical path
systemd-analyze plot > boot.svg         # Visual timeline
```

### Optimization strategies

| Strategy | Expected Savings |
|----------|-----------------|
| Set GRUB timeout to 0 | 3-5 seconds |
| Disable unused services (bluetooth, cups, sshd, gdm) | 2-4 seconds |
| Use `Type=idle` for getty (defer until system is ready) | Scheduling improvement |
| Minimal initramfs (only needed drivers) | 1-2 seconds |
| `quiet splash` kernel params (no console output) | Perceived improvement |
| Parallel service startup (systemd default) | Already enabled |
| Mask emergency/rescue targets | Prevents fallback delays |
| Remove unnecessary kernel modules from initramfs | 0.5-1 second |

### Target breakdown

| Phase | Target |
|-------|--------|
| GRUB to kernel | < 1 second |
| Kernel + initramfs | < 5 seconds |
| systemd to default.target | < 10 seconds |
| User session + compositor | < 5 seconds |
| Chat Shell render + ready | < 3 seconds |
| **Total** | **< 24 seconds** (6 second buffer) |

---

## 10. Testing in VM

### Automated boot test (QEMU)

```bash
#!/bin/bash
# infra/scripts/test-boot.sh
ISO="build/levsha-os.iso"
TIMEOUT=45

qemu-system-x86_64 \
    -m 2048 \
    -smp 2 \
    -drive file="${ISO}",format=raw,if=virtio \
    -display none \
    -serial stdio \
    -net nic,model=virtio \
    -net user \
    -boot d \
    &

QEMU_PID=$!

# Wait for boot indicator (engine writes a marker file)
START=$(date +%s)
while [ $(($(date +%s) - START)) -lt $TIMEOUT ]; do
    # Check for ready signal via serial console or SSH
    sleep 1
done

# Verify boot completed
kill $QEMU_PID
```

### Manual test checklist

| # | Test | Pass Criteria |
|---|------|--------------|
| 1 | Cold boot timing | Chat Shell interactive in < 30 seconds |
| 2 | No login prompt visible | Auto-login with no password prompt |
| 3 | Plymouth splash | Logo and spinner visible during boot (P1) |
| 4 | Smooth transition | No flicker between splash and Chat Shell |
| 5 | Network connectivity | Status bar shows "connected" |
| 6 | Welcome message | First boot shows welcome text |
| 7 | History restore | Second boot shows previous conversation |
| 8 | API key loaded | First user message gets a response |
| 9 | Offline boot | Disconnected network shows status, no crash |
| 10 | No TTY escape | Ctrl+Alt+F2 does not switch to text console |

### Cross-Module Integration Tests

These tests verify the boot chain is correctly wired across all layers. See `00-system-architecture/integration-checks.md` for full details.

| Check | Seam | What it verifies |
|-------|------|------------------|
| IC-50 | L1 -> compositor -> L3 -> L2 | Full boot sequence: Plymouth -> auto-login -> cage -> Chat Shell -> Engine connection in < 30s |
| IC-51 | systemd ordering | NetworkManager -> Engine -> Chat Shell dependency chain correct, no races |
| IC-52 | L3 + systemd | Chat Shell crash triggers restart, reconnects to Engine, reloads history |
| IC-53 | L2 + systemd | Engine crash triggers restart, re-initializes everything, Chat Shell reconnects |
| IC-54 | L1 TTY lockout | Ctrl+Alt+F2 does not escape to TTY |
| IC-60 | L1 network -> L2 -> L3 | Offline boot reaches Chat Shell without hanging, shows "disconnected" |
| IC-72 | Plymouth -> L3 | Seamless visual transition: same parchment background, no flicker |

---

## 11. Implementation Order

1. **User account and auto-login** — kickstart user creation, getty override.
2. **Compositor setup** — cage installation, shell profile for auto-start.
3. **Engine service unit** — systemd service file, dependency ordering.
4. **Chat Shell launch** — binary installation path, cage integration.
5. **Service ordering** — verify boot dependency chain with `systemd-analyze`.
6. **Plymouth theme** — logo, script, initramfs rebuild.
7. **Welcome message** — first-run detection in engine, message insertion.
8. **Boot optimization** — disable services, tune GRUB, measure with `systemd-analyze`.
9. **VM boot testing** — automated and manual tests.
