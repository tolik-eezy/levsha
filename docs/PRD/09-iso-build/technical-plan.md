# 09 — ISO Build: Technical Plan

**Module:** ISO Generation and VM Testing
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

This document details the step-by-step implementation plan for building the Levsha OS ISO, from setting up the build environment to automated testing and CI/CD integration. The build uses Fedora's `livemedia-creator` with a kickstart file and a filesystem overlay containing all Levsha OS components.

---

## 2. Build Environment Setup

### 2.1 Fedora Host (Recommended)

The build must run on a Fedora system (matching the target version) because `livemedia-creator` uses `dnf` and `anaconda` internals.

```bash
# Install build tools
sudo dnf install -y \
    lorax \
    livemedia-creator \
    anaconda \
    pykickstart \
    virt-install \
    libvirt \
    qemu-kvm

# Enable libvirt for livemedia-creator's VM-based builds
sudo systemctl enable --now libvirtd
```

### 2.2 Container Build (CI)

For CI environments without a Fedora host, use a privileged Fedora container:

```dockerfile
# infra/Dockerfile.build
FROM fedora:41

RUN dnf install -y \
    lorax \
    livemedia-creator \
    anaconda \
    pykickstart \
    && dnf clean all

WORKDIR /build
```

```bash
docker run --privileged -v $(pwd):/build fedora-builder \
    livemedia-creator --ks=/build/base/kickstart/levsha-os.ks \
    --make-iso --iso-name=levsha-os
```

The `--privileged` flag is required because `livemedia-creator` uses loop devices and mount operations.

---

## 3. Kickstart File Creation

The kickstart file lives at `base/kickstart/levsha-os.ks`. See the design document for the full kickstart content. Key implementation notes:

### Validation

```bash
# Validate kickstart syntax before building
ksvalidator base/kickstart/levsha-os.ks
```

### Overlay injection

The filesystem overlay is injected in the `%post` section:

```
%post --nochroot --log=/mnt/sysimage/root/ks-overlay.log
# Copy overlay files into the installed system
cp -a /run/install/repo/overlay/* /mnt/sysimage/
%end
```

Alternatively, use `livemedia-creator`'s `--extra-boot-args` and custom `lorax` templates for cleaner injection.

---

## 4. livemedia-creator Commands

### Direct ISO build (requires root)

```bash
sudo livemedia-creator \
    --ks=base/kickstart/levsha-os.ks \
    --make-iso \
    --iso-name=levsha-os-x86_64.iso \
    --iso-only \
    --project="Levsha OS" \
    --releasever=41 \
    --resultdir=build/iso/ \
    --tmp=/var/tmp/levsha-build \
    --logfile=build/logs/livemedia.log
```

### Using virt-install backend (safer, no root FS manipulation)

```bash
sudo livemedia-creator \
    --ks=base/kickstart/levsha-os.ks \
    --make-iso \
    --iso-name=levsha-os-x86_64.iso \
    --iso-only \
    --project="Levsha OS" \
    --releasever=41 \
    --resultdir=build/iso/ \
    --tmp=/var/tmp/levsha-build \
    --virt-uefi \
    --ram=4096 \
    --vcpus=4 \
    --logfile=build/logs/livemedia.log
```

### Build script

```bash
#!/bin/bash
# infra/scripts/build-iso.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BUILD_DIR="$PROJECT_ROOT/build"
KS_FILE="$PROJECT_ROOT/base/kickstart/levsha-os.ks"

# Validate kickstart
ksvalidator "$KS_FILE"

# Compile Levsha OS binaries
(cd "$PROJECT_ROOT/chat-shell" && cargo build --release)
(cd "$PROJECT_ROOT/engine" && cargo build --release)

# Copy binaries to overlay
cp "$PROJECT_ROOT/chat-shell/target/release/levsha-chat-shell" \
    "$PROJECT_ROOT/base/overlay/usr/bin/"
cp "$PROJECT_ROOT/engine/target/release/levsha-engine" \
    "$PROJECT_ROOT/base/overlay/usr/bin/"

# Clean previous build
rm -rf "$BUILD_DIR/iso"
mkdir -p "$BUILD_DIR/iso" "$BUILD_DIR/logs"

# Build ISO
sudo livemedia-creator \
    --ks="$KS_FILE" \
    --make-iso \
    --iso-name=levsha-os-x86_64.iso \
    --iso-only \
    --project="Levsha OS" \
    --releasever=41 \
    --resultdir="$BUILD_DIR/iso/" \
    --tmp=/var/tmp/levsha-build \
    --logfile="$BUILD_DIR/logs/livemedia.log"

echo "ISO built: $BUILD_DIR/iso/levsha-os-x86_64.iso"
ls -lh "$BUILD_DIR/iso/levsha-os-x86_64.iso"
```

