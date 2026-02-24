# Levsha OS — Build System
#
# Three build targets:
#   make fast     — Build + deploy to running VM via SSH (~8s)
#                   or boot existing image + deploy (~25s)
#   make dev      — Full dev rebuild: packages + artifacts + boot QEMU
#                   First run is slow (~15 min), subsequent runs use cached base (~30s)
#   make release  — Release build + ISO generation
#
# Other targets:
#   make boot     — Just boot the existing qcow2 (no rebuild)
#   make clean    — Remove build artifacts (keeps cached base)
#   make clean-all — Remove everything including cached base

.PHONY: fast dev release boot clean clean-all toolchain

# ── Configuration ──
VERSION           := 0.1.0
ISO_NAME          := Levsha-OS-$(VERSION).iso
BUILD_DIR         := /var/tmp/levsha-build
STAGING_DIR       := staging
CONTAINER         := levsha-builder
CONTAINER_ISO     := levsha-iso-tools
CONTAINER_RELEASE := levsha-release
BINARY_OUT        := .build-out/levsha-chat
TEST_QCOW2       := levsha-test.qcow2
CACHED_BASE       := levsha-base-cached.qcow2
SSH_PORT          := 2222
SSH_USER          := levsha
SSH_PASS          := levsha

# ── Container runtime detection ──
CONTAINER_RT := $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null)
ifeq ($(CONTAINER_RT),)
  $(error Neither podman nor docker found. Install one of them.)
endif

# ── Architecture detection ──
UNAME_M := $(shell uname -m)
UNAME_S := $(shell uname -s)

ifeq ($(UNAME_M),arm64)
  QEMU_ARCH    := aarch64
  QCOW2_URL    := https://download.fedoraproject.org/pub/fedora/linux/releases/41/Cloud/aarch64/images/Fedora-Cloud-Base-Generic-41-1.4.aarch64.qcow2
  QCOW2_FILE   := vendor/fedora/Fedora-Cloud-Base-41.aarch64.qcow2
  PLATFORM_FLAG :=
else
  QEMU_ARCH    := x86_64
  QCOW2_URL    := https://download.fedoraproject.org/pub/fedora/linux/releases/41/Cloud/x86_64/images/Fedora-Cloud-Base-Generic-41-1.4.x86_64.qcow2
  QCOW2_FILE   := vendor/fedora/Fedora-Cloud-Base-41.x86_64.qcow2
  PLATFORM_FLAG :=
endif

# ── Fedora Server netinstall ISO (for release ISO build) ──
ifeq ($(QEMU_ARCH),aarch64)
  FEDORA_ISO_URL := https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/aarch64/iso/Fedora-Server-netinst-aarch64-41-1.4.iso
  FEDORA_ISO     := vendor/fedora/Fedora-Server-netinst-aarch64-41-1.4.iso
else
  FEDORA_ISO_URL := https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/iso/Fedora-Server-netinst-x86_64-41-1.4.iso
  FEDORA_ISO     := vendor/fedora/Fedora-Server-netinst-x86_64-41-1.4.iso
endif

# ── QEMU config ──
ifeq ($(QEMU_ARCH),aarch64)
  QEMU_BIN     := qemu-system-aarch64
  QEMU_MACHINE := -machine virt -cpu host
  QEMU_ACCEL   := -accel hvf
  QEMU_BIOS    := -bios /opt/homebrew/share/qemu/edk2-aarch64-code.fd
  QEMU_DISPLAY := -display cocoa
  QEMU_DRIVE   := -drive file=$(TEST_QCOW2),format=qcow2,if=none,id=hd0 -device virtio-blk-pci,drive=hd0
  QEMU_NET     := -device virtio-net-pci,netdev=net0 -netdev user,id=net0,hostfwd=tcp::$(SSH_PORT)-:22
  QEMU_EXTRA   := -device virtio-gpu-pci -device usb-ehci -device usb-kbd -device usb-tablet -serial mon:stdio
else
  QEMU_BIN     := qemu-system-x86_64
  QEMU_MACHINE :=
  ifeq ($(UNAME_S),Darwin)
    QEMU_ACCEL := -accel tcg,thread=multi
  else
    QEMU_ACCEL := -enable-kvm
  endif
  QEMU_BIOS    :=
  QEMU_DISPLAY := -display cocoa
  QEMU_DRIVE   := -drive file=$(TEST_QCOW2),format=qcow2,if=virtio
  QEMU_NET     := -nic user,model=virtio-net-pci,hostfwd=tcp::$(SSH_PORT)-:22
  QEMU_EXTRA   := -device virtio-vga -usb -device usb-kbd -device usb-tablet
