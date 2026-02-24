# 09 — ISO Build: Design

> **Visual styling follows [theme.design.md](../theme.design.md).**

**Module:** ISO Generation and VM Testing
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

The Levsha OS ISO is built using Fedora's standard tooling: a kickstart file defines the package selection and system configuration, and `livemedia-creator` (backed by `lorax`) produces the bootable ISO. A filesystem overlay injects Levsha OS-specific files (binaries, configs, themes) on top of the Fedora base.

---

## 2. Build Pipeline Architecture

```
Kickstart file (base/kickstart/levsha-os.ks)
       |
       v
livemedia-creator
  |-- Installs Fedora minimal packages
  |-- Applies kickstart %post scripts
  |-- Injects filesystem overlay
  |-- Builds initramfs with Plymouth theme
  |-- Configures GRUB bootloader
  |
  v
levsha-os-x86_64.iso
  |
  v
Test in QEMU (automated)
```

---

## 3. Kickstart File Design

The kickstart file (`base/kickstart/levsha-os.ks`) is the central build definition.

### 3.1 Installation Source

```
# Fedora base repository (version pinned)
url --mirrorlist=https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-41&arch=x86_64
repo --name=updates --mirrorlist=https://mirrors.fedoraproject.org/mirrorlist?repo=updates-released-f41&arch=x86_64
```

### 3.2 System Configuration

```
# Language and keyboard
lang en_US.UTF-8
keyboard us
timezone UTC --utc

# Single user, no root password
user --name=levsha --groups=wheel --homedir=/home/levsha
rootpw --lock

# Partitioning (simple, single partition)
clearpart --all --initlabel
autopart --type=plain --nohome

# Bootloader
bootloader --timeout=0 --append="quiet splash plymouth.enable=1 vt.global_cursor_default=0 rd.udev.log_level=3 systemd.show_status=false"

# Network (DHCP, auto-activate)
network --bootproto=dhcp --device=link --activate --onboot=yes

# Services
services --enabled=NetworkManager,levsha-engine
services --disabled=sshd,bluetooth,cups,firewalld

# SELinux permissive
selinux --permissive

# Reboot after install
reboot
```

### 3.3 Package Selection

```
%packages
@core
cage                           # Wayland kiosk compositor
mesa-dri-drivers               # GPU drivers for VM (virtio-gpu, QXL)
mesa-vulkan-drivers
xorg-x11-drv-qxl              # QXL driver for VirtualBox/SPICE
spice-vdagent                  # SPICE guest agent (clipboard, resize)
plymouth                       # Boot splash
plymouth-scripts
NetworkManager                 # Network management
pipewire                       # Audio
pipewire-pulseaudio
google-noto-sans-fonts         # Proportional font
google-noto-sans-mono-fonts    # Monospace font
sqlite                         # Database runtime
vim-minimal                    # Emergency editing (not exposed to user)
sudo

# Exclude unnecessary packages
-iwl*-firmware                 # No Wi-Fi firmware needed in VM
-aic94xx-firmware
-alsa-firmware
-ivtv-firmware
%end
```

### 3.4 Post-Install Script

```
%post --log=/root/ks-post.log

# Passwordless sudo for levsha
echo "levsha ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/levsha
chmod 0440 /etc/sudoers.d/levsha

# Auto-login on tty1
mkdir -p /etc/systemd/system/getty@tty1.service.d/
cat > /etc/systemd/system/getty@tty1.service.d/autologin.conf << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin levsha --noclear %I $TERM
Type=idle
EOF

# Mask emergency/rescue/debug shells
systemctl mask emergency.service
systemctl mask rescue.service
systemctl mask debug-shell.service

# Disable all virtual consoles except tty1
for i in 2 3 4 5 6; do
    systemctl mask getty@tty${i}.service
done

# Create Levsha directories
mkdir -p /var/lib/levsha
chown levsha:levsha /var/lib/levsha
chmod 0700 /var/lib/levsha

mkdir -p /etc/levsha
mkdir -p /run/levsha

# Plymouth theme
plymouth-set-default-theme levsha

# Rebuild initramfs with Plymouth theme
dracut -f --no-hostonly

%end
```

