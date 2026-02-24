# Levsha OS — Phase 2 Implementation Plan

## Context

Phase 1 is complete: a bootable ISO with a polished full-screen chat GUI, Claude API integration, two built-in skills (package manager, system info), persistent conversation history, streaming responses, and destructive command confirmation. ~3,900 LOC of Rust across `chat-shell/` and `engine/`.

Phase 2 makes the OS **alive** — it can modify itself, grow new skills, work offline, and become a real development environment. This is the phase that transforms Levsha OS from a proof of concept into a platform.

**Strategy:** Foundation first, capabilities second. Build the infrastructure (API key config, skill repository, split-view) before the features that depend on it (self-improvement, local LLM, new skills).

---

## Phase 2 Modules

| # | Module | PRD | Priority | Description |
|---|--------|-----|----------|-------------|
| 10 | Self-Improvement | `10-self-improvement/` | P0 | The defining feature. OS reads/patches/rebuilds itself. |
| 11 | Skill Repository | `11-skill-repository/` | P0 | Install/remove skills from Git repos. |
| 12 | Split-View | `12-split-view/` | P0 | Side panel for images, files, diffs, progress. |
| 13 | API Key Config | `13-api-key-config/` | P0 | User-configurable key, model selection. |
| 14 | Local LLM | `14-local-llm/` | P0 | llama.cpp + intelligent routing. |
| 15 | Filesystem Skill | `15-filesystem-skill/` | P1 | Structured file management. |
| 16 | Network Config Skill | `16-network-config-skill/` | P1 | Wi-Fi, IP, DNS management. |
| 17 | Text Editor Skill | `17-text-editor-skill/` | P1 | Line-level editing with undo. |
| 18 | Code Editor Skill | `18-code-editor-skill/` | P1 | Project nav, build/run, Git integration. |
| 19 | Multi-Session | `19-multi-session/` | P0 | Multiple conversation threads, session switching, auto-naming. |

---

## Dependency Graph

```
Phase 2.0 (Foundation)
    │
    ├── 13 — API Key Config ──────────────────────────┐
    │                                                   │
    ├── 12 — Split-View Rendering ─────────┐           │
    │                                       │           │
    └── 11 — Skill Repository ─────────┐   │           │
                                        │   │           │
Phase 2.1 (Core Capabilities)          │   │           │
    │                                   │   │           │
    ├── 14 — Local LLM ◄───────────────┼───┼───────────┘
    │                                   │   │
    ├── 10 — Self-Improvement ◄─────────┼───┘
    │        (needs 11 + 12 + 18)       │
    │                                   │
    └── 15 — Filesystem Skill ◄─────────┘
                                        │
Phase 2.1b (Session Management)         │
    │                                   │
    └── 19 — Multi-Session ◄────────────┼──── (needs 02, 03, 07)
                                        │
Phase 2.2 (Extended Skills)             │
    │                                   │
    ├── 16 — Network Config Skill ◄─────┘
    ├── 17 — Text Editor Skill ◄────────── (needs 12)
    └── 18 — Code Editor Skill ◄────────── (needs 12, 17)
```

---

## Key Architectural Decisions

1. **Source tree on-device** — `/usr/src/levsha/` is a full Git repository with the OS source. Self-improvement modifies this, builds locally. ~2.5 GB for toolchain (Rust + GTK4 dev headers + git).
2. **Skill hot-reload** — Skills are prompt fragments + tool JSON. Installing/removing a skill rescans the directory and injects into the next API call. No restart needed for pure skill changes.
3. **Dual API client** — Engine gets an OpenAI-compatible client alongside the Anthropic client. llama.cpp exposes an OpenAI-compatible endpoint. Router layer selects backend. Phase 2 uses TCP on localhost:8080; the `LocalTransport` enum supports switching to Unix domain sockets (`/run/levsha/llama.sock`) for lower-latency IPC in a future phase.
4. **Split-view via GTK4 Paned** — `GtkPaned` widget divides the Chat Shell into chat + content panel. New L2→L3 message types (`ContentOpen`, `ContentUpdate`, `ContentClose`) control the panel.
5. **Checkpoint-based rollback** — Before any self-modification deploy, current binaries are copied to `/var/lib/levsha/checkpoints/`. Failed health check → automatic restore.
6. **First-boot key flow** — Empty `key` in config triggers a styled key-entry prompt in the Chat Shell. No separate setup wizard.
7. **Session-scoped state** — Each session has independent conversation history, context window, and split-view content. Sessions stored in SQLite `sessions` table with UUID PKs. Existing `messages.session_id` column ties messages to sessions. Auto-naming via lightweight LLM side-request after first assistant response.