endif

SSH_CMD := sshpass -p '$(SSH_PASS)' ssh -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=3 -p $(SSH_PORT) $(SSH_USER)@localhost
SCP_CMD := sshpass -p '$(SSH_PASS)' scp -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -P $(SSH_PORT)

# ══════════════════════════════════════════════════════════════════════
# make fast — Build + deploy to running VM via SSH (~8s)
#   Requires VM already running (use 'make boot' in another terminal)
# ══════════════════════════════════════════════════════════════════════
fast: _build-debug _stage
	@if $(SSH_CMD) "true" 2>/dev/null; then \
		echo ">>> VM is running. Deploying via SSH..."; \
	else \
		echo "ERROR: VM is not running. Start it with 'make boot' first."; \
		exit 1; \
	fi
	@echo ">>> Uploading binary..."
	@$(SCP_CMD) $(STAGING_DIR)/usr/bin/levsha-chat $(SSH_USER)@localhost:/tmp/levsha-chat
	@echo ">>> Uploading skills..."
	@$(SCP_CMD) -r $(STAGING_DIR)/usr/share/levsha/skills $(SSH_USER)@localhost:/tmp/levsha-skills
	@echo ">>> Syncing source tree..."
	@tar czf /tmp/levsha-src.tar.gz --exclude=target --exclude=vendor --exclude=.git \
		--exclude='*.qcow2' --exclude=staging --exclude=.build-out \
		--exclude='levsha-*.qcow2' \
		engine chat-shell skills base infra self-improve-staging assets Cargo.toml Cargo.lock Makefile CLAUDE.md 2>/dev/null || true
	@$(SCP_CMD) /tmp/levsha-src.tar.gz $(SSH_USER)@localhost:/tmp/levsha-src.tar.gz
	@$(SSH_CMD) "\
		echo '$(SSH_PASS)' | sudo -S bash -c '\
		cp /tmp/levsha-chat /usr/bin/levsha-chat && \
		chmod 755 /usr/bin/levsha-chat && \
		rm -rf /usr/share/levsha/skills && \
		cp -r /tmp/levsha-skills /usr/share/levsha/skills && \
		mkdir -p /usr/src/levsha && \
		cd /usr/src/levsha && tar xzf /tmp/levsha-src.tar.gz 2>/dev/null && \
		chown -R levsha:levsha /usr/src/levsha && \
		if [ ! -d /usr/src/levsha/.git ]; then \
			cd /usr/src/levsha && \
			git init -q && \
			git config user.name Levsha && \
			git config user.email levsha@localhost && \
			git add -A && git commit -q -m \"source snapshot\"; \
		else \
			cd /usr/src/levsha && \
			git add -A && git diff --cached --quiet || git commit -q -m \"source update\"; \
		fi && \
		pkill levsha-chat || true \
		' && rm -rf /tmp/levsha-chat /tmp/levsha-skills /tmp/levsha-src.tar.gz"
	@rm -f /tmp/levsha-src.tar.gz
	@echo ">>> Deploy complete. Chat shell restarting."

# ══════════════════════════════════════════════════════════════════════
# make dev — Full dev rebuild with cached base (~30s after first run)
# ══════════════════════════════════════════════════════════════════════
dev: _build-debug _stage _ensure-base _inject-fast boot

_ensure-base: _ensure-iso-tools
	@if [ -f "$(CACHED_BASE)" ]; then \
		echo ">>> Using cached base image."; \
	else \
		echo ">>> No cached base found. Building (this takes ~15 min, one-time only)..."; \
		$(MAKE) _build-base; \
	fi

_build-base: $(QCOW2_FILE) _ensure-iso-tools
	cp $(QCOW2_FILE) $(CACHED_BASE)
	$(CONTAINER_RT) run --rm $(PLATFORM_FLAG) \
		--privileged \
		-v "$$(pwd):/work:z" \
		-e QCOW2_FILE=/work/$(CACHED_BASE) \
		-e STAGING_DIR=/work/$(STAGING_DIR) \
		-e SKIP_SELINUX_RELABEL=1 \
		$(CONTAINER_ISO) \
		/work/infra/inject-artifacts.sh
	@echo ">>> Cached base image ready: $(CACHED_BASE)"

