#!/bin/bash
set -euo pipefail

# ── Levsha OS — Artifact Injection Script ──
# Injects compiled binary, skills, overlay, system config into a Fedora Cloud
# qcow2 image using virt-customize. Optionally installs packages.
#
# Environment variables (set by Makefile):
#   QCOW2_FILE      — path to the qcow2 image to modify
#   STAGING_DIR      — path to the staging directory with artifacts
#   SKIP_PACKAGES    — set to 1 to skip package installation (fast mode)
#   SKIP_SELINUX_RELABEL — set to 1 to skip SELinux relabel
#
# This script runs INSIDE the levsha-iso-tools container (privileged).

QCOW2_FILE="${QCOW2_FILE:?QCOW2_FILE must be set}"
STAGING_DIR="${STAGING_DIR:?STAGING_DIR must be set}"

echo "═══════════════════════════════════════════"
echo "  Levsha OS — Artifact Injection"
echo "  Image:   ${QCOW2_FILE}"
echo "  Staging: ${STAGING_DIR}"
echo "  Packages: $([ "${SKIP_PACKAGES:-0}" = "1" ] && echo "SKIP (cached)" || echo "install")"
echo "  SELinux:  $([ "${SKIP_SELINUX_RELABEL:-0}" = "1" ] && echo "SKIP" || echo "relabel")"
echo "═══════════════════════════════════════════"
echo ""

# Verify inputs
if [ ! -f "${QCOW2_FILE}" ]; then
    echo "ERROR: qcow2 image not found: ${QCOW2_FILE}"
    exit 1
fi
if [ ! -d "${STAGING_DIR}" ]; then
    echo "ERROR: staging directory not found: ${STAGING_DIR}"
    exit 1
fi
if [ ! -f "${STAGING_DIR}/usr/bin/levsha-chat" ]; then
    echo "ERROR: levsha-chat binary not found in staging. Run 'make stage' first."
    exit 1
fi

# ── Resize the image to have room for packages ──
echo ">>> Expanding image to 8GB..."
qemu-img resize "${QCOW2_FILE}" 8G

# ── Build virt-customize command ──
CUSTOMIZE_ARGS=()

# 1. Pre-install all required packages (skip if cached base)
if [ "${SKIP_PACKAGES:-0}" != "1" ]; then
    echo ">>> Preparing package installation..."
    CUSTOMIZE_ARGS+=(
        --install cage,mesa-dri-drivers,seatd
        --install gtk4,libadwaita,pango,cairo,gdk-pixbuf2,librsvg2
        --install ibm-plex-sans-fonts,ibm-plex-mono-fonts,google-noto-sans-fonts,google-noto-sans-mono-fonts,fontconfig
        --install pipewire,pipewire-pulseaudio,wireplumber
        --install sqlite,curl,tree,qemu-guest-agent,spice-vdagent,sudo
        --install nodejs,npm,git,gcc,make,rust,cargo
        --run-command "npm install -g @anthropic-ai/claude-code"
    )
else
    echo ">>> Skipping packages (using cached base)"
fi

# 2. Copy binary
echo ">>> Preparing binary injection..."
CUSTOMIZE_ARGS+=(
    --upload "${STAGING_DIR}/usr/bin/levsha-chat:/usr/bin/levsha-chat"
    --chmod 0755:/usr/bin/levsha-chat
)

# 3. Copy skills
echo ">>> Preparing skills injection..."
CUSTOMIZE_ARGS+=(--mkdir /usr/share/levsha)
if [ -d "${STAGING_DIR}/usr/share/levsha/skills" ]; then
    CUSTOMIZE_ARGS+=(--copy-in "${STAGING_DIR}/usr/share/levsha/skills:/usr/share/levsha")
fi

# 4. Copy source tree for self-improvement
echo ">>> Preparing source tree injection..."
if [ -d "${STAGING_DIR}/../engine" ]; then
    # Copy source directories needed for self-improvement
    for src_dir in engine chat-shell skills base infra self-improve-staging assets Cargo.toml Cargo.lock Makefile; do
        if [ -e "${STAGING_DIR}/../${src_dir}" ]; then
            CUSTOMIZE_ARGS+=(--mkdir /usr/src/levsha)
            CUSTOMIZE_ARGS+=(--copy-in "${STAGING_DIR}/../${src_dir}:/usr/src/levsha")
        fi
    done
    CUSTOMIZE_ARGS+=(--run-command "chown -R levsha:levsha /usr/src/levsha")
    # Initialize git repo for self-improve baseline
    CUSTOMIZE_ARGS+=(--run-command "cd /usr/src/levsha && git init -q && git config user.name Levsha && git config user.email levsha@localhost && git add -A && git commit -q -m 'source snapshot' 2>/dev/null || true")
fi