---

## Phase 2.0: Foundation (~3-4 hrs, 3 agents in parallel)

**Team:** `levsha-p2-foundation`

These modules have no Phase 2 dependencies — they extend the Phase 1 codebase directly.

### Track A — API Key Configuration (`engine` + `chat-shell` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Config key detection | `engine` | `engine/src/config.rs` | Detect empty/missing API key at startup. Send `KeyRequired` message to Chat Shell. |
| Key entry UI | `chat-shell` | `chat-shell/src/key_entry.rs`, `style.css` | Styled key entry prompt in the chat view. Masked input. Replaces welcome message on first boot when no key. |
| Key validation | `engine` | `engine/src/api_client.rs` | Send minimal test request to validate key before accepting. Return success/failure to Chat Shell. |
| Key storage | `engine` | `engine/src/config.rs` | Write validated key to config file. Set file permissions to 600. |
| Model selection | `engine` | `engine/src/config.rs`, `engine/src/api_client.rs` | Read available models from config. Expose `model_switch` tool. Update status bar. |
| Key change flow | `engine` | `engine/src/lib.rs` | Recognize "change API key" as a command. Re-trigger key entry flow. Validate new key before replacing. |
| L2↔L3 protocol update | both | `engine/src/types.rs`, `docs/architecture/l2-l3-protocol.md` | New messages: `KeyRequired`, `KeyEntry`, `KeyValidationResult`, `ModelSwitch`. |

**Deliverable:** ISO boots without a hardcoded key. User enters key on first boot. Key persists. Model switching works.

### Track B — Split-View Content Rendering (`chat-shell` agent)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Paned layout | `chat-shell` | `chat-shell/src/window.rs`, `chat-shell/src/content_panel.rs` | Replace single chat view with `GtkPaned` (chat left, content right). Content panel starts hidden. |
| Content panel widget | `chat-shell` | `chat-shell/src/content_panel.rs`, `style.css` | Content panel container with close button, resize handle, content type dispatching. |
| Image renderer | `chat-shell` | `chat-shell/src/content_panel/image.rs` | Load and display images (PNG, JPEG, SVG, WebP). Zoom controls. Fit-to-panel. |
| Text/code renderer | `chat-shell` | `chat-shell/src/content_panel/text.rs` | Syntax-highlighted text display with line numbers. Scrollable. Word wrap toggle. |
| Diff renderer | `chat-shell` | `chat-shell/src/content_panel/diff.rs` | Unified diff view with color-coded additions/deletions. |
| Progress renderer | `chat-shell` | `chat-shell/src/content_panel/progress.rs` | Build progress, download progress, batch operation status. |
| L2→L3 content messages | `chat-shell` + `engine` | `engine/src/types.rs`, `chat-shell/src/window.rs` | New messages: `ContentOpen(type, data)`, `ContentUpdate(data)`, `ContentClose`. Route from engine to panel. |
| Keyboard shortcuts | `chat-shell` | `chat-shell/src/keybindings.rs` | Escape / Ctrl+W close panel. Ctrl+Shift+P toggle panel. |

**Deliverable:** Split-view works. Images, code files, diffs, and progress render in the panel.

### Track C — Skill Repository System (`engine` agent)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Skill install tool | `engine` | `engine/src/skill_installer.rs` | Clone Git repo to staging, validate manifest, install deps via dnf, copy to `/usr/share/levsha/skills/installed/`. |
| Skill remove tool | `engine` | `engine/src/skill_installer.rs` | Remove installed skill directory. Block removal of built-in skills. |
| Skill registry client | `engine` | `engine/src/skill_registry.rs` | Fetch and parse registry `index.yaml` from Git. Map skill names to repo URLs. |
| Skill search/list tools | `engine` | `engine/src/skill_installer.rs` | `list_skills` (installed), `search_skills` (registry), `skill_info` (details). |
| Skill hot-reload | `engine` | `engine/src/skill_loader.rs` | After install/remove, rescan skills directory. Update system prompt and tool definitions for next API call. |
| Skill update tool | `engine` | `engine/src/skill_installer.rs` | Pull latest from Git, validate, replace. Backup previous version. |
| Installed skills directory | `base-system` | `base/overlay/`, `base/kickstart/levsha-os.ks` | Create `/usr/share/levsha/skills/installed/` with correct permissions. Add `git` to kickstart packages. |
| Config update | `engine` | `engine/src/config.rs` | New config fields: `registry_url`, `installed_path`. |
| Skill management skill | `skills-dev` | `skills/built-in/skill-manager/skill.yaml`, tools, prompt | Built-in skill that exposes install/remove/search/list/update as tools. |