_inject-fast: _ensure-iso-tools
	@echo ">>> Creating test image from cached base..."
	cp $(CACHED_BASE) $(TEST_QCOW2)
	@echo ">>> Injecting artifacts only (no packages)..."
	$(CONTAINER_RT) run --rm $(PLATFORM_FLAG) \
		--privileged \
		-v "$$(pwd):/work:z" \
		-e QCOW2_FILE=/work/$(TEST_QCOW2) \
		-e STAGING_DIR=/work/$(STAGING_DIR) \
		-e SKIP_PACKAGES=1 \
		-e SKIP_SELINUX_RELABEL=1 \
		$(CONTAINER_ISO) \
		/work/infra/inject-artifacts.sh
	@echo ">>> Fast injection complete."

# ══════════════════════════════════════════════════════════════════════
# make release — Release build + installable ISO
#   Downloads Fedora Server netinstall ISO (~700 MB, cached),
#   embeds kickstart + staged artifacts via mkksiso,
#   produces a self-installing ISO that auto-configures Levsha OS.
#   Target machine needs internet during installation (Fedora packages).
# ══════════════════════════════════════════════════════════════════════
release: _build-release _stage-release _build-iso
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo "  Levsha OS — Release ISO ready"
	@echo "  ISO:  .build-out/$(ISO_NAME)"
	@echo "  SHA:  .build-out/$(ISO_NAME).sha256"
	@echo ""
	@echo "  Boot this ISO to auto-install Levsha OS."
	@echo "  The target machine needs internet access"
	@echo "  (packages are downloaded from Fedora repos)."
	@echo "═══════════════════════════════════════════"

_build-iso: _ensure-release-tools $(FEDORA_ISO)
	@echo ">>> Creating staging tarball..."
	@tar czf staging.tar.gz -C $(STAGING_DIR) .
	@echo ">>> Building ISO with mkksiso..."
	@mkdir -p .build-out
	$(CONTAINER_RT) run --rm $(PLATFORM_FLAG) \
		-v "$$(pwd):/build:z" \
		$(CONTAINER_RELEASE) \
		mkksiso --ks /build/base/kickstart/levsha-os.ks \
			-a /build/staging.tar.gz \
			/build/$(FEDORA_ISO) \
			/build/.build-out/$(ISO_NAME)
	@rm -f staging.tar.gz
	@echo ">>> ISO built: .build-out/$(ISO_NAME)"
	@if command -v sha256sum >/dev/null 2>&1; then \
		sha256sum .build-out/$(ISO_NAME) > .build-out/$(ISO_NAME).sha256; \
	else \
		shasum -a 256 .build-out/$(ISO_NAME) > .build-out/$(ISO_NAME).sha256; \
	fi
	@echo ">>> Checksum: $$(cat .build-out/$(ISO_NAME).sha256)"

_ensure-release-tools:
	@$(CONTAINER_RT) build $(PLATFORM_FLAG) -t $(CONTAINER_RELEASE) -f infra/Containerfile.release . -q >/dev/null

$(FEDORA_ISO):
	@echo ">>> Downloading Fedora Server netinstall ISO ($(QEMU_ARCH))..."
	@echo "    This is a one-time download (~700 MB), cached in vendor/fedora/"
	@mkdir -p vendor/fedora
	curl -L --fail --progress-bar -o $(FEDORA_ISO) $(FEDORA_ISO_URL)
	@echo ">>> Download complete."

# ══════════════════════════════════════════════════════════════════════
# make boot — Just boot existing qcow2
# ══════════════════════════════════════════════════════════════════════
boot:
	@if [ ! -f "$(TEST_QCOW2)" ]; then \
		echo "ERROR: $(TEST_QCOW2) not found. Run 'make dev' first."; \
		exit 1; \
	fi
	@echo ">>> Booting $(TEST_QCOW2)..."
	$(QEMU_BIN) \
		$(QEMU_MACHINE) $(QEMU_ACCEL) $(QEMU_BIOS) $(QEMU_DRIVE) \
		-m 4096 -smp 4 \
		$(QEMU_DISPLAY) $(QEMU_NET) $(QEMU_EXTRA)

# ══════════════════════════════════════════════════════════════════════
# Internal: build and stage targets
# ══════════════════════════════════════════════════════════════════════

toolchain:
	@echo ">>> Building build container..."
	@$(CONTAINER_RT) build $(PLATFORM_FLAG) -t $(CONTAINER) -f infra/Containerfile . -q >/dev/null
	@echo ">>> Container ready."