---

## 4. Filesystem Overlay Structure

The overlay directory mirrors the root filesystem. Files are copied on top of the installed system during the build.

```
base/overlay/
  etc/
    levsha/
      config.toml                    # API key, model config, paths
  usr/
    bin/
      levsha-chat-shell              # Chat Shell binary
      levsha-engine                  # Intelligence Engine binary
    share/
      plymouth/themes/levsha/
        levsha.plymouth              # Plymouth theme descriptor
        levsha.script                # Plymouth animation script
        logo.png                     # Boot splash logo
      levsha/
        skills/
          package-manager.yaml       # Built-in skill manifests
          system-info.yaml
  home/
    levsha/
      .bash_profile                  # Compositor auto-start script
  lib/
    systemd/system/
      levsha-engine.service          # Engine systemd unit
```

The overlay is applied during the `%post` section of the kickstart or via `livemedia-creator`'s `--extra-boot-args` and file injection mechanisms.

---

## 5. Package Selection Rationale

### Included — required for function

| Package | Reason |
|---------|--------|
| `@core` | Minimal Fedora base (systemd, glibc, bash, coreutils) |
| `cage` | Wayland kiosk compositor for single-app mode |
| `mesa-dri-drivers` | OpenGL drivers for VirtIO-GPU, QXL |
| `NetworkManager` | DHCP network management |
| `plymouth` | Boot splash |
| `pipewire` + `pipewire-pulseaudio` | Audio subsystem |
| `google-noto-sans-fonts` | Proportional font for chat text |
| `google-noto-sans-mono-fonts` | Monospace font for code blocks |
| `sqlite` | Runtime library for persistence |
| `sudo` | Passwordless root access for levsha user |

### Excluded — size reduction

| Package/Pattern | Reason |
|-----------------|--------|
| `iwl*-firmware` | Wi-Fi firmware, not needed in VM |
| `aic94xx-firmware` | SCSI firmware, not needed |
| `alsa-firmware` | ALSA firmware, PipeWire handles audio |
| All `*-firmware` except VM-relevant | Reduces ISO size significantly |

---

## 6. VM Guest Drivers

### VirtIO (QEMU/KVM)

- `virtio-blk` / `virtio-scsi` — disk I/O (included in kernel)
- `virtio-net` — network (included in kernel)
- `virtio-gpu` — display (included in mesa-dri-drivers)

### VirtualBox

- `mesa-dri-drivers` provides basic display support
- `spice-vdagent` provides dynamic display resize and clipboard sharing
- VirtualBox Guest Additions can be installed post-boot via the package manager skill if needed

### Display resolution

The compositor (cage) inherits the display resolution from the virtual GPU. In VirtualBox, this is set in VM settings. In QEMU, it is set via `-display` flags. The Chat Shell adapts to any resolution at or above 1280x720.

---

## 7. Size Budget

| Component | Estimated Size |
|-----------|---------------|
| Fedora @core | ~400 MB |
| Wayland/Mesa stack | ~150 MB |
| Fonts | ~50 MB |
| Levsha binaries (shell + engine) | ~30 MB |
| Plymouth theme + assets | ~2 MB |
| Skill definitions | < 1 MB |
| PipeWire | ~20 MB |
| NetworkManager | ~30 MB |
| Overhead (metadata, squashfs) | ~100 MB |
| **Total ISO** | **~800 MB** |

Target: under 1.5 GB. Estimated actual: around 800 MB, well within budget.

---

## 8. Diagram: ISO Build Flow

```
Developer machine / CI
  |
  v
+----------------------------+
| livemedia-creator          |
|  --ks=levsha-os.ks         |
|  --make-iso                |
|  --iso-name=levsha-os      |
+-------------+--------------+
              |
    +---------+---------+
    |                   |
    v                   v
 Fedora repos      Overlay files
 (packages)        (binaries, configs)
    |                   |
    +---------+---------+
              |
              v
    levsha-os-x86_64.iso
              |
              v
    QEMU automated test
```