**Deliverable:** Skills can be installed from Git repos, removed, updated, searched. Hot-reload works.

---

## Phase 2.1: Core Capabilities (~4-6 hrs, 3 agents in parallel)

**Team:** `levsha-p2-core`

Depends on Phase 2.0 completion.

### Track D — Local LLM Support (`engine` + `base-system` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| OpenAI-compatible client | `engine` | `engine/src/openai_client.rs` | HTTP client for OpenAI-compatible API (llama.cpp server). SSE streaming. Tool use support. |
| Backend router | `engine` | `engine/src/router.rs` | Rule-based classifier: simple queries → local, complex → cloud, user override, self-improvement → always cloud. |
| Router integration | `engine` | `engine/src/lib.rs` | Plug router into the main conversation loop. Route each request to the appropriate backend. |
| llama-server systemd service | `base-system` | `base/overlay/etc/systemd/system/llama-server.service` | Systemd service for llama.cpp server. On-demand start. |
| Model management tools | `engine` | `engine/src/model_manager.rs` | Download, list, switch, delete local models. |
| Model management skill | `skills-dev` | `skills/built-in/model-manager/skill.yaml`, tools, prompt | Exposes model management as skill tools. |
| Backend switching | `engine` | `engine/src/router.rs`, `engine/src/types.rs` | User commands: "use local model", "use cloud", "use auto". Persisted in config. |
| Status bar backend info | `chat-shell` | `chat-shell/src/status_bar.rs` | Show active backend: `▸ claude-sonnet`, `▸ llama-3-8b (local)`, `▸ auto`. |
| Offline fallback | `engine` | `engine/src/router.rs` | Cloud failure → automatic fallback to local if available. Notify user. |
| Config update | `engine` | `engine/src/config.rs` | New sections: `[local_model]`, `[routing]`. Include `socket_path` option for future Unix domain socket transport (`/run/levsha/llama.sock`); Phase 2 uses TCP by default. |
| Kickstart updates | `base-system` | `base/kickstart/levsha-os.ks` | Add llama.cpp binary. Model storage directory. Increased VM storage (20 GB). Create `/run/levsha/` directory for future UDS transport. |

**Deliverable:** System works offline with local model. Intelligent routing selects backend. User can switch manually.

