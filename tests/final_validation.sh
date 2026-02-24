#!/usr/bin/env bats

# Levsha OS — Final Validation Tests
# Comprehensive pre-release validation extending smoke_test.sh.
# Run with: bats tests/final_validation.sh
#
# Prerequisites:
#   - `make stage` has been run (populates staging/)
#   - Source tree is intact (for code quality checks)

STAGING="staging"
KICKSTART="base/kickstart/levsha-os.ks"
OVERLAY="base/overlay"
SKILLS_SRC="skills/built-in"

# ═══════════════════════════════════════════════════════
# BUILD ARTIFACT CHECKS
# ═══════════════════════════════════════════════════════

@test "levsha-chat binary exists and is executable" {
    [ -f "$STAGING/usr/bin/levsha-chat" ]
    [ -x "$STAGING/usr/bin/levsha-chat" ]
}

@test "levsha-chat binary is a valid ELF binary" {
    run file "$STAGING/usr/bin/levsha-chat"
    [[ "$output" == *"ELF"* ]]
}

@test "levsha-chat binary is under 50MB" {
    size=$(stat -f%z "$STAGING/usr/bin/levsha-chat" 2>/dev/null \
        || stat --printf="%s" "$STAGING/usr/bin/levsha-chat" 2>/dev/null)
    max=$((50 * 1024 * 1024))
    [ "$size" -lt "$max" ]
}

@test "config.toml exists in staging" {
    [ -f "$STAGING/etc/levsha/config.toml" ]
}

@test "config.toml has [api] section" {
    grep -q '^\[api\]' "$STAGING/etc/levsha/config.toml"
}

@test "config.toml has [context] section" {
    grep -q '^\[context\]' "$STAGING/etc/levsha/config.toml"
}

@test "config.toml has [execution] section" {
    grep -q '^\[execution\]' "$STAGING/etc/levsha/config.toml"
}

@test "config.toml has [skills] section" {
    grep -q '^\[skills\]' "$STAGING/etc/levsha/config.toml"
}

@test "config.toml has [persistence] section" {
    grep -q '^\[persistence\]' "$STAGING/etc/levsha/config.toml"
}

# ── Tool Definitions ──

@test "all 11 tool definitions are present" {
    count=$(find "$STAGING/usr/share/levsha/skills" -name '*.json' -type f | wc -l | tr -d ' ')
    [ "$count" -eq 11 ]
}

@test "package-manager has 5 tools" {
    count=$(find "$STAGING/usr/share/levsha/skills/package-manager/tools" -name '*.json' -type f | wc -l | tr -d ' ')
    [ "$count" -eq 5 ]
}

@test "sysinfo has 6 tools" {
    count=$(find "$STAGING/usr/share/levsha/skills/sysinfo/tools" -name '*.json' -type f | wc -l | tr -d ' ')
    [ "$count" -eq 6 ]
}

@test "tool JSON files are valid JSON" {
    for f in $(find "$STAGING/usr/share/levsha/skills" -name '*.json' -type f); do
        run python3 -m json.tool "$f"
        [ "$status" -eq 0 ]
    done
}

@test "skill manifests are valid YAML" {
    # Requires python3 with PyYAML or yq; fall back to basic syntax check
    for f in $(find "$SKILLS_SRC" -name 'skill.yaml' -type f); do
        if command -v python3 >/dev/null 2>&1; then
            run python3 -c "import yaml; yaml.safe_load(open('$f'))"
            [ "$status" -eq 0 ]
        else
            # Basic: file is non-empty and does not start with a tab
            [ -s "$f" ]
            ! head -1 "$f" | grep -qP '^\t'
        fi
    done
}

@test "skills total size is under 1MB" {
    if [ -d "$STAGING/usr/share/levsha/skills" ]; then
        size=$(du -sb "$STAGING/usr/share/levsha/skills" 2>/dev/null \
            || du -sk "$STAGING/usr/share/levsha/skills" | awk '{print $1 * 1024}')
        size=$(echo "$size" | head -1 | awk '{print $1}')
        max=$((1 * 1024 * 1024))
        [ "$size" -lt "$max" ]
    fi
}

# ── Plymouth Theme ──

@test "Plymouth theme descriptor exists" {
    [ -f "$STAGING/usr/share/plymouth/themes/levsha/levsha.plymouth" ]
}

@test "Plymouth script exists" {
    [ -f "$STAGING/usr/share/plymouth/themes/levsha/levsha.script" ]
}

@test "Plymouth descriptor references script module" {
    grep -q 'ModuleName=script' "$STAGING/usr/share/plymouth/themes/levsha/levsha.plymouth"
}

@test "Plymouth descriptor points to correct script path" {
    grep -q 'ScriptFile=.*/levsha\.script' "$STAGING/usr/share/plymouth/themes/levsha/levsha.plymouth"
}

# ── Systemd Services ──

@test "watchdog service exists in staging" {
    [ -f "$STAGING/etc/systemd/system/levsha-chat-watchdog.service" ] \
        || [ -f "$OVERLAY/etc/systemd/system/levsha-chat-watchdog.service" ]
}

@test "watchdog service has Restart=always" {
    svc="$OVERLAY/etc/systemd/system/levsha-chat-watchdog.service"
    grep -q 'Restart=always' "$svc"
}

