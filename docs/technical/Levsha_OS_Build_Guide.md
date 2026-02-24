# Levsha OS — ISO Build Guide

How to build the Levsha OS bootable ISO from a Fedora kickstart file.

---

## Overview

Levsha OS is built as a custom Fedora spin. The entire OS definition lives in a single kickstart (`.ks`) file that tells Fedora's build tools exactly what to include, configure, and strip out. Feed the `.ks` file into `livemedia-creator` → get a bootable ISO.

```
levsha-os.ks  →  livemedia-creator  →  Levsha-OS.iso  →  VirtualBox / QEMU
```

---

## Prerequisites

You need a **Fedora host machine** (or Fedora VM) to build the ISO. The build tools are Fedora-specific.

### Install Build Tools

```bash
sudo dnf install -y \
  livecd-tools \
  lorax \
  anaconda \
  pykickstart \
  virt-install \
  libvirt \
  qemu-kvm
```

- `lorax` — contains `livemedia-creator`, the main ISO build tool
- `pykickstart` — kickstart file parser and validator (`ksvalidator`, `ksflatten`)
- `livecd-tools` — alternative simpler builder (`livecd-creator`)

### Enable Libvirt (for livemedia-creator)

```bash
sudo systemctl enable --now libvirtd
```

---

## Project Structure

```
levsha-os/
├── kickstart/
│   ├── levsha-os.ks           # Main kickstart file (the entire OS definition)
│   └── levsha-base.ks         # Optional: base packages, included by main
├── chat-shell/                # Chat Shell source (Rust/GTK4 project)
│   ├── src/
│   ├── Cargo.toml
│   └── ...
├── intelligence-engine/       # Intelligence Engine source
│   ├── src/
│   └── ...
├── skills/
│   ├── package-manager/       # Built-in skill: package management
│   └── sysinfo/               # Built-in skill: system info
├── config/
│   ├── config.toml              # System config (API key, defaults)
│   ├── levsha-chat.service     # systemd service for Chat Shell
│   └── levsha-session.service  # Wayland session launcher
├── branding/
│   ├── logo.png
│   ├── boot-splash/
│   └── wallpaper.png
├── build.sh                   # Build automation script
└── README.md
```

---

## The Kickstart File