### Track E — Self-Improvement System (`engine` + `base-system` + `chat-shell` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Source tree setup | `base-system` | `base/overlay/usr/src/levsha/`, kickstart | OS source deployed as Git repo at `/usr/src/levsha/`. Includes chat-shell/, engine/, skills/, base/overlay/, Cargo.toml, Makefile. |
| Source read/search tools | `engine` | `engine/src/self_improve.rs` | `source_read(path)`, `source_search(pattern)`. Scoped to `/usr/src/levsha/`. |
| Source write tool | `engine` | `engine/src/self_improve.rs` | `source_write(path, content)`. Creates working copy. Tracks modifications. |
| Source diff tool | `engine` | `engine/src/self_improve.rs` | `source_diff()`. Shows Git diff of working changes. Renders in split-view panel. |
| Build tool | `engine` | `engine/src/self_improve.rs` | `build(component)`. Runs `cargo build --release` in source tree. Streams build output to split-view. |
| Test tool | `engine` | `engine/src/self_improve.rs` | `test(component)`. Runs `cargo test`. Reports results. |
| Checkpoint system | `engine` | `engine/src/checkpoint.rs` | Before deploy: copy current binaries to `/var/lib/levsha/checkpoints/<timestamp>-<commit>/`. Store commit hash, binary paths. |
| Deploy tool | `engine` | `engine/src/self_improve.rs` | Copy built artifacts to runtime locations. User confirmation required (shows diff). |
| Restart logic | `engine` | `engine/src/self_improve.rs` | Restart affected systemd service. For engine changes: restart full levsha-chat process. |
| Health check | `engine` | `engine/src/checkpoint.rs` | After restart: verify Chat Shell responds within 10s. If not → rollback. |
| Rollback tool | `engine` | `engine/src/checkpoint.rs` | `rollback()`. Restore from latest checkpoint. Restart. |
| Git tools | `engine` | `engine/src/self_improve.rs` | `git_log()`, `git_commit(message)`. Auto-commit before deploy. User-viewable history. |
| Self-improvement skill | `skills-dev` | `skills/built-in/self-improve/skill.yaml`, tools, prompt | Skill manifest exposing all self-improvement tools. Prompt instructs LLM on safe self-modification workflow. |
| Confirmation UX | `chat-shell` | `chat-shell/src/message_widget.rs` | Diff display in split-view before deploy. Styled "Apply changes?" confirmation in chat. |
| Rust toolchain in ISO | `base-system` | `base/kickstart/levsha-os.ks`, `infra/inject-artifacts.sh` | Install Rust toolchain, GTK4 dev headers, gcc, make, pkg-config in the base image. ~800 MB additional. |

**Deliverable:** User can ask the OS to fix a bug or add a feature. OS reads source, writes a patch, builds, shows diff, deploys with user approval. Rollback on failure.

### Track F — Filesystem Skill (`skills-dev` agent)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Skill manifest | `skills-dev` | `skills/built-in/filesystem/skill.yaml` | Skill definition with all file tools. |
| File browsing tools | `skills-dev` | `skills/built-in/filesystem/tools/fs_{list,tree,find,size}.json` | Directory listing, tree view, find, size. |
| File operation tools | `skills-dev` | `skills/built-in/filesystem/tools/fs_{copy,move,delete,mkdir,chmod}.json` | Copy, move, delete, mkdir, chmod. Delete/overwrite marked destructive. |
| File viewing tools | `skills-dev` | `skills/built-in/filesystem/tools/fs_{read,head,tail}.json` | Read full file, head N lines, tail N lines. |
| File editing tools | `skills-dev` | `skills/built-in/filesystem/tools/fs_{write,append,replace}.json` | Write, append, find-and-replace. |
| Skill prompt | `skills-dev` | `skills/built-in/filesystem/prompts/filesystem.md` | LLM instructions for file operations, safety guidelines, split-view integration. |

**Deliverable:** Filesystem skill with 15 tools. Installed as built-in.

### Track J — Multi-Session Support (`engine` + `chat-shell` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Sessions table | `engine` | `engine/src/session.rs`, `engine/src/persistence.rs` | SQLite `sessions` table: id (UUID), name, created_at, updated_at, message_count, is_archived, metadata (JSON). CRUD operations. |
| Session manager | `engine` | `engine/src/session.rs` | SessionManager: create, switch, list, rename, archive, restore, delete sessions. Enforces max_active limit from config. |
| Per-session context | `engine` | `engine/src/context.rs` | ContextManager becomes session-aware. `switch_session()` reloads conversation history for the active session. New messages scoped to current session_id. |
| Auto-naming | `engine` | `engine/src/session.rs` | After first assistant response in a new session, fire lightweight LLM side-request (max_tokens=10) to generate 2-4 word title. Update session name. |
| Session IPC messages | `engine` + `chat-shell` | `engine/src/types.rs`, `chat-shell/src/window.rs` | New L2↔L3 messages: `SessionCreate`, `SessionSwitch`, `SessionList`, `SessionRename`, `SessionArchive`, `SessionDelete`, `SessionNameUpdate`. |
| Session list overlay | `chat-shell` | `chat-shell/src/session_list.rs`, `style.css` | Modal overlay (Ctrl+Shift+S) listing sessions with name, timestamp, preview. Active indicator (●/○). Click to switch. [+ New] button. |
| Status bar session name | `chat-shell` | `chat-shell/src/window.rs`, `chat-shell/src/status_bar.rs` | Add session name segment to status bar. Hidden when only one session exists. Click opens session list. |
| Session switching animation | `chat-shell` | `chat-shell/src/window.rs` | Current chat fades out (150ms), new session fades in (200ms). Split-view panel state restores per session. |
| Session keyboard shortcuts | `chat-shell` | `chat-shell/src/keybindings.rs` | Ctrl+N (new), Ctrl+Tab / Ctrl+Shift+Tab (cycle), Ctrl+Shift+S (list), Ctrl+1..9 (direct switch). |
| Session context banner | `chat-shell` | `chat-shell/src/message_widget.rs` | When switching sessions, show separator: "── Session Name · N messages · last active X min ──" |
| Config update | `engine` | `engine/src/config.rs`, `base/overlay/etc/levsha/config.toml` | New `[sessions]` config section: max_active=20, auto_name=true, default_name="New Session". |
| Delete confirmation UX | `chat-shell` | `chat-shell/src/message_widget.rs` | Styled confirmation dialog for permanent session deletion. Shows message count and warning. |