---

## 5. ISO Testing Automation (QEMU)

### Quick boot test

```bash
#!/bin/bash
# infra/scripts/test-boot-qemu.sh
set -euo pipefail

ISO="${1:-build/iso/levsha-os-x86_64.iso}"
TIMEOUT=45
DISK_IMG="/tmp/levsha-test-disk.qcow2"

# Create a temporary disk for the live system
qemu-img create -f qcow2 "$DISK_IMG" 8G

echo "Booting ISO: $ISO"
echo "Timeout: ${TIMEOUT}s"

# Start QEMU with serial console for monitoring
timeout "$TIMEOUT" qemu-system-x86_64 \
    -m 2048 \
    -smp 2 \
    -cdrom "$ISO" \
    -drive file="$DISK_IMG",format=qcow2,if=virtio \
    -boot d \
    -display none \
    -serial mon:stdio \
    -device virtio-net-pci,netdev=net0 \
    -netdev user,id=net0 \
    -no-reboot \
    2>&1 | tee /tmp/levsha-boot.log &

QEMU_PID=$!

# Monitor for boot success markers in serial output
START=$(date +%s)
BOOTED=false

while kill -0 $QEMU_PID 2>/dev/null; do
    ELAPSED=$(($(date +%s) - START))
    if [ $ELAPSED -ge $TIMEOUT ]; then
        echo "FAIL: Boot timeout after ${TIMEOUT}s"
        kill $QEMU_PID 2>/dev/null
        exit 1
    fi
    if grep -q "levsha-engine.*ready" /tmp/levsha-boot.log 2>/dev/null; then
        BOOTED=true
        echo "PASS: System booted in ${ELAPSED}s"
        kill $QEMU_PID 2>/dev/null
        break
    fi
    sleep 1
done

# Cleanup
rm -f "$DISK_IMG"

if [ "$BOOTED" = true ]; then
    echo "Boot test PASSED"
    exit 0
else
    echo "Boot test FAILED"
    exit 1
fi
```

### Interactive test (with display)

```bash
# infra/scripts/run-vm.sh — for manual testing
qemu-system-x86_64 \
    -m 2048 \
    -smp 2 \
    -cdrom build/iso/levsha-os-x86_64.iso \
    -drive file=/tmp/levsha-disk.qcow2,format=qcow2,if=virtio \
    -boot d \
    -display gtk,gl=on \
    -device virtio-net-pci,netdev=net0 \
    -netdev user,id=net0 \
    -device virtio-vga-gl \
    -usb -device usb-tablet
```

---

## 6. CI/CD Pipeline

### GitHub Actions workflow

```yaml
# .github/workflows/build-iso.yml
name: Build Levsha OS ISO

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    container:
      image: fedora:41
      options: --privileged

    steps:
      - uses: actions/checkout@v4

      - name: Install build tools
        run: |
          dnf install -y lorax livemedia-creator pykickstart \
              rust cargo gcc make

      - name: Build Levsha binaries
        run: |
          cd chat-shell && cargo build --release
          cd ../engine && cargo build --release

      - name: Copy binaries to overlay
        run: |
          cp chat-shell/target/release/levsha-chat-shell base/overlay/usr/bin/
          cp engine/target/release/levsha-engine base/overlay/usr/bin/

      - name: Validate kickstart
        run: ksvalidator base/kickstart/levsha-os.ks

      - name: Build ISO
        run: bash infra/scripts/build-iso.sh

      - name: Upload ISO artifact
        uses: actions/upload-artifact@v4
        with:
          name: levsha-os-iso
          path: build/iso/levsha-os-x86_64.iso
```

Note: Actual QEMU boot testing in CI requires nested virtualization support, which is not available on all CI runners. The boot test is run in a separate job on self-hosted runners with KVM access.

