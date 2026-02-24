# ADR-001: cage as Wayland Compositor

**Status:** Accepted
**Date:** 2025-01

---

## Context

Levsha OS runs a single full-screen application (the chat shell). We need a Wayland compositor that:

- Runs exactly one application full-screen with no window decorations
- Is minimal in size and dependencies
- Requires zero configuration
- Works with GTK4 / libadwaita
- Is available in Fedora repositories

## Options Considered

### 1. cage

A wlroots-based kiosk compositor designed to run a single application full-screen. ~2 MB binary. No config files. Usage: `cage -- /usr/bin/levsha-chat`.

**Pros:**
- Purpose-built for kiosk/single-app use cases
- Tiny footprint (~2 MB)
- Zero configuration -- no config files, no window management logic
- wlroots-based -- proven Wayland protocol implementation
- Available in Fedora repos

**Cons:**
- No multi-window support (by design)
- No screen sharing or advanced Wayland protocols
- Limited community compared to larger compositors

### 2. labwc

A stacking Wayland compositor inspired by Openbox.

**Pros:**
- More features (multi-window, theming, keybindings)
- Active development

**Cons:**
- Requires configuration to disable window decorations and force full-screen
- Unnecessary complexity for a single-app OS
- Larger footprint

### 3. Mutter (GNOME / Fedora default)

The GNOME compositor, shipped with Fedora by default.

**Pros:**
- First-class Fedora support
- Best GTK4 integration
- Mature, battle-tested

**Cons:**
- Pulls in the entire GNOME Shell stack
- ~200+ MB of dependencies
- Massive overkill for a single-app kiosk
- Hard to strip down to kiosk mode without GNOME session infrastructure

### 4. sway

An i3-compatible Wayland compositor built on wlroots.

**Pros:**
- Mature, well-tested
- wlroots-based
- Good community

**Cons:**
- Designed for tiling window management -- unnecessary for single-app use
- Requires a config file
- More moving parts than needed

## Decision

**cage** -- it is the only compositor in this list that was specifically designed for the kiosk use case. It does exactly one thing (run a single app full-screen) and does it well, with no configuration overhead.

## Consequences

- **No window management:** The chat shell is the only application that can display on screen. This is an intentional constraint, not a limitation.
- **No multi-app workflows:** If future versions of Levsha OS need to display multiple applications (e.g., a split-view terminal), cage would need to be replaced with a more capable compositor like labwc.
- **Minimal attack surface:** Fewer features means fewer bugs and less surface area for security issues.
- **Simple boot chain:** `levsha-session` is just `exec cage -- /usr/bin/levsha-chat` -- a two-line shell script.
