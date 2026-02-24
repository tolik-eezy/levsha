# Levsha OS

**The Chat Is the Computer.**

A minimal Linux distribution where the entire user interface is a single, beautiful, full-screen chat. No windows, no desktop, no file manager. The chat is the computer.

See `docs/ideation/Levsha_OS_PRD_v3.md` for the full PRD.

## Architecture (4 Layers)

```
L3 — Chat Shell       Wayland-native full-screen chat GUI (Rust)
L2 — Intelligence      LLM engine: intent parsing, skill dispatch, tool calling (Anthropic Claude API)
L1 — Base System       Fedora minimal + systemd + Wayland compositor + networking
L0 — Kernel            Linux kernel (Fedora default, minimal config)
```

## Base Distribution

**Fedora** — chosen for best GTK4/libadwaita support, excellent Wayland maturity, and polished font rendering out of the box.

The Fedora base image is stored in `vendor/fedora/` (gitignored). ISO customization uses Fedora's `lorax` / `livemedia-creator` tooling with kickstart files.

## Directory Structure

```
Levsha.OS/
  chat-shell/          # L3 — Wayland-native chat GUI (Rust + GTK4/Iced)
  engine/              # L2 — Intelligence engine, LLM API client, skill orchestration
  skills/              # Skill definitions (system prompt fragments + tool schemas)
    built-in/          #   Package manager, system info (MVP)
  base/                # L1 — Fedora kickstart configs, system customization
    kickstart/         #   Kickstart files for ISO build
    overlay/           #   Filesystem overlay (configs, services, branding)
  infra/               # Build system, ISO generation, VM testing
  docs/                # Documentation, PRD, ADRs
    ideation/          #   PRD and design docs
  tests/               # Integration and system tests
  vendor/              # Third-party sources (gitignored)
    fedora/            #   Fedora base ISO and packages
```

## Technology Stack (MVP)

| Component | Choice |
|-----------|--------|
| Base distro | Fedora (minimal) |
| Init | systemd |
| Display | Wayland (wlroots or Fedora's Mutter) |
| Chat Shell | Rust (GTK4 + libadwaita or Iced) |
| LLM backend | Anthropic Claude API (HTTP, hardcoded key in MVP) |
| Persistence | SQLite |
| Package manager | dnf (Fedora native) |
| ISO tooling | lorax / livemedia-creator |

## Building

On macOS, all Rust compilation and image assembly is done inside Docker (or Podman) containers — the Makefile automatically builds a cross-compilation container and runs `cargo build` inside it, so no local Rust/Linux toolchain is needed.

## Development Workflow

### Git Worktrees (Required for Parallel Agents)

All parallel agents MUST use git worktrees to avoid file conflicts. The team lead is responsible for setting up worktrees before assigning tasks.

**Setup (team lead does this before spawning agents):**

```bash
# Create a worktree for each agent that will edit files
git worktree add ../Levsha.OS-wt-<agent-name> main
# Example:
git worktree add ../Levsha.OS-wt-chat-shell main
git worktree add ../Levsha.OS-wt-engine main
git worktree add ../Levsha.OS-wt-infra main
```

**Agent instructions:** Each agent MUST work exclusively in its assigned worktree directory (`../Levsha.OS-wt-<agent-name>/`). Never edit files in the main worktree — that belongs to the team lead.

**Merging (team lead does this after agents complete):**

```bash
# From the main worktree, merge each agent's changes
cd /Users/anatoliysveshnikov/Development/Levsha.OS
git merge --no-ff <agent-branch>   # or cherry-pick / rebase as needed
```

**Cleanup:**

```bash
git worktree remove ../Levsha.OS-wt-<agent-name>
```

### Team-Based Task Execution

Every non-trivial task MUST be planned and executed using Claude Code teams:

1. **Plan** — Break the task into subtasks using `EnterPlanMode`
2. **Team Up** — Create a team with `TeamCreate` and spawn relevant agents
3. **Worktrees** — Create a git worktree per agent (`git worktree add ../Levsha.OS-wt-<agent-name> main`)
4. **Assign** — Create tasks with `TaskCreate` and assign to agents; include the worktree path in the task description
5. **Execute** — Agents work in parallel, each in its own worktree
6. **Review** — Team lead reviews and merges all worktree branches
7. **Cleanup** — Remove worktrees (`git worktree remove`) and shut down the team

### Predefined Agents

Defined in `.claude/agents/` — spawn only those relevant to the current task:

| Agent | Role | Scope |
|-------|------|-------|
| `chat-shell` | GUI Engineer | L3 — Wayland chat GUI, rendering, animations, typography, input |
| `engine` | Intelligence Engineer | L2 — LLM API client, tool calling, skill dispatch, context mgmt |
| `skills-dev` | Skills Engineer | Skill system, built-in skills (pkg manager, sysinfo), skill format |
| `base-system` | System Engineer | L1/L0 — Fedora customization, kickstart, kernel config, services |
| `infra` | Infrastructure Engineer | ISO generation, build system, lorax, QEMU testing, CI/CD |
| `qa` | QA Engineer | Testing, boot testing, VM automation, integration tests |
| `docs` | Technical Writer | Documentation, user guide, ADRs |
| `security` | Security Engineer | Destructive command confirmation, API key handling, system hardening |

### Agent Spawning Convention

```bash
# 1. Create worktrees
git worktree add ../Levsha.OS-wt-chat-shell main
git worktree add ../Levsha.OS-wt-engine main

# 2. Create team
TeamCreate(team_name="<feature-name>")

# 3. Spawn agents — tell each agent its worktree path
Task(subagent_type="general-purpose", team_name="<feature-name>", name="chat-shell",
     prompt="Work ONLY in ../Levsha.OS-wt-chat-shell/ ...")
Task(subagent_type="general-purpose", team_name="<feature-name>", name="engine",
     prompt="Work ONLY in ../Levsha.OS-wt-engine/ ...")
```

## Coding Standards

- **Rust**: Edition 2021, `clippy` clean, `rustfmt` formatted
- **Shell scripts**: POSIX sh preferred, bash when necessary; must pass `shellcheck`
- **Config**: TOML for Rust configs, YAML for skill manifests, kickstart for Fedora
- **Naming**: snake_case for files/variables, PascalCase for Rust types
- **GUI quality**: Visual polish is non-negotiable. Less functionality > ugly UI.

## MVP Scope

**In:** Bootable ISO (VM), beautiful chat GUI, single session, cloud API (Claude), package manager skill, system info skill, persistent history, streaming responses, destructive command confirmation.

**Out:** Self-improvement, skill install/remove, multi-session, local LLM, theming, voice, Wi-Fi config.

## Git Conventions

- Branch naming: `<component>/<feature>` (e.g., `chat-shell/streaming-render`)
- Commit messages: `[component] short description` (e.g., `[engine] add tool calling`)
- `.gitignore` excludes `vendor/` and build artifacts