---

## 7. Size Budget and Monitoring

### Tracking ISO size

The build script logs the ISO size. CI fails if it exceeds the budget:

```bash
ISO_SIZE=$(stat -c%s build/iso/levsha-os-x86_64.iso)
MAX_SIZE=$((1500 * 1024 * 1024))  # 1.5 GB

if [ "$ISO_SIZE" -gt "$MAX_SIZE" ]; then
    echo "FAIL: ISO size $(numfmt --to=iec $ISO_SIZE) exceeds budget (1.5 GB)"
    exit 1
fi
echo "ISO size: $(numfmt --to=iec $ISO_SIZE) (budget: 1.5 GB)"
```

### Package audit

After building, extract the package list from the ISO for review:

```bash
# List all RPMs included in the ISO
rpm -qa --root=/mnt/levsha-iso-mount | sort > build/logs/package-list.txt
```

This list is committed to the repo and reviewed in PRs to catch unexpected package additions.

---

## 8. Testing Checklist

| # | Test | Method | Pass Criteria |
|---|------|--------|--------------|
| 1 | ISO builds successfully | CI | Exit code 0, ISO file exists |
| 2 | ISO size within budget | CI | < 1.5 GB |
| 3 | Kickstart validates | CI | `ksvalidator` exits 0 |
| 4 | QEMU boot | Automated script | Boot to Chat Shell in < 30s |
| 5 | VirtualBox boot | Manual | Boot to Chat Shell in < 30s |
| 6 | Display 1280x720 | Manual | Chat Shell renders correctly |
| 7 | Display 1920x1080 | Manual | Chat Shell renders correctly |
| 8 | Network connectivity | Automated | `curl api.anthropic.com` succeeds |
| 9 | Offline boot | Manual | Chat Shell appears with "disconnected" status |
| 10 | End-to-end message | Manual | User sends message, receives Claude response |
| 11 | Memory usage | Automated | `free -m` < 1024 MB at idle |
| 12 | Package list audit | CI | No unexpected packages |
| 13 | No TTY escape | Manual | Ctrl+Alt+F2 does not switch to console |
| 14 | Reboot persistence | Manual | Conversation history survives reboot |
| 15 | Welcome message | Manual | First boot shows welcome text |

### Cross-Module Integration Tests

The ISO is the ultimate integration test — it bundles all modules. See `00-system-architecture/integration-checks.md` for full details.

| Check | Seam | What it verifies |
|-------|------|------------------|
| IC-90 | Full stack | First boot: Plymouth -> Chat Shell -> welcome message -> status bar "connected" |
| IC-91 | L3 -> L2 -> API -> L3 | Simple conversation: user types, streaming response appears |
| IC-92 | L3 -> L2 -> API -> L2 -> L1 -> L2 -> API -> L3 | System query: disk usage tool call executed, result rendered |
| IC-93 | Full stack + L1 | Package install: dnf executed, binary installed on base system |
| IC-94 | L2 -> L3 -> L2 -> L1 | Destructive confirmation: prompt rendered, approval executes command |
| IC-95 | Full stack + reboot | Conversation history persists across VM reboot |
| IC-96 | Network -> L2 -> L3 | Error recovery: disconnect -> error shown -> reconnect -> conversation resumes |

These checks supersede the ISO's own smoke tests — every IC-90+ check must pass on the final ISO before release.

---

## 9. Implementation Order

1. **Build environment** — Install lorax/livemedia-creator on Fedora dev machine or set up container.
2. **Kickstart file** — Write and validate `levsha-os.ks` with Fedora minimal base.
3. **Filesystem overlay** — Create directory structure, populate with placeholder binaries and configs.
4. **First ISO build** — Build with placeholders; verify it boots to a Fedora console.
5. **Auto-login and compositor** — Add getty override and cage; verify boots to empty Wayland session.
6. **Levsha binaries** — Replace placeholders with real Chat Shell and Engine binaries.
7. **Plymouth theme** — Add theme files, rebuild initramfs, verify splash.
8. **QEMU test script** — Automate boot testing.
9. **Size audit** — Verify ISO size, trim unnecessary packages.
10. **CI pipeline** — Set up GitHub Actions for automated builds.
11. **Full acceptance testing** — Run through complete checklist.
