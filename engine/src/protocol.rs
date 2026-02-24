//! L2↔L3 IPC protocol.
//!
//! In MVP, the engine is embedded as a library inside the chat-shell binary.
//! Communication uses tokio::mpsc channels — no serialization overhead.

use crate::types::{EngineToShell, ShellToEngine};
use tokio::sync::mpsc;

/// Channel capacity for engine-to-shell messages.
pub const ENGINE_TO_SHELL_CAPACITY: usize = 256;

/// Channel capacity for shell-to-engine messages.
pub const SHELL_TO_ENGINE_CAPACITY: usize = 64;

/// Create a bidirectional channel pair for L2↔L3 communication.
pub fn create_channels() -> (ProtocolShellSide, ProtocolEngineSide) {
    let (shell_tx, shell_rx) = mpsc::channel::<ShellToEngine>(SHELL_TO_ENGINE_CAPACITY);
    let (engine_tx, engine_rx) = mpsc::channel::<EngineToShell>(ENGINE_TO_SHELL_CAPACITY);

    let shell_side = ProtocolShellSide {
        sender: shell_tx,
        receiver: engine_rx,
    };

    let engine_side = ProtocolEngineSide {
        sender: engine_tx,
        receiver: shell_rx,
    };

    (shell_side, engine_side)
}

/// The Chat Shell's end of the protocol.
pub struct ProtocolShellSide {
    /// Send messages to the engine (UserMessage, ConfirmResponse, CancelStream).
    pub sender: mpsc::Sender<ShellToEngine>,
    /// Receive messages from the engine (StreamChunk, ConfirmRequest, etc.).
    pub receiver: mpsc::Receiver<EngineToShell>,
}

/// The Engine's end of the protocol.
pub struct ProtocolEngineSide {
    /// Send messages to the shell (StreamChunk, ConfirmRequest, etc.).
    pub sender: mpsc::Sender<EngineToShell>,
    /// Receive messages from the shell (UserMessage, ConfirmResponse, CancelStream).
    pub receiver: mpsc::Receiver<ShellToEngine>,
}
