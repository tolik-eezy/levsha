#!/usr/bin/env bats

# Levsha OS — Smoke Tests
# Validates that all build artifacts are correctly staged.
# Run with: bats tests/smoke_test.sh

@test "levsha-chat binary exists in staging" {
    [ -f staging/usr/bin/levsha-chat ]
}

@test "config.toml exists in staging" {
    [ -f staging/etc/levsha/config.toml ]
}

@test "package-manager skill is staged" {
    [ -d staging/usr/share/levsha/skills/package-manager ]
}

@test "sysinfo skill is staged" {
    [ -d staging/usr/share/levsha/skills/sysinfo ]
}

@test "Plymouth theme is staged" {
    [ -f staging/usr/share/plymouth/themes/levsha/levsha.plymouth ]
}

@test "Plymouth script is staged" {
    [ -f staging/usr/share/plymouth/themes/levsha/levsha.script ]
}

@test "kickstart file exists" {
    [ -f base/kickstart/levsha-os.ks ]
}

@test "kickstart includes plymouth package" {
    grep -q "plymouth" base/kickstart/levsha-os.ks
}

@test "kickstart does not exclude plymouth" {
    ! grep -q "^-plymouth" base/kickstart/levsha-os.ks
}

@test "levsha-session script exists" {
    [ -f base/overlay/usr/bin/levsha-session ]
}

@test "systemd watchdog service exists" {
    [ -f base/overlay/etc/systemd/system/levsha-chat-watchdog.service ]
}

@test "autologin config exists" {
    [ -f base/overlay/etc/systemd/system/getty@tty1.service.d/autologin.conf ]
}

@test "branding ascii logo exists" {
    [ -f branding/ascii-logo.txt ]
}

@test "branding colors doc exists" {
    [ -f branding/colors.md ]
}