**Deliverable:** Users can create, switch, name, archive, and delete multiple conversation sessions. Each session has independent history and context.

---

## Phase 2.2: Extended Skills (~2-3 hrs, 3 agents in parallel)

**Team:** `levsha-p2-skills`

Depends on Phase 2.0 (skill repository) and Phase 2.1 (filesystem skill for text/code editors).

### Track G — Network Config Skill (`skills-dev` agent)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Skill manifest | `skills-dev` | `skills/built-in/network-config/skill.yaml` | Full skill with 13 tools. |
| Wi-Fi tools | `skills-dev` | `skills/built-in/network-config/tools/wifi_*.json` | Scan, connect, disconnect, forget, saved. |
| IP config tools | `skills-dev` | `skills/built-in/network-config/tools/net_*.json` | Interfaces, static IP, DHCP, DNS. |
| Diagnostic tools | `skills-dev` | `skills/built-in/network-config/tools/net_{ping,traceroute,dns_lookup,ports}.json` | Ping, traceroute, dig, ss. |
| Skill prompt | `skills-dev` | `skills/built-in/network-config/prompts/network-config.md` | LLM instructions for network management. |
| Kickstart packages | `base-system` | `base/kickstart/levsha-os.ks` | Add bind-utils, traceroute if not present. |

**Deliverable:** Network configuration via chat. Wi-Fi works.

### Track H — Text Editor Skill (`skills-dev` + `chat-shell` agents)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Skill manifest | `skills-dev` | `skills/built-in/text-editor/skill.yaml` | Full skill with 11 tools. |
| Edit state in engine | `engine` | `engine/src/edit_state.rs` | In-memory edit buffer: opened file, undo stack, dirty tracking. |
| Line-level tools | `skills-dev` | `skills/built-in/text-editor/tools/edit_*.json` | Open, insert, delete_lines, replace_lines, replace_text, append. |
| Save/undo tools | `skills-dev` | `skills/built-in/text-editor/tools/edit_{save,undo,redo,diff,close}.json` | Save, undo, redo, diff, close. |
| Split-view integration | `chat-shell` | `chat-shell/src/content_panel/text.rs` | File opens in content panel. Edits update in real-time. Dirty indicator. |
| Skill prompt | `skills-dev` | `skills/built-in/text-editor/prompts/text-editor.md` | LLM instructions for editing workflows. |

**Deliverable:** Text editing via chat with split-view preview, undo, save.

### Track I — Code Editor Skill (`skills-dev` agent)

| Task | Agent | Files | Description |
|------|-------|-------|-------------|
| Skill manifest | `skills-dev` | `skills/built-in/code-editor/skill.yaml` | Full skill with 16 tools. |
| Project nav tools | `skills-dev` | `skills/built-in/code-editor/tools/code_*.json` | Open project, find definition, find references, tree, search. |
| Build/run tools | `skills-dev` | `skills/built-in/code-editor/tools/code_{build,run,test}.json` | Build, run, test with project type detection. |
| Git tools | `skills-dev` | `skills/built-in/code-editor/tools/git_*.json` | Status, diff, add, commit, log, branch, push, pull. |
| Skill prompt | `skills-dev` | `skills/built-in/code-editor/prompts/code-editor.md` | LLM instructions for development workflows, language detection, Git conventions. |

**Deliverable:** Full development environment via chat. Build, test, Git workflow.

---

