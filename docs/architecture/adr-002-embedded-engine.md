# ADR-002: Engine Embedded as Rust Library

**Status:** Accepted
**Date:** 2025-01

---

## Context

Levsha OS has two main subsystems: the Chat Shell (L3, GUI) and the Intelligence Engine (L2, LLM + tool execution). They need to communicate bidirectionally:

- Shell sends user messages to the engine
- Engine streams LLM responses back to the shell
- Engine sends confirmation requests for destructive commands
- Shell sends confirmation responses

We need to choose an IPC mechanism and deployment model.

## Options Considered

### 1. Embedded Rust library (tokio::mpsc channels)

The engine is a Rust library crate (`levsha-engine`) that the chat shell binary depends on. Communication uses `tokio::mpsc` channels -- typed Rust enums passed between an async engine thread and the GTK main loop.

**Pros:**
- Single binary deployment -- one file to install, no process management
- Zero-copy IPC -- Rust enums moved between threads, no serialization
- Compile-time type safety for all messages
- Simplest debugging -- one process, one log stream
- No network overhead, no socket setup

**Cons:**
- Engine lifecycle tied to GUI -- if one crashes, both crash
- Cannot restart engine independently
- Harder to replace engine implementation later

### 2. Separate process with Unix domain socket

Engine runs as a separate binary/service. Communication over a Unix socket with a serialized protocol (e.g., JSON lines or MessagePack).

**Pros:**
- Independent process lifecycle -- can restart engine without restarting GUI
- Language-agnostic -- engine could be rewritten in Python or another language
- Process isolation for security

**Cons:**
- Two binaries to deploy and manage
- Serialization/deserialization overhead for every message (especially streaming tokens)
- Need to handle socket lifecycle, reconnection, process supervision
- More complex error handling and startup sequencing

### 3. D-Bus

Use the system or session D-Bus for IPC.

**Pros:**
- Standard Linux IPC, well-integrated with systemd
- Language-agnostic
- Built-in service activation

**Cons:**
- High overhead for streaming (D-Bus is not designed for high-frequency messages)
- Complex API surface for simple request/response patterns
- Adds a dependency on the D-Bus daemon
- Verbose boilerplate for Rust bindings

### 4. gRPC

Use gRPC with HTTP/2 streaming for bidirectional communication.

**Pros:**
- Excellent streaming support
- Well-defined service contracts via protobuf
- Language-agnostic

**Cons:**
- Heavy dependency (tonic + prost + hyper)
- Network overhead (HTTP/2 frames) for local-only communication
- Overkill for two components on the same machine

## Decision

**Embedded Rust library with tokio::mpsc channels.** For an MVP where both components are written in Rust and run on the same machine, this is the simplest correct solution. The channel-based approach gives us:

- 3 message types Shell-to-Engine: `UserMessage`, `ConfirmResponse`, `CancelStream`
- 7 message types Engine-to-Shell: `StreamChunk`, `StreamEnd`, `ConfirmRequest`, `ToolStatus`, `Error`, `ConnectionStatus`, `HistoryMessage`

Channel capacities: 64 (shell to engine), 256 (engine to shell -- higher for streaming tokens).

The full protocol is documented in [L2-L3 Protocol](l2-l3-protocol.md).

## Consequences

- **Single binary:** `levsha-chat` contains both the GUI and the engine. Deployment is copying one file to `/usr/bin/`.
- **Coupled lifecycle:** If the engine panics, the entire application exits. The `levsha-chat-watchdog` systemd service restarts it.
- **No independent scaling:** Cannot run multiple engine instances or offload engine work to another machine. This is acceptable for an OS that runs on a single machine.
- **Future migration path:** If we later need process isolation (e.g., for sandboxing tool execution), the typed message enums can be serialized over a Unix socket with minimal changes -- the `ShellToEngine` and `EngineToShell` enums already derive `Serialize`/`Deserialize`.
