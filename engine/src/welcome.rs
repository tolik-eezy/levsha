//! Welcome message logic.
//!
//! On first launch (empty DB): insert welcome message as first assistant message.
//! On subsequent boots: restore conversation from DB (no repeat welcome).
//! After history clear: re-show welcome.

use crate::history::History;
use crate::types::{EngineToShell, MessageRole};
use tokio::sync::mpsc;
use tracing::info;

/// The welcome message displayed on first boot.
pub const WELCOME_TEXT: &str = "\
Welcome. I'm your operating system.
Everything you need \u{2014} just ask.

I can manage your packages and tell you about your system.
More skills are coming soon.";

/// Check if this is a first launch for the active session and send the welcome
/// message, or restore the session's existing history to the shell.
pub async fn check_and_send_welcome(
    history: &History,
    sender: &mpsc::Sender<EngineToShell>,
    session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if history.is_session_empty(session_id)? {
        // Empty session — insert and send welcome message.
        info!("Empty session '{}', sending welcome message", session_id);

        history.insert_message_for_session(
            &MessageRole::Assistant,
            WELCOME_TEXT,
            None,
            None,
            None,
            Some("complete"),
            session_id,
        )?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sender
            .send(EngineToShell::HistoryMessage {
                role: MessageRole::Assistant,
                content: WELCOME_TEXT.to_string(),
                timestamp,
            })
            .await
            .ok();
    } else {
        // Restore session history.
        info!("Restoring history for session '{}'", session_id);

        send_session_history(history, sender, session_id).await?;
    }

    Ok(())
}

/// Send all user/assistant messages for a session to the shell as HistoryMessage events.
pub async fn send_session_history(
    history: &History,
    sender: &mpsc::Sender<EngineToShell>,
    session_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let messages = history.get_messages_for_session(session_id)?;
    let mut count = 0;
    for msg in &messages {
        // Only send user and assistant messages to the shell for display.
        // Tool calls/results are internal context.
        match msg.role {
            MessageRole::User | MessageRole::Assistant => {
                sender
                    .send(EngineToShell::HistoryMessage {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                        timestamp: msg.timestamp,
                    })
                    .await
                    .ok();
                count += 1;
            }
            _ => {}
        }
    }

    info!("Sent {} history messages for session '{}'", count, session_id);
    Ok(())
}