This is the heart of the build. Everything Levsha OS is (and isn't) is defined here.

### levsha-os.ks

```kickstart
# Levsha OS Kickstart
# Fedora-based, minimal, chat-only operating system

#──────────────────────────────────────────────────────
# SYSTEM CONFIGURATION
#──────────────────────────────────────────────────────

# Language and locale
lang en_US.UTF-8
keyboard us
timezone UTC --utc

# Root password (development only — will be replaced by proper auth)
rootpw --plaintext levsha

# Create a default user with auto-login
user --name=levsha --password=levsha --plaintext --groups=wheel

# Network: auto-configure via DHCP (ethernet in VM)
network --bootproto=dhcp --device=link --activate --onboot=yes

# Bootloader
bootloader --location=mbr --timeout=3

# Disk layout: wipe and auto-partition
zerombr
clearpart --all --initlabel
autopart --type=plain --nohome

# SELinux: permissive for development
selinux --permissive

# Firewall: off for development
firewall --disabled

# No graphical installer — fully automated
text
skipx
reboot

#──────────────────────────────────────────────────────
# PACKAGE SELECTION
#──────────────────────────────────────────────────────

%packages --excludedocs --nobase --instLangs=en

# === Absolute minimum system ===
@core --nodefaults
kernel
systemd
dnf
sudo
NetworkManager

# === Wayland / Display ===
wayland-protocols
mesa-dri-drivers
mesa-vulkan-drivers
xorg-x11-server-Xwayland          # XWayland fallback for some GTK apps
wlroots
seatd

# === GUI Toolkit ===
gtk4
libadwaita
pango
cairo
gdk-pixbuf2
librsvg2

# === Fonts (critical for beautiful rendering) ===
google-noto-sans-fonts
google-noto-sans-mono-fonts
google-noto-emoji-fonts
fontconfig
freetype

# === Audio (PipeWire) ===
pipewire
pipewire-pulseaudio
wireplumber

# === Database ===
sqlite

# === Networking tools ===
curl
wget

# === Development (needed for self-improvement in Phase 2) ===
# git
# gcc
# rust
# cargo

# === Remove things we don't need ===
-plymouth                          # We'll add our own boot splash
-dracut-config-rescue
-firewalld
-sssd*
-abrt*
-PackageKit*
-dnf-plugins-core
-man-db
-man-pages
-vim-minimal

%end

#──────────────────────────────────────────────────────
# POST-INSTALL SCRIPT
#──────────────────────────────────────────────────────

%post --log=/root/ks-post.log

echo ">>> Levsha OS post-install starting..."

# ── Auto-login: no display manager, no login screen ──
# We use a minimal Wayland session that launches the Chat Shell directly

mkdir -p /etc/systemd/system/getty@tty1.service.d
cat > /etc/systemd/system/getty@tty1.service.d/autologin.conf << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin levsha --noclear %I $TERM
EOF

# ── Wayland Session: launch Chat Shell on login ──
mkdir -p /home/levsha/.config

cat > /home/levsha/.bash_profile << 'BASHEOF'
# If on tty1 and not already in a Wayland session, start the Chat Shell
if [ "$(tty)" = "/dev/tty1" ] && [ -z "$WAYLAND_DISPLAY" ]; then
    # Start a minimal Wayland compositor that runs only the Chat Shell
    exec levsha-session
fi
BASHEOF

chown levsha:levsha /home/levsha/.bash_profile

# ── Levsha OS Configuration ──
# Canonical config path: /etc/levsha/config.toml (TOML format)
mkdir -p /etc/levsha
cat > /etc/levsha/config.toml << 'CONFEOF'
[api]
key = "sk-ant-REPLACE_WITH_ACTUAL_KEY"
model = "claude-sonnet-4-20250514"
base_url = "https://api.anthropic.com"
timeout_seconds = 30
max_retries = 3

[context]
max_tokens = 200000
response_reserve = 4096
chars_per_token = 4

[execution]
command_timeout_seconds = 60

[skills]
path = "/usr/share/levsha/skills"

[persistence]
db_path = "/var/lib/levsha/history.db"
CONFEOF

# ── Install Levsha Chat Shell binary ──
# (In real build, this copies the pre-built binary from the build artifacts)
# cp /path/to/levsha-chat /usr/bin/levsha-chat
# cp /path/to/levsha-session /usr/bin/levsha-session
# chmod +x /usr/bin/levsha-chat /usr/bin/levsha-session

# ── Install built-in skills ──
mkdir -p /usr/share/levsha/skills/package-manager
mkdir -p /usr/share/levsha/skills/sysinfo
# cp -r /path/to/skills/package-manager/* /usr/share/levsha/skills/package-manager/
# cp -r /path/to/skills/sysinfo/* /usr/share/levsha/skills/sysinfo/

# ── Create data directories ──
mkdir -p /home/levsha/.local/share/levsha
chown -R levsha:levsha /home/levsha/.local

# ── systemd service for Chat Shell watchdog ──
cat > /etc/systemd/system/levsha-chat-watchdog.service << 'SVCEOF'
[Unit]
Description=Levsha Chat Shell Watchdog
After=graphical.target

[Service]
Type=simple
User=levsha
ExecStart=/usr/bin/levsha-chat --watchdog
Restart=always
RestartSec=2

[Install]
WantedBy=graphical.target
SVCEOF

# ── Sudoers: allow levsha to run dnf without password (for package skill) ──
echo "levsha ALL=(ALL) NOPASSWD: /usr/bin/dnf" > /etc/sudoers.d/levsha-dnf
chmod 440 /etc/sudoers.d/levsha-dnf

# ── Font configuration for beautiful rendering ──
cat > /etc/fonts/local.conf << 'FONTEOF'
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
  <!-- Enable subpixel rendering -->
  <match target="font">
    <edit name="rgba" mode="assign"><const>rgb</const></edit>
    <edit name="hinting" mode="assign"><bool>true</bool></edit>
    <edit name="hintstyle" mode="assign"><const>hintslight</const></edit>
    <edit name="antialias" mode="assign"><bool>true</bool></edit>
    <edit name="lcdfilter" mode="assign"><const>lcddefault</const></edit>
  </match>

  <!-- Default sans-serif: Noto Sans -->
  <alias>
    <family>sans-serif</family>
    <prefer><family>Noto Sans</family></prefer>
  </alias>

  <!-- Default monospace: Noto Sans Mono -->
  <alias>
    <family>monospace</family>
    <prefer><family>Noto Sans Mono</family></prefer>
  </alias>
</fontconfig>
FONTEOF

# ── Disable unnecessary services ──
systemctl disable bluetooth.service 2>/dev/null || true
systemctl disable cups.service 2>/dev/null || true
systemctl disable avahi-daemon.service 2>/dev/null || true
systemctl disable ModemManager.service 2>/dev/null || true

# ── Set default target ──
systemctl set-default multi-user.target

echo ">>> Levsha OS post-install complete."

%end

#──────────────────────────────────────────────────────
# POST-INSTALL: NOCHROOT (for ISO-specific tweaks)
#──────────────────────────────────────────────────────

%post --nochroot

echo ">>> Copying branding assets..."
# cp branding/logo.png $INSTALL_ROOT/usr/share/levsha/logo.png

%end
```

---

## Building the ISO

### Option 1: livemedia-creator (recommended)

This uses lorax's `livemedia-creator` which runs an actual Anaconda install inside a VM to produce the ISO:

```bash
# Validate the kickstart file first
ksvalidator kickstart/levsha-os.ks

# Build the ISO
sudo livemedia-creator \
  --ks kickstart/levsha-os.ks \
  --no-virt \
  --resultdir /var/tmp/levsha-build \
  --project "Levsha OS" \
  --releasever 41 \
  --volid "LevshaOS" \
  --iso-only \
  --iso-name Levsha-OS-0.1.iso \
  --title "Levsha OS" \
  --macboot
```

The `--no-virt` flag builds directly in the host (faster, but requires root and a clean environment). Remove it to build inside a VM (safer, slower).

### Option 2: livecd-creator (simpler, faster)

```bash
# SELinux must be permissive for livecd-creator
sudo setenforce 0

sudo livecd-creator \
  --config=kickstart/levsha-os.ks \
  --fslabel=LevshaOS \
  --cache=/var/cache/live \
  --verbose
```

This produces a live ISO that boots without installation. Simpler but less control than `livemedia-creator`.

---

## Build Automation

### build.sh

```bash
#!/bin/bash
set -euo pipefail

# ── Configuration ──
VERSION="0.1.0"
ISO_NAME="Levsha-OS-${VERSION}.iso"
KICKSTART="kickstart/levsha-os.ks"
BUILD_DIR="/var/tmp/levsha-build"
FEDORA_RELEASE="41"

echo "═══════════════════════════════════════════"
echo "  Levsha OS Build — v${VERSION}"
echo "═══════════════════════════════════════════"

# ── Step 1: Build the Chat Shell ──
echo ">>> Building Chat Shell..."
cd chat-shell
cargo build --release
cd ..

# ── Step 2: Build the Intelligence Engine ──
echo ">>> Building Intelligence Engine..."
cd intelligence-engine
cargo build --release
cd ..

# ── Step 3: Stage build artifacts for kickstart ──
echo ">>> Staging artifacts..."
mkdir -p staging/bin staging/skills staging/branding

cp chat-shell/target/release/levsha-chat staging/bin/
cp chat-shell/target/release/levsha-session staging/bin/
cp intelligence-engine/target/release/levsha-engine staging/bin/
cp -r skills/package-manager staging/skills/
cp -r skills/sysinfo staging/skills/
cp -r branding/* staging/branding/

# ── Step 4: Validate kickstart ──
echo ">>> Validating kickstart..."
ksvalidator "${KICKSTART}"

# ── Step 5: Build the ISO ──
echo ">>> Building ISO..."
sudo rm -rf "${BUILD_DIR}"

sudo livemedia-creator \
  --ks "${KICKSTART}" \
  --no-virt \
  --resultdir "${BUILD_DIR}" \
  --project "Levsha OS" \
  --releasever "${FEDORA_RELEASE}" \
  --volid "LevshaOS" \
  --iso-only \
  --iso-name "${ISO_NAME}" \
  --title "Levsha OS"

echo ">>> ISO built: ${BUILD_DIR}/${ISO_NAME}"

# ── Step 6: Calculate checksum ──
cd "${BUILD_DIR}"
sha256sum "${ISO_NAME}" > "${ISO_NAME}.sha256"
echo ">>> Checksum: $(cat ${ISO_NAME}.sha256)"

echo ""
echo "═══════════════════════════════════════════"
echo "  Build complete."
echo "  ISO: ${BUILD_DIR}/${ISO_NAME}"
echo "  Size: $(du -h ${ISO_NAME} | cut -f1)"
echo "═══════════════════════════════════════════"
```

Make it executable:

```bash
chmod +x build.sh
```

---

## Testing the ISO

### VirtualBox

```bash
# Create a VM
VBoxManage createvm --name "LevshaOS" --ostype "Fedora_64" --register
VBoxManage modifyvm "LevshaOS" --memory 2048 --cpus 2 --graphicscontroller vmsvga --vram 128
VBoxManage createhd --filename "LevshaOS.vdi" --size 8192
VBoxManage storagectl "LevshaOS" --name "SATA" --add sata
VBoxManage storageattach "LevshaOS" --storagectl "SATA" --port 0 --device 0 --type hdd --medium "LevshaOS.vdi"
VBoxManage storageattach "LevshaOS" --storagectl "SATA" --port 1 --device 0 --type dvddrive --medium /var/tmp/levsha-build/Levsha-OS-0.1.iso

# Start the VM
VBoxManage startvm "LevshaOS"
```

### QEMU/KVM

```bash
# Quick test boot (no persistent disk)
qemu-system-x86_64 \
  -cdrom /var/tmp/levsha-build/Levsha-OS-0.1.iso \
  -m 2048 \
  -smp 2 \
  -enable-kvm \
  -display gtk \
  -device virtio-vga \
  -nic user,model=virtio-net-pci

# With persistent disk
qemu-img create -f qcow2 levsha-disk.qcow2 8G

qemu-system-x86_64 \
  -cdrom /var/tmp/levsha-build/Levsha-OS-0.1.iso \
  -drive file=levsha-disk.qcow2,format=qcow2 \
  -m 2048 \
  -smp 2 \
  -enable-kvm \
  -display gtk \
  -device virtio-vga \
  -nic user,model=virtio-net-pci \
  -boot d
```

---

## Boot Flow (What Happens on Startup)

```
BIOS/UEFI
   │
   ▼
GRUB (3 second timeout)
   │
   ▼
Linux Kernel loads
   │
   ▼
systemd starts
   │
   ├── NetworkManager  →  DHCP on eth0  →  connected
   ├── PipeWire        →  audio ready
   └── getty@tty1      →  auto-login as "levsha" user
                              │
                              ▼
                       .bash_profile runs
                              │
                              ▼
                       levsha-session starts
                       (minimal Wayland compositor)
                              │
                              ▼
                       levsha-chat launches
                       (full-screen Chat Shell GUI)
                              │
                              ▼
                       Welcome message appears.
                       User starts typing.
```

Total time target: **< 30 seconds** from power-on to chat.

---

## Customizing the Kickstart

### Adding packages

Add to the `%packages` section:

```kickstart
# I need ffmpeg in the base image
ffmpeg
```

### Changing the API key

Edit the `%post` section where `config.toml` is written:

```kickstart
api_key = sk-ant-YOUR_ACTUAL_KEY_HERE
```

### Adding files to the ISO

Use `%post --nochroot` to copy files from the build host into the image:

```kickstart
%post --nochroot
cp /path/on/host/my-file.txt $INSTALL_ROOT/etc/levsha/my-file.txt
%end
```

### Flattening includes

If you split the kickstart into multiple files with `%include`, flatten them before building:

```bash
ksflatten -c kickstart/levsha-os.ks -o kickstart/levsha-os-flat.ks
```

---

## Troubleshooting

### Build fails with "no space"

`livemedia-creator` needs ~10GB free in `/var/tmp`. Clean up or point to a bigger disk:

```bash
sudo livemedia-creator --tmp /mnt/bigdisk/tmp ...
```

### SELinux blocks the build

```bash
sudo setenforce 0
# Build...
sudo setenforce 1
```

### Kickstart validation errors

```bash
# Check for syntax errors
ksvalidator kickstart/levsha-os.ks

# See the fully resolved kickstart (with all includes expanded)
ksflatten -c kickstart/levsha-os.ks -o /tmp/flat.ks
cat /tmp/flat.ks
```

### VM boots to a blank screen

Likely missing GPU drivers or Wayland compositor. Check:
- `mesa-dri-drivers` is in `%packages`
- The Wayland compositor (`levsha-session`) binary is actually installed
- Try adding `--device virtio-vga` to QEMU or using VMSVGA in VirtualBox

### Chat Shell doesn't start

Check the post-boot logs:

```bash
# From a serial console or by adding a debug TTY
journalctl -u levsha-chat-watchdog
journalctl --user -u levsha-session
cat /root/ks-post.log
```

---

## References

- Fedora Kickstart Docs: https://docs.fedoraproject.org/en-US/fedora/latest/install-guide/appendixes/Kickstart_Syntax_Reference/
- Fedora Spin Kickstarts: https://pagure.io/fedora-kickstarts
- livemedia-creator: https://weldr.io/lorax/livemedia-creator.html
- livecd-creator: https://github.com/livecd-tools/livecd-tools
- Fedora Custom OS Image: https://docs.fedoraproject.org/en-US/remix-building/

---

*This guide is part of the Levsha OS project. See Levsha_OS_PRD_v3.md for the full product requirements.*
