//! Streaming renderer — manages stream state transitions.

/// The current state of streaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    /// No active stream. Input is enabled.
    Idle,
    /// User sent a message. Waiting for first token. Typing indicator shown.
    Waiting,
    /// Receiving tokens. Input disabled.
    Streaming,
}