@test "autologin drop-in exists" {
    [ -f "$STAGING/etc/systemd/system/getty@tty1.service.d/autologin.conf" ] \
        || [ -f "$OVERLAY/etc/systemd/system/getty@tty1.service.d/autologin.conf" ]
}

@test "autologin drop-in uses --autologin flag" {
    conf="$OVERLAY/etc/systemd/system/getty@tty1.service.d/autologin.conf"
    grep -q '\-\-autologin' "$conf"
}

# ── Session Launcher ──

@test "levsha-session script exists and is executable" {
    [ -f "$OVERLAY/usr/bin/levsha-session" ]
    [ -x "$OVERLAY/usr/bin/levsha-session" ]
}

@test "levsha-session launches cage" {
    grep -q 'cage' "$OVERLAY/usr/bin/levsha-session"
}

# ═══════════════════════════════════════════════════════
# KICKSTART VALIDATION
# ═══════════════════════════════════════════════════════

@test "kickstart file exists" {
    [ -f "$KICKSTART" ]
}

@test "kickstart includes cage (Wayland kiosk)" {
    grep -q '^cage$' "$KICKSTART"
}

@test "kickstart includes gtk4" {
    grep -q '^gtk4$' "$KICKSTART"
}

@test "kickstart includes libadwaita" {
    grep -q '^libadwaita$' "$KICKSTART"
}

@test "kickstart includes plymouth" {
    grep -q '^plymouth$' "$KICKSTART"
}

@test "kickstart includes sqlite" {
    grep -q '^sqlite$' "$KICKSTART"
}

@test "kickstart includes NetworkManager" {
    grep -q '^NetworkManager$' "$KICKSTART"
}

@test "kickstart includes IBM Plex fonts" {
    grep -q 'ibm-plex-sans-fonts' "$KICKSTART"
    grep -q 'ibm-plex-mono-fonts' "$KICKSTART"
}

@test "kickstart GRUB timeout is 0" {
    grep -q '\-\-timeout=0' "$KICKSTART"
}

@test "kickstart has autologin user" {
    grep -q 'user.*--name=levsha' "$KICKSTART"
}

@test "kickstart disables VT switching (masks getty tty2-6)" {
    grep -q 'mask getty@tty2' "$KICKSTART"
    grep -q 'mask getty@tty3' "$KICKSTART"
    grep -q 'mask getty@tty4' "$KICKSTART"
    grep -q 'mask getty@tty5' "$KICKSTART"
    grep -q 'mask getty@tty6' "$KICKSTART"
}

@test "kickstart enables quiet splash boot" {
    grep -q 'quiet splash' "$KICKSTART"
}

@test "kickstart sets Plymouth theme to levsha" {
    grep -q 'plymouth-set-default-theme levsha' "$KICKSTART"
}

# ═══════════════════════════════════════════════════════
# CODE QUALITY CHECKS
# ═══════════════════════════════════════════════════════

@test "no hardcoded API keys in Rust source" {
    # Scan Rust source for anything that looks like sk-ant-api keys
    ! grep -r 'sk-ant-api' chat-shell/src/ engine/src/ 2>/dev/null
}

@test "no hardcoded API keys in skill definitions" {
    ! grep -r 'sk-ant-api' skills/ 2>/dev/null
}

@test "overlay config uses placeholder key, not real key" {
    # The overlay config should use a placeholder, not a real API key
    # It's acceptable to have sk-ant-REPLACE or similar placeholder
    if grep -q 'sk-ant-api' "$OVERLAY/etc/levsha/config.toml" 2>/dev/null; then
        # Real key pattern: sk-ant-api followed by alphanumeric chars
        ! grep -qP 'sk-ant-api[0-9]{2}-[A-Za-z0-9]{3}' "$OVERLAY/etc/levsha/config.toml"
    fi
}

@test "all engine .rs files have module-level docs" {
    for f in engine/src/*.rs; do
        [ -f "$f" ] || continue
        # Check for //! doc comment in first 10 lines
        head -10 "$f" | grep -q '//!' || {
            echo "Missing //! module doc in: $f"
            return 1
        }
    done
}

@test "all chat-shell .rs files have module-level docs" {
    for f in chat-shell/src/*.rs; do
        [ -f "$f" ] || continue
        head -10 "$f" | grep -q '//!' || {
            echo "Missing //! module doc in: $f"
            return 1
        }
    done
}

# ── Branding Assets ──

@test "branding ASCII logo exists" {
    [ -f branding/ascii-logo.txt ]
}

@test "branding colors doc exists" {
    [ -f branding/colors.md ]
}

# ═══════════════════════════════════════════════════════
# CROSS-COMPONENT CONSISTENCY
# ═══════════════════════════════════════════════════════

@test "config skills path matches staging layout" {
    # config.toml points to /usr/share/levsha/skills
    grep -q 'path = "/usr/share/levsha/skills"' "$STAGING/etc/levsha/config.toml"
    # And that path exists in staging
    [ -d "$STAGING/usr/share/levsha/skills" ]
}

@test "config persistence path is set" {
    grep -q 'db_path' "$STAGING/etc/levsha/config.toml"
}

@test "watchdog service references levsha-chat binary" {
    grep -q '/usr/bin/levsha-chat' "$OVERLAY/etc/systemd/system/levsha-chat-watchdog.service"
}

@test "levsha-session references levsha-chat binary" {
    grep -q 'levsha-chat' "$OVERLAY/usr/bin/levsha-session"
}
