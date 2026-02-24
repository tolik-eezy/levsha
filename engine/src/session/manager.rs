//! Session lifecycle management (Track J).
//!
//! Provides high-level session operations: create, switch, list, rename,
//! archive, and delete. Tracks the currently active session.

use crate::session::store::SessionStore;
use crate::types::SessionInfo;
use tracing::info;

/// Manages session lifecycle and tracks the active session.
pub struct SessionManager {
    store: SessionStore,
    active_session_id: String,
}

impl SessionManager {
    /// Create a new session manager. Ensures a default session exists.
    pub fn new(
        store: SessionStore,
        default_name: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let active_id = store.ensure_default_session(default_name)?;
        info!("Session manager initialized, active session: {}", active_id);

        Ok(Self {
            store,
            active_session_id: active_id,
        })
    }

    /// Create a new session and optionally switch to it.
    pub fn create(
        &mut self,
        name: Option<String>,
        default_name: &str,
    ) -> Result<SessionInfo, Box<dyn std::error::Error>> {
        let id = uuid::Uuid::new_v4().to_string();
        let name = name.unwrap_or_else(|| default_name.to_string());

        let mut session = self.store.create_session(&id, &name)?;
        session.is_active = true;

        // Switch to the new session.
        self.active_session_id = id.clone();
        info!("Created and switched to new session: {} ({})", name, id);

        Ok(session)
    }

    /// Switch to an existing session.
    pub fn switch_to(
        &mut self,
        session_id: &str,
    ) -> Result<SessionInfo, Box<dyn std::error::Error>> {
        let session = self
            .store
            .get_session(session_id)?
            .ok_or_else(|| format!("Session '{}' not found", session_id))?;

        if session.is_archived {
            return Err(format!("Session '{}' is archived", session_id).into());
        }

        self.active_session_id = session_id.to_string();
        self.store.update_activity(session_id)?;

        let mut info = session;
        info.is_active = true;

        info!("Switched to session: {} ({})", info.name, session_id);
        Ok(info)
    }

    /// Get the active session ID.
    pub fn active_session_id(&self) -> &str {
        &self.active_session_id
    }

    /// List all sessions, marking the active one.
    pub fn list(&self) -> Result<Vec<SessionInfo>, Box<dyn std::error::Error>> {
        let mut sessions = self.store.list_sessions()?;

        for session in &mut sessions {
            session.is_active = session.id == self.active_session_id;
        }

        Ok(sessions)
    }

    /// Rename a session.
    pub fn rename(
        &self,
        session_id: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.store.rename_session(session_id, name)
    }

    /// Archive a session.
    pub fn archive(
        &mut self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if session_id == self.active_session_id {
            return Err("Cannot archive the active session".into());
        }
        self.store.archive_session(session_id)
    }

    /// Delete a session.
    pub fn delete(
        &mut self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if session_id == self.active_session_id {
            return Err("Cannot delete the active session".into());
        }
        self.store.delete_session(session_id)
    }

    /// Get a reference to the underlying store.
    pub fn store(&self) -> &SessionStore {
        &self.store
    }
}