_build-debug: toolchain
	@echo ">>> Building (debug)..."
	@mkdir -p .build-out
	@$(CONTAINER_RT) run --rm $(PLATFORM_FLAG) \
		-v "$$(pwd):/build:z" \
		-v levsha-cargo-cache:/build/target:z \
		-v levsha-cargo-registry:/root/.cargo/registry:z \
		$(CONTAINER) \
		sh -c 'cargo build --workspace 2>&1 | tail -5 && cp target/debug/levsha-chat /build/.build-out/levsha-chat'
	@echo ">>> Build complete."

_build-release: toolchain
	@echo ">>> Building (release)..."
	@mkdir -p .build-out
	@$(CONTAINER_RT) run --rm $(PLATFORM_FLAG) \
		-v "$$(pwd):/build:z" \
		-v levsha-cargo-cache-rel:/build/target:z \
		-v levsha-cargo-registry:/root/.cargo/registry:z \
		$(CONTAINER) \
		sh -c 'cargo build --release --workspace 2>&1 | tail -5 && cp target/release/levsha-chat /build/.build-out/levsha-chat'
	@echo ">>> Release build complete."

_stage:
	@echo ">>> Staging (debug)..."
	@rm -rf $(STAGING_DIR)
	@mkdir -p $(STAGING_DIR)/usr/bin $(STAGING_DIR)/usr/share/levsha/skills
	@cp $(BINARY_OUT) $(STAGING_DIR)/usr/bin/levsha-chat
	@chmod +x $(STAGING_DIR)/usr/bin/levsha-chat
	@if [ -d "skills/built-in" ] && [ "$$(ls -A skills/built-in 2>/dev/null)" ]; then \
		cp -r skills/built-in/* $(STAGING_DIR)/usr/share/levsha/skills/; \
	fi
	@if [ -d "base/overlay" ] && [ "$$(ls -A base/overlay 2>/dev/null)" ]; then \
		cp -a base/overlay/* $(STAGING_DIR)/; \
	fi

_stage-release:
	@echo ">>> Staging (release)..."
	@rm -rf $(STAGING_DIR)
	@mkdir -p $(STAGING_DIR)/usr/bin $(STAGING_DIR)/usr/share/levsha/skills
	@cp $(BINARY_OUT) $(STAGING_DIR)/usr/bin/levsha-chat
	@chmod +x $(STAGING_DIR)/usr/bin/levsha-chat
	@if [ -d "skills/built-in" ] && [ "$$(ls -A skills/built-in 2>/dev/null)" ]; then \
		cp -r skills/built-in/* $(STAGING_DIR)/usr/share/levsha/skills/; \
	fi
	@if [ -d "base/overlay" ] && [ "$$(ls -A base/overlay 2>/dev/null)" ]; then \
		cp -a base/overlay/* $(STAGING_DIR)/; \
	fi

# ══════════════════════════════════════════════════════════════════════
# Internal: infrastructure
# ══════════════════════════════════════════════════════════════════════

_ensure-iso-tools:
	@$(CONTAINER_RT) build $(PLATFORM_FLAG) -t $(CONTAINER_ISO) -f infra/Containerfile.iso . -q >/dev/null

$(QCOW2_FILE):
	@echo ">>> Downloading Fedora Cloud base image ($(QEMU_ARCH))..."
	@mkdir -p vendor/fedora
	curl -L --fail -o $(QCOW2_FILE) $(QCOW2_URL)
	@if file $(QCOW2_FILE) | grep -q "QEMU\|QCOW"; then \
		echo ">>> Download complete."; \
	else \
		echo "ERROR: Downloaded file is not a valid qcow2 image."; \
		rm -f $(QCOW2_FILE); \
		exit 1; \
	fi

# ══════════════════════════════════════════════════════════════════════
# Clean
# ══════════════════════════════════════════════════════════════════════

clean:
	@echo ">>> Cleaning build artifacts (keeping cached base and Fedora ISO)..."
	rm -rf $(STAGING_DIR)
	rm -f $(TEST_QCOW2)
	rm -f staging.tar.gz
	rm -f .build-out/$(ISO_NAME) .build-out/$(ISO_NAME).sha256
	@echo ">>> Clean. Cached base and Fedora ISO preserved."

clean-all: clean
	@echo ">>> Removing cached base, Fedora ISO, and target..."
	rm -f $(CACHED_BASE)
	rm -f $(FEDORA_ISO)
	rm -rf target
	@echo ">>> Full clean complete."
