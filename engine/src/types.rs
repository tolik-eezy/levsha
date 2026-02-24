//! Core types shared between engine and chat-shell.

use serde::{Deserialize, Serialize};

/// Messages sent from Chat Shell (L3) to Engine (L2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellToEngine {
    /// User typed a message.
    UserMessage { content: String },
    /// User approved or rejected a destructive command.
    ConfirmResponse { request_id: String, approved: bool },
    /// User requested cancellation of the current streaming response.
    CancelStream,
    /// User submitted an API key (from first-boot or inline key entry).
    SubmitApiKey { key: String },
    /// User requested switching to a different model.
    SwitchModel { model_alias: String },
    /// Switch LLM backend routing mode (Track D).
    SwitchBackend { mode: String },
    /// Create a new session (Track J).
    SessionCreate { name: Option<String> },
    /// Switch to an existing session (Track J).
    SessionSwitch { session_id: String },
    /// List all sessions (Track J).
    SessionList,
    /// Rename a session (Track J).
    SessionRename { session_id: String, name: String },
    /// Archive a session (Track J).
    SessionArchive { session_id: String },
    /// Delete a session (Track J).
    SessionDelete { session_id: String },
    /// Switch to the next session (Track J).
    SessionNext,
    /// Switch to the previous session (Track J).
    SessionPrev,
    /// User toggled sidebar visibility (Module 26).
    SidebarToggle { visible: bool },
}

/// Messages sent from Engine (L2) to Chat Shell (L3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineToShell {
    /// A chunk of streaming text from the LLM.
    StreamChunk { content: String },
    /// The stream has ended. The response is complete.
    StreamEnd,
    /// A destructive command needs user confirmation.
    ConfirmRequest {
        request_id: String,
        command: String,
        description: String,
    },
    /// A tool execution status update.
    ToolStatus {
        tool_name: String,
        status: ToolExecutionStatus,
        description: String,
    },
    /// An error occurred.
    Error {
        message: String,
        retryable: bool,
    },
    /// Connection status changed.
    ConnectionStatus {
        connected: bool,
        backend: String,
    },
    /// History messages for restoring the chat on startup.
    HistoryMessage {
        role: MessageRole,
        content: String,
        timestamp: i64,
    },
    /// Engine requires an API key before it can proceed.
    KeyRequired,
    /// Result of API key validation.
    KeyValidationResult {
        success: bool,
        error_message: Option<String>,
    },
    /// The active model has changed.
    ModelChanged {
        model_id: String,
        display_name: String,
    },
    /// Engine requests the shell to show the key change UI.
    RequestKeyChange,
    /// Open a content panel with the given content.
    ContentOpen {
        content_id: String,
        content_type: ContentType,
        title: String,
        content: String,
    },
    /// Update content in the currently open content panel.
    ContentUpdate {
        content_id: String,
        data: ContentUpdateData,
    },
    /// Close the content panel.
    ContentClose {
        content_id: String,
    },
    /// Backend routing status (Track D).
    BackendStatus {
        mode: String,
        active: String,
        local_available: bool,
        cloud_available: bool,
    },
    /// Session switched (Track J).
    SessionSwitched { session: SessionInfo },
    /// Session list response (Track J).
    SessionList { sessions: Vec<SessionInfo> },
    /// Session created (Track J).
    SessionCreated { session: SessionInfo },
    /// Session renamed (Track J).
    SessionRenamed { session_id: String, name: String },
    /// Session archived (Track J).
    SessionArchived { session_id: String },
    /// Session deleted (Track J).
    SessionDeleted { session_id: String },
}

/// Status of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolExecutionStatus {
    Started,
    Completed { success: bool },
}

/// Role of a message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    ToolCall,
    ToolResult,
}

/// Information about an available model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub alias: String,
    pub model_id: String,
    pub display_name: String,
}

/// Content type for the content panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    FilePreview {
        file_path: String,
        language: String,
    },
    DiffView {
        file_path: String,
        old_content: String,
        new_content: String,
    },
    ImageView {
        file_path: String,
        mime_type: String,
        data: String,
    },
    ProgressView {
        operation: String,
        total: Option<u64>,
    },
    AgentActivity {
        operation: String,
    },
}

/// Update data for an existing content panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentUpdateData {
    ReplaceText {
        text: String,
    },
    AppendText {
        text: String,
    },
    Progress {
        current: u64,
        message: String,
    },
    ReplaceDiff {
        old_content: String,
        new_content: String,
    },
    AgentEvent {
        event_type: String,
        content: String,
        file_path: Option<String>,
    },
}

/// Information about a session (Track J).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub updated_at: i64,
    pub message_count: i32,
    pub is_archived: bool,
    pub last_message_preview: Option<String>,
    pub is_active: bool,
}