## Phase 2.3: Integration & Wiring (~2-3 hrs, mostly sequential)

**Team:** `levsha-p2-integration`

### Wiring Checks

| Check | Components | Verification |
|-------|-----------|--------------|
| First-boot key flow | 13 ↔ chat-shell ↔ engine | Boot fresh ISO → key prompt appears → enter valid key → welcome message → chat works. Enter invalid key → error → retry. |
| Key persistence | 13 ↔ engine | Enter key → reboot → chat works (no re-prompt). Key file has 600 permissions. |
| Model switch | 13 ↔ engine ↔ chat-shell | "Use Opus" → next request uses opus model → status bar updates → persists across restart. |
| Skill install from URL | 11 ↔ engine | "Install skill from https://..." → repo cloned → manifest validated → deps installed → skill available → next message uses new tools. |
| Skill install from name | 11 ↔ engine | "Install filesystem skill" → registry queried → repo cloned → installed → available. |
| Skill remove | 11 ↔ engine | "Remove filesystem skill" → confirmation → removed → tools no longer in context. |
| Skill hot-reload | 11 ↔ engine | After install, the very next message includes the new skill's tools. No restart. |
| Split-view image | 12 ↔ chat-shell | "Show me /tmp/photo.png" → image opens in content panel. Zoom works. Escape closes. |
| Split-view file | 12 ↔ chat-shell ↔ engine | "Show me /etc/levsha/config.toml" → file opens in panel with syntax highlighting. |
| Split-view diff | 12 ↔ chat-shell ↔ 10 | Self-improvement shows diff in panel before deploy confirmation. |
| Local model start | 14 ↔ engine ↔ base-system | llama-server starts on demand → engine sends request → gets response → renders in chat. |
| Auto routing | 14 ↔ engine | "What time is it?" → routes to local. "Help me write a Rust parser" → routes to cloud. |
| Offline fallback | 14 ↔ engine | Disconnect network → cloud requests fail → auto-fallback to local → user informed. |
| Backend switch | 14 ↔ engine ↔ chat-shell | "Use local model" → all requests go local → status bar shows `▸ llama-3-8b (local)`. |
| Self-improvement read | 10 ↔ engine | "Show me the Chat Shell source" → source_read tool returns file content. |
| Self-improvement patch | 10 ↔ engine ↔ chat-shell | "Make the background darker" → reads CSS → writes patch → shows diff in panel → user approves → builds → deploys → Chat Shell restarts with new style. |
| Self-improvement rollback | 10 ↔ engine | After a bad deploy → health check fails → auto-rollback → previous version restored → user informed. |
| Manual rollback | 10 ↔ engine | "Undo the last change" → rollback tool restores from checkpoint → restart → verified. |
| Git versioning | 10 ↔ engine | "Show me what changed" → git_log shows commits. "Show the diff" → displays in panel. |
| Filesystem skill | 15 ↔ engine | "List files in /home" → fs_list tool → formatted output. "Delete /tmp/test.txt" → confirmation → deleted. |
| Wi-Fi connect | 16 ↔ engine ↔ base-system | "Scan for Wi-Fi" → wifi_scan → networks listed. "Connect to MyNetwork" → password prompt → connected. |
| Text editor | 17 ↔ engine ↔ chat-shell | "Edit /etc/hosts" → file opens in panel. "Add line '10.0.0.1 myserver'" → inserted → panel updates. "Save" → written. "Undo" → reverted. |
| Code editor build | 18 ↔ engine ↔ chat-shell | "Open project /usr/src/levsha" → tree displayed. "Build the engine" → cargo build output in panel. |
| Code editor Git | 18 ↔ engine | "Show Git status" → status displayed. "Commit with message 'fix bug'" → committed. |
| Session create | 19 ↔ engine ↔ chat-shell | "New session" or Ctrl+N → new session created → clean chat → status bar updates. |
| Session switch | 19 ↔ engine ↔ chat-shell | Ctrl+Tab → cycles to next session → history restores → split-view state restores. |
| Session list | 19 ↔ chat-shell | Ctrl+Shift+S → overlay with all sessions → click session → switches. |
| Session auto-name | 19 ↔ engine | Create new session → send first message → session auto-named based on content → status bar updates. |
| Session archive/restore | 19 ↔ engine | "Archive this session" → hidden from list. "Show archived" → visible. "Restore" → active again. |
| Session delete | 19 ↔ engine ↔ chat-shell | "Delete this session" → confirmation dialog → delete → switch to another session. |
| Session persistence | 19 ↔ engine | Create sessions → reboot → all sessions preserved with history. |
| Session-scoped state | 19 ↔ 12 ↔ engine ↔ chat-shell | Open file in split-view in session A → switch to B (panel closes) → switch back to A (panel restores). |

