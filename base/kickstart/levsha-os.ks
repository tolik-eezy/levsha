# Levsha OS — Production Kickstart
# Fedora-based, minimal, chat-only operating system
# Phase 1 MVP

#──────────────────────────────────────────────────────
# SYSTEM CONFIGURATION
#──────────────────────────────────────────────────────

# Language and locale
lang en_US.UTF-8
keyboard us
timezone UTC --utc

# Root password (development only)
rootpw --plaintext levsha

# Create the levsha user with auto-login, in the wheel group
user --name=levsha --password=levsha --plaintext --groups=wheel

# Network: auto-configure via DHCP (ethernet in VM)
network --bootproto=dhcp --device=link --activate --onboot=yes

# Bootloader: GRUB timeout=0 for instant boot (BF-03)
bootloader --location=mbr --timeout=0 --append="quiet splash plymouth.enable=1 rd.lvm=0 rd.luks=0 rd.md=0 rd.dm=0"

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

%packages --excludedocs --instLangs=en

# === Absolute minimum system ===
@core --nodefaults
kernel
systemd
dnf
sudo
NetworkManager

# === Wayland compositor (kiosk mode) ===
cage
wayland-protocols
mesa-dri-drivers
mesa-vulkan-drivers
seatd

# === GUI Toolkit ===
gtk4
libadwaita
pango
cairo
gdk-pixbuf2
librsvg2

# === Fonts (critical for beautiful rendering) ===
# IBM Plex Sans/Mono are the brand typefaces (theme.design.md)
# Noto Sans/Mono are fallbacks
ibm-plex-sans-fonts
ibm-plex-mono-fonts
google-noto-sans-fonts
google-noto-sans-mono-fonts
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

# === VirtIO / QXL guest drivers (VM testing) ===
qemu-guest-agent
spice-vdagent

# === Boot splash ===
plymouth
plymouth-scripts

# === Remove things we don't need ===
-dracut-config-rescue
-firewalld
-sssd-common
-sssd-client
-sssd-kcm
-abrt
-abrt-libs
-abrt-addon-ccpp
-abrt-addon-python
-abrt-cli
-PackageKit
-PackageKit-glib
-man-db
-man-pages
-vim-minimal

%end

#──────────────────────────────────────────────────────
# POST-INSTALL SCRIPT
#──────────────────────────────────────────────────────

%post --log=/root/ks-post.log

echo ">>> Levsha OS post-install starting..."

