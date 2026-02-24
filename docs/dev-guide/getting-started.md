# Developer Getting Started Guide

---

## Prerequisites

| Requirement | Why | Install |
|-------------|-----|---------|
| macOS or Linux host | Development and build host | -- |
| Docker or Podman | Containerized builds (GTK4 cannot cross-compile natively on macOS) | `brew install podman` or install Docker Desktop |
| QEMU | VM testing | `brew install qemu` (macOS) or `dnf install qemu-system-x86` (Fedora) |
| Rust toolchain | IDE support only (actual compilation happens in the container) | `rustup install stable` |
| Anthropic API key | Cloud LLM backend | Get one at [console.anthropic.com](https://console.anthropic.com) |

> **Note:** You do not need GTK4 or libadwaita installed on your host machine. All Rust compilation happens inside a Fedora container with the correct system libraries.

---

## Quick Start

```bash
# Clone the repository
git clone https://github.com/levsha-os/Levsha.OS.git
cd Levsha.OS

# Set your API key (required for the engine to connect to Claude)
export ANTHROPIC_API_KEY="sk-ant-..."

# Build the toolchain container (one-time, ~2 min)
make toolchain

# Fast dev cycle: debug build + inject into Fedora VM + boot in QEMU
make test-dev
```

This will:
1. Build a Fedora 41 container with Rust + GTK4 dev dependencies
2. Compile the Rust workspace (debug mode) inside the container
3. Stage the binary, skills, and overlay configs
4. Inject everything into a Fedora Cloud qcow2 image
5. Boot the image in QEMU

---

## Project Structure

```
Levsha.OS/
  chat-shell/              # L3 — Wayland chat GUI (Rust, GTK4 + libadwaita)
    src/
      main.rs              #   Application entry point
      window.rs            #   Full-screen window, engine spawning
      chat_view.rs         #   Scrollable message list, typing indicator
      input_bar.rs         #   Text input with history navigation
      status_bar.rs        #   Connection status, clock, model name
      search.rs            #   Ctrl+F search overlay
      message_widget/      #   Rich text rendering
        mod.rs             #     Markdown, tables, lists, code blocks
        syntax.rs          #     Syntax highlighting (Python, Rust, Bash, JS, Go, JSON, YAML)
      streaming.rs         #   Stream state management
      error_display.rs     #   Error widgets and confirmation dialogs
      keybindings.rs       #   Keyboard shortcut setup
      css.rs               #   GTK4 CSS theme loading
  engine/                  # L2 — Intelligence engine (Rust library)
    src/
      lib.rs               #   Engine struct, main loop, tool call execution
      api_client.rs        #   Anthropic API, SSE streaming, retries
      risk_classifier.rs   #   Destructive command detection (High/Medium/Low)
      context.rs           #   Context window management, token estimation
      history.rs           #   SQLite persistence (WAL mode)
      skill_loader.rs      #   Skill discovery and YAML/JSON loading
      tool_executor.rs     #   Command template rendering, process execution
      config.rs            #   TOML config loading with dev-mode fallback
      types.rs             #   ShellToEngine / EngineToShell message enums
      protocol.rs          #   Channel creation (create_channels)
      welcome.rs           #   Welcome message / history restoration
  skills/                  # Skill definitions
    built-in/
      system-prompt.md     #   Base system prompt
      package-manager/     #   dnf package management (5 tools)
      sysinfo/             #   System info queries (6 tools)
  base/                    # L1 — Fedora system customization
    kickstart/             #   Kickstart files for ISO build
    overlay/               #   Filesystem overlay deployed to the OS
      etc/levsha/          #     Production config.toml
      etc/systemd/system/  #     Auto-login drop-in, watchdog service
      etc/fonts/           #     Font configuration
      etc/sudoers.d/       #     Passwordless sudo for dnf
      usr/bin/             #     levsha-session launcher script
      usr/share/plymouth/  #     Custom Plymouth boot splash theme
  infra/                   # Build system and ISO generation
    Containerfile          #   Fedora build container (Rust + GTK4 deps)
    Containerfile.iso      #   ISO tools container (libguestfs)
    inject-artifacts.sh    #   Script to inject artifacts into qcow2
    build.sh               #   Full ISO build script
  docs/                    # Documentation
    architecture/          #   Architecture overview, protocol spec, ADRs
    dev-guide/             #   This guide
    PRD/                   #   Product requirements (per-component)
    ideation/              #   Original PRD
  tests/                   # Integration and system tests
    smoke_test.sh          #   BATS smoke tests
  vendor/                  # Third-party sources (gitignored)
    fedora/                #   Fedora Cloud base image (downloaded on first build)
  Cargo.toml               # Workspace manifest (2 members: chat-shell, engine)
  Makefile                 # Build system entry point
```

---

## Development Workflow

### 1. Edit code on your host

Use any editor with Rust support (VS Code + rust-analyzer, IntelliJ + Rust plugin, neovim, etc.). The Rust toolchain on your host is only for IDE features -- you do not compile locally.

### 2. Build inside the container

```bash
# Debug build (faster, uses cargo cache volume)
make build-dev

# Release build (optimized, slower)
make build
```

The container mounts your project directory and compiles the full Rust workspace inside Fedora with the correct GTK4/libadwaita libraries.

### 3. Stage artifacts

```bash
# Stage debug artifacts
make stage-dev

# Stage release artifacts
make stage
```

Staging copies the compiled binary, skills, and overlay configs into `staging/` in the layout expected by the target filesystem.

### 4. Test in QEMU

```bash
# Full dev cycle: debug build + stage + inject + boot
make test-dev

# Just boot an existing qcow2 (no rebuild)
make test-quick-boot
```

On ARM Macs, QEMU uses `aarch64` with HVF acceleration for fast iteration. On x86_64 hosts, it uses native KVM.

### 5. Run tests

```bash
# BATS smoke tests (requires bats-core)
make smoke-test

# Engine unit tests (run inside the container)
podman run --rm -v .:/build:z levsha-builder cargo test --workspace
```

---

## Configuration

The engine loads configuration using a fallback chain:

1. `LEVSHA_CONFIG` environment variable (custom path)
2. `./config.dev.toml` in the current directory (local development)
3. `/etc/levsha/config.toml` (production, deployed via overlay)

The `ANTHROPIC_API_KEY` environment variable overrides the `api.key` field in any config file.

### Example `config.dev.toml`

Create this file in the project root for local development:

```toml
[api]
key = "sk-ant-..."  # or set ANTHROPIC_API_KEY env var instead
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
```

---

## Make Targets Reference

| Target | Description |
|--------|-------------|
| `make toolchain` | Build the Fedora build container with Rust + GTK4 dev deps |
| `make build` | Release build inside the container |
| `make build-dev` | Debug build with persistent cargo cache volume (faster) |
| `make stage` | Stage release artifacts into `staging/` |
| `make stage-dev` | Stage debug artifacts into `staging/` |
| `make iso` | Full ISO build (build + stage + ISO generation via lorax) |
| `make test-vm` | Boot the built ISO in QEMU |
| `make test-quick` | Release build + inject into qcow2 + boot QEMU |
| `make test-dev` | Debug build + inject (no SELinux relabel) + boot QEMU (fastest) |
| `make test-quick-boot` | Boot existing qcow2 in QEMU (no rebuild) |
| `make smoke-test` | Run BATS smoke tests |
| `make clean` | Remove `staging/`, `target/`, test qcow2, and build dir |

---

## Adding a New Skill

Skills are self-contained bundles in `skills/built-in/`. Each skill has a YAML manifest, markdown prompt, and JSON tool definitions.

### 1. Create the skill directory

```
skills/built-in/my-skill/
  skill.yaml
  prompts/my-skill.md
  tools/my-tool.json
```

### 2. Write the manifest (`skill.yaml`)

```yaml
name: my-skill
version: 0.1.0
description: "Brief description of what this skill does"
author: levsha
builtin: true

prompt: prompts/my-skill.md

tools:
  - tools/my-tool.json

requires:
  packages: []

assets: []
```

### 3. Write the prompt (`prompts/my-skill.md`)

This markdown is injected into the system prompt to give the LLM context about when and how to use the tools:

```markdown
## My Skill

Use the `my_tool` tool when the user asks about [topic].
Always provide [specific guidance for the LLM].
```

### 4. Define tools (`tools/my-tool.json`)

```json
{
  "name": "my_tool",
  "description": "What this tool does. Use when the user asks about [topic].",
  "input_schema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "The search query."
      }
    },
    "required": ["query"]
  },
  "execution": {
    "type": "shell",
    "command_template": "my-command {{query | quote}}"
  }
}
```

The `name`, `description`, and `input_schema` fields are sent to the Anthropic API. The `execution` block stays on the engine side. See [Tool Schema](../architecture/tool-schema.md) for the full specification.

### 5. Rebuild and test

```bash
make test-dev
```

The skill loader automatically discovers all skills in the configured skills path.

---

## Architecture Overview

Levsha OS uses a 4-layer architecture (L0 Kernel, L1 Base System, L2 Intelligence Engine, L3 Chat Shell). The engine is embedded in the chat shell as a Rust library -- they share a single binary.

For full details, see [Architecture Overview](../architecture/overview.md).

---

## Troubleshooting

**"GTK4 not found" or linker errors**
You must build inside the container, not directly on macOS. Run `make build-dev` instead of `cargo build`.

**"No config file found"**
Create a `config.dev.toml` in the project root (see the example above), or set the `LEVSHA_CONFIG` environment variable.

**"API key invalid" or "authentication failed"**
Set the `ANTHROPIC_API_KEY` environment variable, or update the `api.key` field in your config file.

**QEMU shows a black screen**
Check that the qcow2 file exists (`ls levsha-test.qcow2`). If not, run `make test-dev` from scratch to build and inject artifacts.

**Container build fails**
Ensure Docker or Podman is running. Check your internet connection (the container downloads Fedora packages).

**"Permission denied" during qcow2 injection**
The injection script requires `--privileged` mode (for libguestfs). This is handled automatically by the Makefile.

**Slow builds on ARM Mac**
Debug builds (`make build-dev`) use a persistent cargo cache volume and are significantly faster than release builds. Use `make test-dev` for the fastest iteration cycle.