### Build & Release

| Task | Agent | Description |
|------|-------|-------------|
| Update kickstart | `base-system` | Add all new packages: git, rust toolchain, GTK4 dev, llama.cpp, bind-utils, traceroute, tree, ripgrep. |
| Update overlay | `base-system` | New systemd services (llama-server), updated config.toml (empty key, new sections), source tree at /usr/src/levsha/. |
| Update Makefile | `infra` | New build targets for local model download, source tree packaging. |
| ISO size check | `infra` | Verify ISO < 3 GB. VM disk requirement: 20 GB. |
| Rebuild ISO | `infra` | Full rebuild with all Phase 2 components. |
| Boot test | `qa` | Boot → key entry → chat works → install a skill → use local model → self-improve. |

---

## Verification

### After Phase 2.0 — Foundation

- [ ] ISO boots without hardcoded API key
- [ ] First-boot key prompt appears and works
- [ ] Key persists across reboot (file permissions 600)
- [ ] Model switching works and status bar updates
- [ ] Split-view panel opens/closes/resizes
- [ ] Image rendering in panel works
- [ ] Code file rendering with syntax highlighting works
- [ ] Diff rendering with color-coded +/- works
- [ ] Skill install from Git URL works
- [ ] Skill remove works (with confirmation)
- [ ] Skill hot-reload: installed skill available on next message
- [ ] Built-in skills cannot be removed
- [ ] Skill registry search works

### After Phase 2.1 — Core

- [ ] llama-server starts and serves local model
- [ ] Local model inference works (no internet)
- [ ] Auto routing: simple → local, complex → cloud
- [ ] Manual backend switch works and persists
- [ ] Offline mode: auto-fallback to local, user informed
- [ ] Self-improvement: LLM reads source via `source_read`
- [ ] Self-improvement: LLM writes patch via `source_write`
- [ ] Self-improvement: build succeeds via `build` tool
- [ ] Self-improvement: diff shown in split-view before deploy
- [ ] Self-improvement: user confirmation required for deploy
- [ ] Self-improvement: checkpoint created before deploy
- [ ] Self-improvement: auto-rollback on failed health check
- [ ] Self-improvement: manual rollback works ("undo last change")
- [ ] Self-improvement: Git commit created for every modification
- [ ] Filesystem skill: all 15 tools work
- [ ] Filesystem skill: delete triggers confirmation
- [ ] Multi-session: Ctrl+N creates new session with clean context
- [ ] Multi-session: Ctrl+Tab cycles between sessions
- [ ] Multi-session: Ctrl+Shift+S opens session list overlay
- [ ] Multi-session: session auto-named after first response
- [ ] Multi-session: session history preserved on switch
- [ ] Multi-session: split-view state is per-session
- [ ] Multi-session: archive hides session, restore brings it back
- [ ] Multi-session: delete requires confirmation and removes permanently
- [ ] Multi-session: sessions persist across reboot
- [ ] Multi-session: status bar shows session name (hidden when only 1 session)

### After Phase 2.2 — Skills

- [ ] Network: Wi-Fi scan, connect, disconnect work
- [ ] Network: IP/DNS configuration works
- [ ] Network: diagnostic tools (ping, traceroute) work
- [ ] Text editor: file opens in split-view panel
- [ ] Text editor: line-level insert/delete/replace works
- [ ] Text editor: save writes to disk
- [ ] Text editor: undo reverts last operation
- [ ] Text editor: close prompts for unsaved changes
- [ ] Code editor: project navigation works
- [ ] Code editor: build/run with auto-detection works
- [ ] Code editor: Git workflow (status, add, commit, push) works

### Performance & Size

- [ ] Idle RAM < 2 GB (with local model loaded)
- [ ] Idle RAM < 1 GB (cloud-only mode)
- [ ] ISO size < 3 GB
- [ ] VM disk: 20 GB sufficient for OS + model + source
- [ ] Local model first-token latency < 3s (8B model)
- [ ] All Phase 1 functionality still works

