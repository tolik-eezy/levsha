//! Multi-session management (Track J).
//!
//! Provides session persistence (SQLite) and session lifecycle management.
//! Each session has its own conversation history, name, and metadata.

pub mod manager;
pub mod store;

pub use manager::SessionManager;
pub use store::SessionStore;
