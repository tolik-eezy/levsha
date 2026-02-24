#!/bin/bash
set -euo pipefail

# ── Configuration ──
VERSION="0.1.0"
ISO_NAME="Levsha-OS-${VERSION}.iso"
KICKSTART="base/kickstart/levsha-os.ks"
BUILD_DIR="/var/tmp/levsha-build"
STAGING_DIR="staging"
FEDORA_RELEASE="41"
CONTAINER_NAME="levsha-builder"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}"

# ── Container runtime detection (podman preferred, fallback to docker) ──
if command -v podman &>/dev/null; then
    CONTAINER_RT="podman"
elif command -v docker &>/dev/null; then
    CONTAINER_RT="docker"
else
    echo "ERROR: Neither podman nor docker found. Install one of them."
    exit 1
fi

# ── Platform flags for cross-architecture builds (ARM Mac → x86_64 container) ──
UNAME_M="$(uname -m)"
PLATFORM_FLAG=""
if [ "${UNAME_M}" = "arm64" ] || [ "${UNAME_M}" = "aarch64" ]; then
    PLATFORM_FLAG="--platform linux/amd64"
fi

echo "═══════════════════════════════════════════"
echo "  Levsha OS Build — v${VERSION}"
echo "  Runtime: ${CONTAINER_RT} ${PLATFORM_FLAG}"
echo "═══════════════════════════════════════════"
echo ""

# ── Step 1: Build Rust workspace inside Fedora container ──
echo ">>> Step 1: Building Rust workspace in container..."
${CONTAINER_RT} run --rm ${PLATFORM_FLAG} \
    -v "${PROJECT_ROOT}:/build:z" \
    "${CONTAINER_NAME}" \
    cargo build --release --workspace

echo ">>> Rust build complete."
echo ""

# ── Step 2: Extract binaries from container build ──
echo ">>> Step 2: Verifying build artifacts..."
BINARY="target/release/levsha-chat"
if [ ! -f "${BINARY}" ]; then
    echo "ERROR: Binary not found: ${BINARY}"
    echo "       The container build should have produced it via the volume mount."
    exit 1
fi
echo "    Found: ${BINARY} ($(du -h "${BINARY}" | cut -f1))"
echo ""

# ── Step 3: Stage artifacts into staging/ directory ──
echo ">>> Step 3: Staging artifacts..."
rm -rf "${STAGING_DIR}"

# Binary
mkdir -p "${STAGING_DIR}/usr/bin"
cp "${BINARY}" "${STAGING_DIR}/usr/bin/levsha-chat"
chmod +x "${STAGING_DIR}/usr/bin/levsha-chat"

# Skills
mkdir -p "${STAGING_DIR}/usr/share/levsha/skills"
if [ -d "skills/built-in" ] && [ "$(ls -A skills/built-in 2>/dev/null)" ]; then
    cp -r skills/built-in/* "${STAGING_DIR}/usr/share/levsha/skills/"
fi

# Configuration
mkdir -p "${STAGING_DIR}/etc/levsha"
if [ -f "base/overlay/etc/levsha/config.toml" ]; then
    cp "base/overlay/etc/levsha/config.toml" "${STAGING_DIR}/etc/levsha/config.toml"
fi

# All overlay files (configs, systemd units, fonts, home skeleton, etc.)
if [ -d "base/overlay" ] && [ "$(ls -A base/overlay 2>/dev/null)" ]; then
    cp -a base/overlay/* "${STAGING_DIR}/"
fi

echo "    Staged files:"
find "${STAGING_DIR}" -type f | sort | while read -r f; do
    echo "      ${f}"
done
echo ""

# ── Step 4: Validate kickstart ──
echo ">>> Step 4: Validating kickstart..."
if [ ! -f "${KICKSTART}" ]; then
    echo "WARNING: Kickstart file not found: ${KICKSTART}"
    echo "         Skipping validation. Create it before the ISO build."
else
    ${CONTAINER_RT} run --rm ${PLATFORM_FLAG} \
        -v "${PROJECT_ROOT}:/build:z" \
        "${CONTAINER_NAME}" \
        ksvalidator "/build/${KICKSTART}"
    echo "    Kickstart is valid."
fi
echo ""

# ── Step 5: Build ISO with livemedia-creator ──
echo ">>> Step 5: Building ISO..."
if [ ! -f "${KICKSTART}" ]; then
    echo "ERROR: Cannot build ISO without kickstart file: ${KICKSTART}"
    exit 1
fi

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

echo "    ISO built: ${BUILD_DIR}/${ISO_NAME}"
echo ""

# ── Step 6: Calculate SHA256 checksum ──
echo ">>> Step 6: Calculating checksum..."
sha256sum "${BUILD_DIR}/${ISO_NAME}" > "${BUILD_DIR}/${ISO_NAME}.sha256"
echo "    $(cat "${BUILD_DIR}/${ISO_NAME}.sha256")"
echo ""

# ── Done ──
ISO_SIZE="$(du -h "${BUILD_DIR}/${ISO_NAME}" | cut -f1)"

echo "═══════════════════════════════════════════"
echo "  Build complete."
echo "  ISO:      ${BUILD_DIR}/${ISO_NAME}"
echo "  Size:     ${ISO_SIZE}"
echo "  Checksum: ${BUILD_DIR}/${ISO_NAME}.sha256"
echo "═══════════════════════════════════════════"