---

## Risk Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Rust toolchain in ISO makes it huge (~800 MB) | Large ISO, slow downloads | Consider remote build server in Phase 2.1. Ship toolchain as an installable skill. |
| Self-improvement breaks the system | Unbootable OS | Checkpoint + auto-rollback + Git versioning. Mandatory diff review. |
| Local model quality too low for tool use | Wrong commands executed | Route tool-heavy tasks to cloud. Local model only for simple queries. |
| Skill hot-reload race condition | Garbled context | Lock skill loader during reload. Queue requests during transition. |
| llama.cpp OOM on 2GB VM | Crash | Require 4GB minimum RAM for local model. Fall back to cloud on OOM. |
| Split-view breaks fullscreen aesthetic | Ugly UI | Careful GTK4 Paned styling. Content panel matches chat visual language. |
| Wi-Fi password handling | Security concern | Passwords go directly to NetworkManager. Never stored in chat history or logs. |
| Session context reload overhead | Slow switching | Load only last N messages into context window on switch. Full history stays in SQLite. |
| Auto-naming LLM cost | Extra API calls | Use haiku model (or local LLM if available) with max_tokens=10. Skip if naming disabled in config. |

---

## Estimated Effort

| Phase | Parallelism | Estimated Time |
|-------|-------------|----------------|
| 2.0 — Foundation | 3 agents | 3-4 hours |
| 2.1 — Core + Sessions | 4 agents | 5-7 hours |
| 2.2 — Skills | 3 agents | 2-3 hours |
| 2.3 — Integration | Sequential | 2-3 hours |
| **Total** | | **12-17 hours agent time** |

---

## Appendix: Module ↔ File Map

| Module | New Files | Modified Files |
|--------|-----------|----------------|
| 10 — Self-Improvement | `engine/src/self_improve.rs`, `engine/src/checkpoint.rs`, `skills/built-in/self-improve/` | `engine/src/lib.rs`, `engine/src/types.rs`, kickstart |
| 11 — Skill Repository | `engine/src/skill_installer.rs`, `engine/src/skill_registry.rs`, `skills/built-in/skill-manager/` | `engine/src/skill_loader.rs`, `engine/src/config.rs` |
| 12 — Split-View | `chat-shell/src/content_panel.rs`, `chat-shell/src/content_panel/` (image, text, diff, progress) | `chat-shell/src/window.rs`, `chat-shell/src/keybindings.rs`, `engine/src/types.rs` |
| 13 — API Key Config | `chat-shell/src/key_entry.rs` | `engine/src/config.rs`, `engine/src/api_client.rs`, `engine/src/lib.rs`, `base/overlay/etc/levsha/config.toml` |
| 14 — Local LLM | `engine/src/openai_client.rs`, `engine/src/router.rs`, `engine/src/model_manager.rs`, `base/overlay/etc/systemd/system/llama-server.service`, `skills/built-in/model-manager/` | `engine/src/lib.rs`, `engine/src/config.rs`, `chat-shell/src/status_bar.rs`, kickstart |
| 15 — Filesystem | `skills/built-in/filesystem/` (skill.yaml, 15 tool JSONs, prompt) | None |
| 16 — Network Config | `skills/built-in/network-config/` (skill.yaml, 13 tool JSONs, prompt) | kickstart (new packages) |
| 17 — Text Editor | `skills/built-in/text-editor/` (skill.yaml, 11 tool JSONs, prompt), `engine/src/edit_state.rs` | `chat-shell/src/content_panel/text.rs` |
| 18 — Code Editor | `skills/built-in/code-editor/` (skill.yaml, 16 tool JSONs, prompt) | None |
| 19 — Multi-Session | `engine/src/session.rs`, `chat-shell/src/session_list.rs` | `engine/src/context.rs`, `engine/src/persistence.rs`, `engine/src/config.rs`, `engine/src/types.rs`, `chat-shell/src/window.rs`, `chat-shell/src/keybindings.rs`, `chat-shell/src/message_widget.rs`, `chat-shell/src/status_bar.rs`, `base/overlay/etc/levsha/config.toml` |

**Total new files:** ~125
**Total modified files:** ~35