# 5. Copy overlay files (config, systemd units, fonts config, session launcher, etc.)
echo ">>> Preparing overlay injection..."
OVERLAY_FILES=(
    "etc/levsha/config.toml"
    "etc/systemd/system/levsha-chat-watchdog.service"
    "etc/systemd/system/getty@tty1.service.d/autologin.conf"
    "etc/fonts/local.conf"
    "etc/sudoers.d/levsha-dnf"
    "home/levsha/.bash_profile"
    "usr/bin/levsha-session"
)

for rel in "${OVERLAY_FILES[@]}"; do
    src="${STAGING_DIR}/${rel}"
    if [ -f "${src}" ]; then
        dir="/$(dirname "${rel}")"
        CUSTOMIZE_ARGS+=(--mkdir "${dir}")
        CUSTOMIZE_ARGS+=(--upload "${src}:/${rel}")
    fi
done

# Make levsha-session executable
CUSTOMIZE_ARGS+=(--chmod 0755:/usr/bin/levsha-session)

# 5. Disable cloud-init (interferes with our user setup)
echo ">>> Disabling cloud-init..."
CUSTOMIZE_ARGS+=(
    --run-command "systemctl disable cloud-init.service cloud-init-local.service cloud-config.service cloud-final.service 2>/dev/null || true"
    --run-command "systemctl mask cloud-init.service cloud-init-local.service cloud-config.service cloud-final.service 2>/dev/null || true"
    --run-command "touch /etc/cloud/cloud-init.disabled"
)

# 6. Create levsha user and configure system
echo ">>> Preparing user and system configuration..."
CUSTOMIZE_ARGS+=(
    --run-command "id levsha &>/dev/null || useradd -m -G wheel levsha"
    --run-command "echo 'levsha:levsha' | chpasswd"
    --run-command "mkdir -p /var/lib/levsha && chown levsha:levsha /var/lib/levsha"
    --run-command "mkdir -p /usr/share/levsha/skills/package-manager /usr/share/levsha/skills/sysinfo"
    --run-command "mkdir -p /home/levsha/.local/share/levsha"
    --run-command "chown -R levsha:levsha /home/levsha"
    --run-command "chmod 755 /home/levsha"
    --run-command "restorecon -R /home/levsha 2>/dev/null || true"
    --run-command "sed -i 's/^SELINUX=enforcing/SELINUX=permissive/' /etc/selinux/config"
    --run-command "chmod 440 /etc/sudoers.d/levsha-dnf 2>/dev/null || true"
    --run-command "sed -i 's/^PasswordAuthentication no/PasswordAuthentication yes/' /etc/ssh/sshd_config 2>/dev/null || true"
    --run-command "mkdir -p /etc/ssh/sshd_config.d && echo 'PasswordAuthentication yes' > /etc/ssh/sshd_config.d/50-levsha.conf"
)

# 7. Enable/disable services
CUSTOMIZE_ARGS+=(
    --run-command "systemctl enable NetworkManager.service"
    --run-command "systemctl enable sshd.service"
    --run-command "systemctl enable seatd.service 2>/dev/null || true"
    --run-command "systemctl enable qemu-guest-agent.service 2>/dev/null || true"
    --run-command "systemctl set-default multi-user.target"
    --run-command "systemctl disable levsha-chat-watchdog.service 2>/dev/null || true"
    --run-command "systemctl disable getty@tty2.service 2>/dev/null || true"
    --run-command "systemctl disable getty@tty3.service 2>/dev/null || true"
    --run-command "systemctl disable getty@tty4.service 2>/dev/null || true"
    --run-command "systemctl disable getty@tty5.service 2>/dev/null || true"
    --run-command "systemctl disable getty@tty6.service 2>/dev/null || true"
    --run-command "systemctl mask getty@tty2.service"
    --run-command "systemctl mask getty@tty3.service"
    --run-command "systemctl mask getty@tty4.service"
    --run-command "systemctl mask getty@tty5.service"
    --run-command "systemctl mask getty@tty6.service"
)

# 8. Set root password for dev access
CUSTOMIZE_ARGS+=(
    --root-password password:levsha
)

# ── Run virt-customize ──
echo ""
echo ">>> Running virt-customize (packages + files + config)..."
echo ""

if [ "${SKIP_SELINUX_RELABEL:-0}" = "1" ]; then
    echo ">>> Skipping SELinux relabel (dev mode)"
    virt-customize \
        -a "${QCOW2_FILE}" \
        "${CUSTOMIZE_ARGS[@]}" \
        --no-selinux-relabel
else
    virt-customize \
        -a "${QCOW2_FILE}" \
        "${CUSTOMIZE_ARGS[@]}" \
        --selinux-relabel
fi

echo ""
echo "═══════════════════════════════════════════"
echo "  Injection complete."
echo "  Image: ${QCOW2_FILE}"
echo ""
echo "  All packages pre-installed. No first-boot wait needed."
echo ""
echo "  Boot with:"
echo "    make test-quick-boot"
echo "═══════════════════════════════════════════"