# ── Copy overlay files from staging area ──
# The build system stages overlay/ into /tmp/levsha-overlay before kickstart runs.
# If the staging dir exists, copy everything into place.
if [ -d /tmp/levsha-overlay ]; then
    cp -a /tmp/levsha-overlay/* /
    echo ">>> Overlay files copied from staging."
else
    echo ">>> No staging overlay found, creating files inline..."

    # ── Auto-login: getty@tty1 autologin drop-in ──
    mkdir -p /etc/systemd/system/getty@tty1.service.d
    cat > /etc/systemd/system/getty@tty1.service.d/autologin.conf << 'EOF'
[Service]
ExecStart=
ExecStart=-/sbin/agetty --autologin levsha --noclear %I $TERM
EOF

    # ── Levsha OS Configuration ──
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

    # ── Wayland session launcher ──
    cat > /usr/bin/levsha-session << 'SESSEOF'
#!/bin/sh
exec cage -- /usr/bin/levsha-chat
SESSEOF
    chmod +x /usr/bin/levsha-session

    # ── Bash profile: launch compositor on tty1 ──
    cat > /home/levsha/.bash_profile << 'BASHEOF'
# Levsha OS — launch Wayland session on tty1
if [ "$(tty)" = "/dev/tty1" ] && [ -z "$WAYLAND_DISPLAY" ]; then
    exec levsha-session
fi
BASHEOF
    chown levsha:levsha /home/levsha/.bash_profile

    # ── systemd service for Chat Shell watchdog ──
    cat > /etc/systemd/system/levsha-chat-watchdog.service << 'SVCEOF'
[Unit]
Description=Levsha Chat Shell Watchdog
After=multi-user.target

[Service]
Type=simple
User=levsha
ExecStart=/usr/bin/levsha-chat
Restart=always
RestartSec=2
Environment=WAYLAND_DISPLAY=wayland-0
Environment=XDG_RUNTIME_DIR=/run/user/1000

[Install]
WantedBy=multi-user.target
SVCEOF

    # ── Sudoers: allow levsha to run dnf without password ──
    echo "levsha ALL=(ALL) NOPASSWD: /usr/bin/dnf" > /etc/sudoers.d/levsha-dnf
    chmod 440 /etc/sudoers.d/levsha-dnf

    # ── Font configuration ──
    mkdir -p /etc/fonts
    cat > /etc/fonts/local.conf << 'FONTEOF'
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
  <!-- Enable subpixel rendering (theme.design.md 3.3) -->
  <match target="font">
    <edit name="rgba" mode="assign"><const>rgb</const></edit>
    <edit name="hinting" mode="assign"><bool>true</bool></edit>
    <edit name="hintstyle" mode="assign"><const>hintslight</const></edit>
    <edit name="antialias" mode="assign"><bool>true</bool></edit>
    <edit name="lcdfilter" mode="assign"><const>lcddefault</const></edit>
  </match>

  <!-- Default sans-serif: IBM Plex Sans (brand typeface), fallback Noto Sans -->
  <alias>
    <family>sans-serif</family>
    <prefer>
      <family>IBM Plex Sans</family>
      <family>Noto Sans</family>
    </prefer>
  </alias>

  <!-- Default monospace: IBM Plex Mono, fallback Noto Sans Mono -->
  <alias>
    <family>monospace</family>
    <prefer>
      <family>IBM Plex Mono</family>
      <family>Noto Sans Mono</family>
    </prefer>
  </alias>
</fontconfig>
FONTEOF

fi

# ── Configure Plymouth boot splash ──
plymouth-set-default-theme levsha
dracut -f --regenerate-all

# ── Create data directories ──
mkdir -p /var/lib/levsha
chown levsha:levsha /var/lib/levsha
mkdir -p /usr/share/levsha/skills/package-manager
mkdir -p /usr/share/levsha/skills/sysinfo
mkdir -p /home/levsha/.local/share/levsha
chown -R levsha:levsha /home/levsha/.local

# ── Enable required services ──
systemctl enable NetworkManager.service
systemctl enable qemu-guest-agent.service 2>/dev/null || true
systemctl enable seatd.service 2>/dev/null || true

# ── Disable unnecessary gettys (VT switching lockdown) ──
# Only tty1 is needed for auto-login. Disable tty2-tty6 to prevent
# VT switching escape from the kiosk compositor. (BS Acceptance Criteria #6)
systemctl disable getty@tty2.service 2>/dev/null || true
systemctl disable getty@tty3.service 2>/dev/null || true
systemctl disable getty@tty4.service 2>/dev/null || true
systemctl disable getty@tty5.service 2>/dev/null || true
systemctl disable getty@tty6.service 2>/dev/null || true
systemctl mask getty@tty2.service
systemctl mask getty@tty3.service
systemctl mask getty@tty4.service
systemctl mask getty@tty5.service
systemctl mask getty@tty6.service

# ── Disable unnecessary services ──
systemctl disable bluetooth.service 2>/dev/null || true
systemctl disable cups.service 2>/dev/null || true
systemctl disable avahi-daemon.service 2>/dev/null || true
systemctl disable ModemManager.service 2>/dev/null || true

# ── Set default target (no graphical target — we use cage directly) ──
systemctl set-default multi-user.target

# ── Ensure correct ownership ──
chown levsha:levsha /home/levsha/.bash_profile 2>/dev/null || true

echo ">>> Levsha OS post-install complete."

%end

#──────────────────────────────────────────────────────
# POST-INSTALL: NOCHROOT (for ISO build artifacts)
#──────────────────────────────────────────────────────

%post --nochroot

echo ">>> Staging overlay and build artifacts..."

# Method 1: ISO-based install (mkksiso embeds staging.tar.gz in the ISO)
# During Anaconda installation, the ISO is mounted at /run/install/isodir/
for isodir in /run/install/isodir /mnt/install/isodir; do
    if [ -f "${isodir}/staging.tar.gz" ]; then
        echo ">>> Found staging tarball at ${isodir}/staging.tar.gz"
        tar xzf "${isodir}/staging.tar.gz" -C $INSTALL_ROOT/
        echo ">>> Staged artifacts extracted from ISO."
        exit 0
    fi
done

# Method 2: Direct build (livemedia-creator or local build)
# The build system stages all artifacts into staging/ using a proper
# filesystem hierarchy:
#   staging/usr/bin/levsha-chat
#   staging/usr/share/levsha/skills/...
#   staging/etc/levsha/config.toml
#   staging/etc/systemd/...  (from overlay)
#   staging/home/...         (from overlay)
for d in staging /build/staging; do
    if [ -d "$d" ] && [ "$(ls -A "$d" 2>/dev/null)" ]; then
        cp -a "$d"/* $INSTALL_ROOT/
        echo ">>> Staged artifacts copied from $d"
        exit 0
    fi
done

echo ">>> WARNING: No staging tarball or directory found. Binary and skills not installed."

%end
