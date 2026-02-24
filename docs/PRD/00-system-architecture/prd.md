# 00 — System Architecture: Product Requirements

**Module:** System Architecture (4-Layer Model)
**Phase:** 1 (MVP)
**Status:** Draft

---

## 1. Overview

Levsha OS is built on a strict 4-layer architecture where each layer has a single, well-defined responsibility. The architecture enforces the core thesis: the chat is the only interface, and everything below it exists solely to serve the conversation.

This document defines the product requirements for the overall system architecture — how the layers are structured, how they communicate, and the constraints that govern their interactions.

---

## 2. Guiding Principles (Architecture-Relevant)

| Principle | Architectural Implication |
|---|---|
| **Chat-First, Chat-Only** | L3 (Chat Shell) is the sole user-facing surface. No TTY, no fallback shell, no escape hatch. |
| **Radically Minimal** | Each layer includes only what is strictly necessary. L1 boots, renders, and connects. L2 thinks. L3 shows. |
| **Beautiful by Default** | L3 owns all visual output. L1 must provide a display stack (Wayland) capable of 60fps rendering. |
| **AI Quality is Non-Negotiable** | L2 uses a cloud API (Anthropic Claude) to guarantee response quality. No local model in MVP. |
| **Skills, Not Apps** | L2 orchestrates skills. Skills are not standalone processes — they are prompt fragments and tool schemas that L2 injects into the LLM context. |

---

## 3. Layer Definitions

### L0 — Kernel

| Attribute | Value |
|---|---|
| Component | Linux kernel (Fedora default) |
| Responsibility | Hardware abstraction, process scheduling, memory management, device drivers |
| MVP scope | Fedora's default kernel with minimal module config. No custom patches. |

### L1 — Base System

| Attribute | Value |
|---|---|
| Component | Fedora minimal + systemd + Wayland compositor + networking |
| Responsibility | Boot sequence, service management, filesystem, display server, audio/video subsystem, network (DHCP) |
| MVP scope | Auto-login, auto-DHCP, Wayland compositor running, Chat Shell launched as default session |

### L2 — Intelligence Engine

| Attribute | Value |
|---|---|
| Component | LLM engine: intent parsing, skill dispatch, tool calling |
| Responsibility | Interpret user intent, select and invoke skills, execute system commands via tool calling, manage conversation context |
| MVP scope | Cloud API only (Anthropic Claude). Hardcoded API key. Two built-in skills (package manager, system info). |

### L3 — Chat Shell

| Attribute | Value |
|---|---|
| Component | Wayland-native full-screen chat GUI (Rust) |
| Responsibility | Render conversation, capture user input, display streaming responses, show status bar |
| MVP scope | Single session, light theme (see theme.design.md), streaming, code blocks, syntax highlighting, tables, progress indicators |

---

## 4. Interaction Flow

```
User types message
       |
       v
+------------------+
|   L3 Chat Shell  |  Captures input, sends to L2
+--------+---------+
         |
         v
+------------------+
|  L2 Intelligence |  Parses intent, builds API request with
|     Engine       |  system prompt + skill prompts + history,
|                  |  sends to Claude API, receives response
+--------+---------+
         |
         v  (if tool call required)
+------------------+
|  L1 Base System  |  Executes command (dnf, df, ps, etc.),
|                  |  returns stdout/stderr to L2
+--------+---------+
         |
         v
   L2 incorporates tool output, may make
   additional API calls, then sends final
   response back to L3 for rendering
```

**Key rules:**
- L3 never talks to L1 directly. All system interaction is mediated by L2.
- L2 is the only layer that makes network calls to the cloud API.
- L1 exposes system capabilities via standard Linux commands. L2 invokes them through tool calling.
- L0 is invisible to all layers above L1.

---

## 5. Functional Requirements

| ID | Requirement | Priority |
|---|---|---|
| SA-01 | The system boots from L0 through L1 into L3 without any user interaction (no login, no setup wizard). | P0 |
| SA-02 | L3 communicates with L2 via a local IPC mechanism (Unix socket, D-Bus, or direct library call). No network hop between L3 and L2. | P0 |
| SA-03 | L2 communicates with the Claude API over HTTPS. Connection status is reported to L3 for display in the status bar. | P0 |
| SA-04 | L2 executes system commands on L1 with full root access. No sandboxing. | P0 |
| SA-05 | If L3 crashes, systemd restarts it automatically. Conversation history is not lost (persisted in SQLite by L2). | P0 |
| SA-06 | If L2 cannot reach the API, L3 displays a styled error message with a retry option. No fallback to a terminal. | P0 |
| SA-07 | Destructive commands detected by L2 require explicit user confirmation rendered by L3 as a visually distinct prompt. | P0 |
| SA-08 | The full stack (L0 through L3) fits within 2 GB RAM and 8 GB disk in VM. | P0 |
| SA-09 | Boot to interactive chat takes under 30 seconds in VM (VirtualBox/QEMU). | P0 |
| SA-10 | L3 renders at a consistent 60fps for scrolling and animations. | P0 |

---

## 6. Non-Functional Requirements

| Requirement | Target |
|---|---|
| System idle RAM | < 1 GB |
| Disk footprint (installed) | < 4 GB |
| Boot to chat (cold start, VM) | < 30 seconds |
| First-token latency (API) | < 1 second |
| GUI frame rate | >= 60 fps |
| Crash recovery (Chat Shell) | Automatic restart via systemd, < 3 seconds |

---

## 7. Acceptance Criteria

1. **Boots to chat:** A freshly built ISO boots in QEMU to the Chat Shell GUI with no user interaction required.
2. **End-to-end flow works:** User types a message in L3, L2 sends it to the Claude API, receives a response, and L3 renders it with streaming.
3. **Tool calling works:** User asks "how much disk space do I have?", L2 invokes `df` on L1, and L3 renders the formatted output.
4. **No escape hatch:** There is no way to reach a TTY, terminal, or shell outside the chat. Ctrl+Alt+F2 does nothing useful.
5. **Crash resilience:** Killing the Chat Shell process causes systemd to restart it. Conversation history is intact after restart.
6. **Error handling:** Disconnecting the VM's network adapter causes L3 to display a styled error, not a crash or blank screen.
7. **Performance:** Boot time, RAM usage, and frame rate meet the targets in section 6.

---

## 8. Out of Scope (MVP)

- Self-improvement / self-modification (Phase 2)
- Multiple chat sessions (Phase 2)
- Local LLM support (Phase 2)
- Theming (Phase 2, via self-improvement)
- Voice input/output (Phase 3)
- Skill marketplace (Phase 3)
- Bare-metal installation (Phase 4)
