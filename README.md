# Levsha OS

**The Chat Is the Computer.**

A minimal Linux distribution where the entire user interface is a single, beautiful, full-screen chat. No windows, no desktop, no file manager. The chat is the computer.

Built on Fedora, powered by Claude.

## Quick Start

```bash
# First time: full build + boot (~15 min, creates cached base image)
make dev

# Set your API key on the running VM
sshpass -p levsha ssh -o StrictHostKeyChecking=no -p 2222 levsha@localhost \
  "echo 'levsha' | sudo -S sed -i 's|YOUR_ANTHROPIC_API_KEY_HERE|sk-ant-YOUR-KEY-HERE|' /etc/levsha/config.toml"

# Restart chat to pick up the new key
sshpass -p levsha ssh -p 2222 levsha@localhost \
  "echo 'levsha' | sudo -S pkill levsha-chat"
```

After first setup, the dev cycle is:

```bash
# Terminal 1: boot the VM
make boot

# Terminal 2: build + deploy in ~5 seconds
make fast
```

## Build Targets

| Target | Time | Description |
|--------|------|-------------|
| `make fast` | ~5s | Build + deploy to running VM via SSH. Requires VM running (`make boot`). |
| `make dev` | ~30s* | Full dev rebuild. Creates bootable qcow2 + boots QEMU. |
| `make release` | — | Release build + ISO generation. |
| `make boot` | — | Boot existing qcow2 in QEMU (no rebuild). |
| `make clean` | — | Remove build artifacts (keeps cached base). |
| `make clean-all` | — | Remove everything including cached base. |

\* First `make dev` takes ~15 min (installs packages, cached afterwards).

## SSH Access

QEMU forwards port 22 inside the VM to **localhost:2222** on the host.

```bash
ssh -p 2222 levsha@localhost
# Password: levsha
```

The `levsha` user has sudo access (password: `levsha`).

### Setting the API Key

The VM ships with a placeholder API key. Set a real one before using the chat:

```bash
sshpass -p levsha ssh -o StrictHostKeyChecking=no -p 2222 levsha@localhost \
  "echo 'levsha' | sudo -S sed -i 's|YOUR_ANTHROPIC_API_KEY_HERE|sk-ant-YOUR-KEY-HERE|' /etc/levsha/config.toml"
```

Then restart the chat shell:

```bash
sshpass -p levsha ssh -p 2222 levsha@localhost \
  "echo 'levsha' | sudo -S pkill levsha-chat"
```

The API key persists across VM reboots. `make fast` does not overwrite it.

### Useful SSH Commands

```bash
# View logs
ssh -p 2222 levsha@localhost "journalctl --user -f"

# Restart chat shell (Cage auto-relaunches it)
ssh -p 2222 levsha@localhost "echo 'levsha' | sudo -S pkill levsha-chat"

# Restart entire Cage session
ssh -p 2222 levsha@localhost "echo 'levsha' | sudo -S pkill cage"
```

## Architecture

```
L3 — Chat Shell       Wayland-native full-screen chat GUI (Rust + GTK4)
L2 — Intelligence      LLM engine: intent parsing, skill dispatch, tool calling
L1 — Base System       Fedora minimal + systemd + Wayland (Cage) + networking
L0 — Kernel            Linux kernel (Fedora default)
```

## Prerequisites

- macOS or Linux host
- Docker or Podman (`brew install podman`)
- QEMU (`brew install qemu`)
- sshpass (`brew install hudochenkov/sshpass/sshpass`)
- Rust toolchain (for IDE support only; compilation happens in container)
- Anthropic API key ([console.anthropic.com](https://console.anthropic.com))

## VM Credentials

| User | Password | Notes |
|------|----------|-------|
| `levsha` | `levsha` | Default user, auto-logged in, has sudo |
| `root` | `levsha` | Direct root login disabled over SSH |

## Configuration

The engine loads config from `/etc/levsha/config.toml` with a fallback chain:

1. `LEVSHA_CONFIG` env var (custom path)
2. `./config.dev.toml` (local development)
3. `/etc/levsha/config.toml` (production)

The `ANTHROPIC_API_KEY` env var overrides `api.key` in any config file.

## Documentation

- [Developer Getting Started Guide](docs/dev-guide/getting-started.md)
- [ISO Build Guide](docs/technical/Levsha_OS_Build_Guide.md)
- [Architecture Overview](docs/architecture/overview.md)
- [L2-L3 Protocol](docs/architecture/l2-l3-protocol.md)
- [Full PRD](docs/ideation/Levsha_OS_PRD_v3.md)
