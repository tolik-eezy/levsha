//! Levsha OS Intelligence Engine (L2)
//!
//! The engine interprets user intent, dispatches system calls through skills,
//! manages conversation context, and streams responses back to the Chat Shell.

pub mod api_client;
pub mod config;
pub mod context;
pub mod history;
pub mod llm;
pub mod protocol;
pub mod risk_classifier;
pub mod self_improve;
pub mod session;
pub mod skill_installer;
pub mod skill_loader;
pub mod skill_registry;
pub mod tool_executor;
pub mod types;
pub mod edit_state;
pub mod welcome;

use api_client::{ApiClient, ApiTool, MessageRequest, StreamResult, ToolCall};
use config::Config;
use context::ContextBuilder;
use history::History;
use llm::anthropic::AnthropicBackend;
use llm::openai_compat::OpenAiCompatBackend;
use llm::router::{Router, RoutingMode};
use llm::LlmBackend;
use protocol::ProtocolEngineSide;
use self_improve::SelfImproveManager;
use session::SessionManager;
use skill_installer::SkillInstaller;
use skill_loader::LoadedSkills;
use skill_registry::RegistryClient;
use tool_executor::{DestructiveGuard, ToolExecutor};
use types::{EngineToShell, MessageRole, ShellToEngine, ToolExecutionStatus};

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// System prompt template. Template variables: {{datetime}}.
const SYSTEM_PROMPT: &str = "\
You are Levsha OS, a minimal Linux operating system where the chat is the \
entire user interface. You are running on Fedora Linux.

Current time: {{datetime}}

## Behavior

- You ARE the operating system. Speak as the system, not an assistant.
- Execute tasks using the provided tools. Do not guess at output.
- Be concise. Use tables and code blocks for structured output.
- When a command fails, report the error clearly and suggest fixes.
- Never refuse to execute a command \u{2014} but destructive commands will \
require user confirmation automatically.";

/// Default registry URL for the skill repository.
const DEFAULT_REGISTRY_URL: &str = "https://github.com/levsha-os/skill-registry.git";

/// The main Engine struct — orchestrates all subsystems.
pub struct Engine {
    config: Config,
    sender: mpsc::Sender<EngineToShell>,
    receiver: mpsc::Receiver<ShellToEngine>,
    history: History,
    skills: LoadedSkills,
    router: Router,
    context_builder: ContextBuilder,
    tool_executor: ToolExecutor,
    destructive_guard: DestructiveGuard,
    /// Skill installer for managing third-party skills (Track C).
    skill_installer: SkillInstaller,
    /// Registry client for discovering skills (Track C).
    registry_client: RegistryClient,
    /// Self-improvement manager (Track E).
    self_improve_mgr: SelfImproveManager,
    /// Session manager (Track J).
    session_mgr: SessionManager,
    /// Active cancellation token for the current streaming request.
    current_cancel: Option<CancellationToken>,
    /// Pending confirmation requests: request_id -> oneshot sender.
    pending_confirmations: HashMap<String, oneshot::Sender<bool>>,
    /// In-memory text editor state (Track H).
    edit_state: Option<edit_state::EditState>,
    /// When true, skip the next LLM iteration after self_improve completes.
    skip_next_llm_iteration: bool,
}

impl Engine {
    /// Create a new Engine from config and protocol channel.
    pub fn new(
        mut config: Config,
        engine_side: ProtocolEngineSide,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Open history database.
        let history = History::open(&config.persistence.db_path)?;

        // Determine skill paths.
        let builtin_path = PathBuf::from(&config.skills.path);
        let installed_path = config
            .skills
            .installed_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/lib/levsha/skills"));
        let cache_path = config
            .skills
            .cache_path
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/var/cache/levsha"));

        // Load skills from both builtin and installed directories.
        let builtin_str = builtin_path.to_string_lossy().to_string();
        let installed_str = installed_path.to_string_lossy().to_string();
        let skills =
            skill_loader::load_skills_multi(&[&builtin_str, &installed_str]);

        // When DeepSeek is enabled, override api.model so the request pipeline
        // sends the correct model ID (e.g. "deepseek-chat" instead of "claude-sonnet-...").
        if config.deepseek.enabled {
            config.api.model = config.deepseek.model.clone();
        }

        // Create LLM backends and router (Track D).
        let cloud_backend: Arc<dyn LlmBackend> = if config.deepseek.enabled {
            Arc::new(OpenAiCompatBackend::new_with_auth(
                &config.deepseek.base_url,
                &config.deepseek.key,
                "DeepSeek",
            ))
        } else {
            Arc::new(AnthropicBackend::new(
                ApiClient::new(
                    &config.api.base_url,
                    &config.api.key,
                    &config.api.model,
                    config.api.timeout_seconds,
                    config.api.max_retries,
                ),
                config.current_display_name(),
                config.has_key(),
            ))
        };

        let local_backend: Option<Arc<dyn LlmBackend>> = if config.local_model.enabled {
            let base_url = format!("http://localhost:{}", config.local_model.port);
            Some(Arc::new(OpenAiCompatBackend::new(&base_url, "Local LLM")))
        } else {
            None
        };

        let routing_mode = RoutingMode::from_str(&config.routing.mode);
        let router = Router::new(cloud_backend, local_backend, routing_mode);

        // Create context builder.
        let context_builder = ContextBuilder::new(
            config.context.max_tokens,
            config.context.response_reserve,
            config.context.chars_per_token,
        );

        // Create tool executor.
        let tool_executor = ToolExecutor::new(Duration::from_secs(
            config.execution.command_timeout_seconds,
        ));

        // Create destructive guard.
        let destructive_guard = DestructiveGuard::new();

        // Create skill installer.
        let staging_path = cache_path.join("staging");
        let skill_installer =
            SkillInstaller::new(builtin_path, installed_path, staging_path);

        // Create registry client.
        let registry_url = config
            .skills
            .registry_url
            .as_deref()
            .unwrap_or(DEFAULT_REGISTRY_URL);
        let registry_client = RegistryClient::new(registry_url, cache_path);

        // Create self-improvement manager (Track E).
        let agent_api_key = if config.deepseek.enabled {
            Some(config.deepseek.key.clone())
        } else {
            Some(config.api.key.clone())
        };
        let agent_api_key_env = if config.deepseek.enabled {
            "DEEPSEEK_API_KEY".to_string()
        } else {
            config.self_improve.agent_api_key_env.clone()
        };
        let agent_config = self_improve::CodingAgentConfig {
            backend: self_improve::CodingAgentBackend::from_str(
                &config.self_improve.agent_backend,
            ),
            binary_path: PathBuf::from(&config.self_improve.agent_binary),
            source_root: PathBuf::from(&config.self_improve.source_root),
            timeout: Duration::from_secs(config.self_improve.agent_timeout),
            max_turns: config.self_improve.agent_max_turns,
            model: config.self_improve.agent_model.clone(),
            api_key: agent_api_key,
            api_key_env: agent_api_key_env,
        };
        let self_improve_mgr = SelfImproveManager::new(
            &config.self_improve.source_root,
            &config.self_improve.checkpoint_dir,
            config.self_improve.max_checkpoints,
            agent_config,
        );

        // Create session manager (Track J).
        let session_store = session::SessionStore::open(&config.persistence.db_path)?;
        let session_mgr =
            SessionManager::new(session_store, &config.sessions.default_name)?;

        Ok(Self {
            config,
            sender: engine_side.sender,
            receiver: engine_side.receiver,
            history,
            skills,
            router,
            context_builder,
            tool_executor,
            destructive_guard,
            skill_installer,
            registry_client,
            self_improve_mgr,
            session_mgr,
            current_cancel: None,
            pending_confirmations: HashMap::new(),
            edit_state: None,
            skip_next_llm_iteration: false,
        })
    }

    /// Run the engine main loop. This consumes the Engine.
    pub async fn run(mut self) {
        info!("Engine starting");

        // Check if we have a valid API key.
        if !self.config.has_key() {
            // No cloud key — check if local LLM is available before blocking.
            let local_available = self.router.check_local_available().await;
            if local_available {
                info!("No API key configured, but local LLM available — switching to local mode");
                self.router.set_mode(crate::llm::router::RoutingMode::LocalOnly);
            } else {
                // Neither cloud nor local available — enter key setup flow.
                info!("No API key configured and no local LLM, entering key setup flow");
                self.sender.send(EngineToShell::KeyRequired).await.ok();

                if !self.wait_for_key_setup().await {
                    info!("Key setup failed or channel closed, shutting down");
                    return;
                }
            }
        }

        // Send connection status.
        let local_available = self.router.check_local_available().await;
        let cloud_available = self.config.has_key();
        let active_display = if cloud_available {
            self.config.current_display_name()
        } else if local_available {
            "Local LLM".to_string()
        } else {
            "None".to_string()
        };

        self.sender
            .send(EngineToShell::ConnectionStatus {
                connected: cloud_available || local_available,
                backend: active_display.clone(),
            })
            .await
            .ok();

        // Send backend status (Track D).
        self.sender
            .send(EngineToShell::BackendStatus {
                mode: self.router.current_mode().as_str().to_string(),
                active: active_display,
                local_available,
                cloud_available,
            })
            .await
            .ok();

        // Send welcome or restore history for the active session.
        let active_id = self.session_mgr.active_session_id().to_string();
        if let Err(e) =
            welcome::check_and_send_welcome(&self.history, &self.sender, &active_id).await
        {
            error!("Failed to send welcome/history: {}", e);
        }

        info!("Engine ready, entering main loop");

        // Main event loop.
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ShellToEngine::UserMessage { content } => {
                    self.handle_user_message(content).await;
                }
                ShellToEngine::ConfirmResponse {
                    request_id,
                    approved,
                } => {
                    self.handle_confirm_response(request_id, approved);
                }
                ShellToEngine::CancelStream => {
                    self.handle_cancel();
                }
                ShellToEngine::SubmitApiKey { key } => {
                    self.handle_key_change(key).await;
                }
                ShellToEngine::SwitchModel { model_alias } => {
                    self.handle_model_switch(model_alias).await;
                }
                ShellToEngine::SwitchBackend { mode } => {
                    self.handle_backend_switch(mode).await;
                }
                ShellToEngine::SessionCreate { name } => {
                    self.handle_session_create(name).await;
                }
                ShellToEngine::SessionSwitch { session_id } => {
                    self.handle_session_switch(session_id).await;
                }
                ShellToEngine::SessionList => {
                    self.handle_session_list().await;
                }
                ShellToEngine::SessionRename { session_id, name } => {
                    self.handle_session_rename(session_id, name).await;
                }
                ShellToEngine::SessionArchive { session_id } => {
                    self.handle_session_archive(session_id).await;
                }
                ShellToEngine::SessionDelete { session_id } => {
                    self.handle_session_delete(session_id).await;
                }
                ShellToEngine::SessionNext => {
                    self.handle_session_cycle(true).await;
                }
                ShellToEngine::SessionPrev => {
                    self.handle_session_cycle(false).await;
                }
                ShellToEngine::SidebarToggle { visible } => {
                    self.handle_sidebar_toggle(visible).await;
                }
            }
        }

        info!("Engine shutting down (channel closed)");
    }

    /// Wait for the user to submit a valid API key during initial setup.
    /// Returns true if a valid key was received and configured.
    async fn wait_for_key_setup(&mut self) -> bool {
        loop {
            let msg = match self.receiver.recv().await {
                Some(msg) => msg,
                None => return false, // Channel closed.
            };

            match msg {
                ShellToEngine::SubmitApiKey { key } => {
                    info!("Received API key submission, validating...");

                    let timeout = Duration::from_secs(self.config.api.timeout_seconds);
                    match ApiClient::validate_key(&self.config.api.base_url, &key, timeout)
                        .await
                    {
                        Ok(()) => {
                            info!("API key validated successfully");

                            // Save to config file if we know the path.
                            if let Some(config_path) = &self.config.config_path {
                                if let Err(e) =
                                    Config::save_api_key(&key, config_path)
                                {
                                    warn!("Failed to save API key to config: {}", e);
                                }
                            }

                            // Update in-memory config and cloud backend.
                            self.config.api.key = key.clone();
                            self.router.cloud_backend().set_api_key(&key).await;

                            self.sender
                                .send(EngineToShell::KeyValidationResult {
                                    success: true,
                                    error_message: None,
                                })
                                .await
                                .ok();

                            return true;
                        }
                        Err(e) => {
                            warn!("API key validation failed: {}", e);
                            self.sender
                                .send(EngineToShell::KeyValidationResult {
                                    success: false,
                                    error_message: Some(e.user_message()),
                                })
                                .await
                                .ok();
                            // Continue looping — wait for another key.
                        }
                    }
                }
                ShellToEngine::CancelStream => {
                    return false;
                }
                _ => {
                    // Ignore other messages during key setup.
                    debug!("Ignoring message during key setup: {:?}", msg);
                }
            }
        }
    }

    /// Handle a mid-session API key change.
    async fn handle_key_change(&mut self, key: String) {
        info!("Handling API key change request");

        let timeout = Duration::from_secs(self.config.api.timeout_seconds);
        match ApiClient::validate_key(&self.config.api.base_url, &key, timeout).await {
            Ok(()) => {
                // Save to config file.
                if let Some(config_path) = &self.config.config_path {
                    if let Err(e) = Config::save_api_key(&key, config_path) {
                        warn!("Failed to save API key to config: {}", e);
                    }
                }

                self.config.api.key = key.clone();
                self.router.cloud_backend().set_api_key(&key).await;

                self.sender
                    .send(EngineToShell::KeyValidationResult {
                        success: true,
                        error_message: None,
                    })
                    .await
                    .ok();
            }
            Err(e) => {
                self.sender
                    .send(EngineToShell::KeyValidationResult {
                        success: false,
                        error_message: Some(e.user_message()),
                    })
                    .await
                    .ok();
            }
        }
    }

    /// Handle a model switch request.
    async fn handle_model_switch(&mut self, alias: String) {
        info!("Handling model switch to alias '{}'", alias);

        let model_id = match self.config.resolve_model(&alias) {
            Some(id) => id,
            None => {
                // Try treating the alias as a raw model ID.
                alias.clone()
            }
        };

        // Save to config file.
        if let Some(config_path) = &self.config.config_path {
            if let Err(e) = Config::save_model(&model_id, config_path) {
                warn!("Failed to save model to config: {}", e);
            }
        }

        self.config.api.model = model_id.clone();
        self.router.cloud_backend().set_model(&model_id).await;

        let display_name = self.config.current_display_name();

        self.sender
            .send(EngineToShell::ModelChanged {
                model_id,
                display_name,
            })
            .await
            .ok();
    }

    /// Handle a backend switch request (Track D).
    async fn handle_backend_switch(&mut self, mode: String) {
        info!("Handling backend switch to mode '{}'", mode);
        let routing_mode = RoutingMode::from_str(&mode);
        self.router.set_mode(routing_mode);

        let local_available = self.router.check_local_available().await;
        self.sender
            .send(EngineToShell::BackendStatus {
                mode: self.router.current_mode().as_str().to_string(),
                active: self.config.current_display_name(),
                local_available,
                cloud_available: self.config.has_key(),
            })
            .await
            .ok();
    }

    // -----------------------------------------------------------------------
    // Session handlers (Track J)
    // -----------------------------------------------------------------------

    async fn handle_session_create(&mut self, name: Option<String>) {
        match self
            .session_mgr
            .create(name, &self.config.sessions.default_name)
        {
            Ok(session) => {
                self.sender
                    .send(EngineToShell::SessionCreated {
                        session: session.clone(),
                    })
                    .await
                    .ok();
                self.sender
                    .send(EngineToShell::SessionSwitched { session })
                    .await
                    .ok();
            }
            Err(e) => {
                self.sender
                    .send(EngineToShell::Error {
                        message: format!("Failed to create session: {}", e),
                        retryable: false,
                    })
                    .await
                    .ok();
            }
        }
    }

    async fn handle_session_switch(&mut self, session_id: String) {
        match self.session_mgr.switch_to(&session_id) {
            Ok(session) => {
                self.sender
                    .send(EngineToShell::SessionSwitched { session })
                    .await
                    .ok();

                // Send history messages for the switched-to session.
                if let Err(e) = welcome::send_session_history(
                    &self.history,
                    &self.sender,
                    &session_id,
                )
                .await
                {
                    error!("Failed to send session history: {}", e);
                }
            }
            Err(e) => {
                self.sender
                    .send(EngineToShell::Error {
                        message: format!("Failed to switch session: {}", e),
                        retryable: false,
                    })
                    .await
                    .ok();
            }
        }
    }

    async fn handle_session_list(&mut self) {
        match self.session_mgr.list() {
            Ok(sessions) => {
                self.sender
                    .send(EngineToShell::SessionList { sessions })
                    .await
                    .ok();
            }
            Err(e) => {
                self.sender
                    .send(EngineToShell::Error {
                        message: format!("Failed to list sessions: {}", e),
                        retryable: false,
                    })
                    .await
                    .ok();
            }
        }
    }

    async fn handle_session_rename(&mut self, session_id: String, name: String) {
        match self.session_mgr.rename(&session_id, &name) {
            Ok(()) => {
                self.sender
                    .send(EngineToShell::SessionRenamed { session_id, name })
                    .await
                    .ok();
            }
            Err(e) => {
                self.sender
                    .send(EngineToShell::Error {
                        message: format!("Failed to rename session: {}", e),
                        retryable: false,
                    })
                    .await
                    .ok();
            }
        }
    }

    async fn handle_session_archive(&mut self, session_id: String) {
        match self.session_mgr.archive(&session_id) {
            Ok(()) => {
                self.sender
                    .send(EngineToShell::SessionArchived {
                        session_id,
                    })
                    .await
                    .ok();
            }
            Err(e) => {
                self.sender
                    .send(EngineToShell::Error {
                        message: format!("Failed to archive session: {}", e),
                        retryable: false,
                    })
                    .await
                    .ok();
            }
        }
    }

    async fn handle_session_delete(&mut self, session_id: String) {
        match self.session_mgr.delete(&session_id) {
            Ok(()) => {
                self.sender
                    .send(EngineToShell::SessionDeleted { session_id })
                    .await
                    .ok();
            }
            Err(e) => {
                self.sender
                    .send(EngineToShell::Error {
                        message: format!("Failed to delete session: {}", e),
                        retryable: false,
                    })
                    .await
                    .ok();
            }
        }
    }

    /// Cycle to the next or previous session.
    async fn handle_session_cycle(&mut self, forward: bool) {
        let sessions = match self.session_mgr.list() {
            Ok(s) => s,
            Err(_) => return,
        };

        if sessions.len() <= 1 {
            return; // Nothing to cycle to.
        }

        let current_id = self.session_mgr.active_session_id().to_string();
        let current_idx = sessions
            .iter()
            .position(|s| s.id == current_id)
            .unwrap_or(0);

        let next_idx = if forward {
            (current_idx + 1) % sessions.len()
        } else {
            (current_idx + sessions.len() - 1) % sessions.len()
        };

        let next_id = sessions[next_idx].id.clone();
        self.handle_session_switch(next_id).await;
    }

    /// Handle sidebar toggle — persist visibility preference.
    async fn handle_sidebar_toggle(&self, visible: bool) {
        if let Some(ref path) = self.config.config_path {
            if let Err(e) = Config::save_sidebar_preference(visible, path) {
                tracing::warn!("Failed to save sidebar preference: {}", e);
            }
        }
    }

    /// Handle a user message: persist it, build context, call API, stream response.
    async fn handle_user_message(&mut self, content: String) {
        info!("Processing user message ({} chars)", content.len());

        // Check for special commands before sending to API.
        if self.try_handle_special_command(&content).await {
            return;
        }

        // Persist user message for the active session.
        let session_id = self.session_mgr.active_session_id().to_string();
        if let Err(e) = self.history.insert_message_for_session(
            &MessageRole::User,
            &content,
            None,
            None,
            None,
            None,
            &session_id,
        ) {
            error!("Failed to persist user message: {}", e);
        }

        // Update session activity.
        if let Err(e) = self.session_mgr.store().update_activity(&session_id) {
            warn!("Failed to update session activity: {}", e);
        }

        // Create a cancellation token for this request.
        let cancel = CancellationToken::new();
        self.current_cancel = Some(cancel.clone());

        // Process the conversation turn (may involve multiple API calls for tool use).
        if let Err(e) = self.process_turn(cancel).await {
            error!("Turn processing failed: {}", e);
            self.sender
                .send(EngineToShell::Error {
                    message: e.user_message(),
                    retryable: e.is_retryable_for_user(),
                })
                .await
                .ok();
            self.sender.send(EngineToShell::StreamEnd).await.ok();
        }

        self.current_cancel = None;
    }

    /// Check for special user commands like "change api key" or "use opus".
    /// Returns true if the message was handled as a special command.
    async fn try_handle_special_command(&mut self, content: &str) -> bool {
        let lower = content.to_lowercase();
        let lower = lower.trim();

        // API key change requests.
        if lower.contains("change api key")
            || lower.contains("change my api key")
            || lower.contains("update api key")
            || lower.contains("set api key")
        {
            self.sender.send(EngineToShell::RequestKeyChange).await.ok();
            return true;
        }

        // Model switch requests: "use opus", "use sonnet", "use haiku".
        let model_aliases = ["opus", "sonnet", "haiku"];
        for alias in &model_aliases {
            if lower == format!("use {}", alias)
                || lower == format!("switch to {}", alias)
            {
                self.handle_model_switch(alias.to_string()).await;
                return true;
            }
        }

        // Backend switch requests (Track D).
        if lower == "use local" || lower == "switch to local" {
            self.handle_backend_switch("local".to_string()).await;
            return true;
        }
        if lower == "use cloud" || lower == "switch to cloud" {
            self.handle_backend_switch("cloud".to_string()).await;
            return true;
        }
        if lower == "use auto" || lower == "switch to auto" {
            self.handle_backend_switch("auto".to_string()).await;
            return true;
        }

        // Session commands (Track J).
        if lower == "new session" || lower == "create session" {
            self.handle_session_create(None).await;
            return true;
        }
        if lower == "list sessions" || lower == "sessions" {
            self.handle_session_list().await;
            return true;
        }

        false
    }

    /// Process a complete conversation turn, potentially with multiple tool calls.
    async fn process_turn(&mut self, cancel: CancellationToken) -> Result<(), api_client::ApiError> {
        // Allow up to 10 tool-call iterations to prevent infinite loops.
        let max_iterations = 10;
        let session_id = self.session_mgr.active_session_id().to_string();

        for iteration in 0..max_iterations {
            // If self_improve just completed, stop the LLM from running another turn.
            if self.skip_next_llm_iteration {
                self.skip_next_llm_iteration = false;
                self.sender.send(EngineToShell::StreamEnd).await.ok();
                return Ok(());
            }

            debug!("Turn iteration {}", iteration);

            // Build context from history for the active session.
            let messages = self
                .history
                .get_all_messages_for_session(&session_id)
                .map_err(|e| {
                    api_client::ApiError::MalformedResponse(format!(
                        "History read error: {}",
                        e
                    ))
                })?;

            let system_prompt = render_system_prompt();

            let (system, api_messages) =
                self.context_builder
                    .build(&system_prompt, &self.skills.prompts, &messages);

            // Build tool definitions for the API.
            let api_tools: Vec<ApiTool> =
                self.skills.tools.iter().map(ApiTool::from_definition).collect();

            let request = MessageRequest {
                model: self.config.api.model.clone(),
                max_tokens: self.config.context.response_reserve as u32,
                system,
                messages: api_messages,
                tools: api_tools,
                stream: true,
            };

            // Create a channel for streaming text chunks.
            let (chunk_tx, mut chunk_rx) = mpsc::channel::<String>(256);

            // Spawn a task to forward chunks to the shell.
            let sender = self.sender.clone();
            let forward_handle = tokio::spawn(async move {
                while let Some(chunk) = chunk_rx.recv().await {
                    sender
                        .send(EngineToShell::StreamChunk {
                            content: chunk,
                        })
                        .await
                        .ok();
                }
            });

            // Send the request via the router.
            let result = self
                .router
                .send_streaming(request, &chunk_tx, cancel.clone())
                .await;

            // Drop chunk_tx so the forwarder finishes.
            drop(chunk_tx);
            forward_handle.await.ok();

            match result {
                Ok(StreamResult::TextComplete(text)) => {
                    // Persist the assistant response.
                    if !text.is_empty() {
                        if let Err(e) = self.history.insert_message_for_session(
                            &MessageRole::Assistant,
                            &text,
                            None,
                            None,
                            None,
                            Some("complete"),
                            &session_id,
                        ) {
                            error!("Failed to persist assistant message: {}", e);
                        }
                    }

                    // Auto-name the session after the first assistant response (iteration 0).
                    if iteration == 0 && self.config.sessions.auto_name {
                        self.try_auto_name_session(&session_id).await;
                    }

                    self.sender.send(EngineToShell::StreamEnd).await.ok();
                    return Ok(());
                }
                Ok(StreamResult::ToolUse {
                    text_before,
                    tool_calls,
                }) => {
                    // Persist any text before the tool calls.
                    if !text_before.is_empty() {
                        if let Err(e) = self.history.insert_message_for_session(
                            &MessageRole::Assistant,
                            &text_before,
                            None,
                            None,
                            None,
                            Some("complete"),
                            &session_id,
                        ) {
                            error!("Failed to persist assistant text: {}", e);
                        }
                    }

                    // Execute each tool call.
                    for tool_call in &tool_calls {
                        self.execute_tool_call(tool_call).await;
                    }

                    // Continue the loop — the next iteration will send tool results to the API.
                }
                Err(api_client::ApiError::Cancelled) => {
                    // User cancelled — send StreamEnd.
                    self.sender.send(EngineToShell::StreamEnd).await.ok();
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        warn!("Max tool call iterations reached");
        self.sender
            .send(EngineToShell::Error {
                message: "Too many tool calls in a single turn. Please try again.".to_string(),
                retryable: false,
            })
            .await
            .ok();
        self.sender.send(EngineToShell::StreamEnd).await.ok();
        Ok(())
    }

    /// Execute a single tool call: look up the tool, render the command, check
    /// destructive guard, execute, persist results.
    async fn execute_tool_call(&mut self, tool_call: &ToolCall) {
        info!("Executing tool call: {} (id={})", tool_call.name, tool_call.id);

        let session_id = self.session_mgr.active_session_id().to_string();

        // Persist the tool call in history.
        if let Err(e) = self.history.insert_message_for_session(
            &MessageRole::ToolCall,
            &tool_call.id,
            Some(&tool_call.name),
            Some(&tool_call.input.to_string()),
            None,
            None,
            &session_id,
        ) {
            error!("Failed to persist tool call: {}", e);
        }

        // Look up the tool definition.
        let tool_def = self.skills.tools.iter().find(|t| t.name == tool_call.name);

        let tool_def = match tool_def {
            Some(d) => d.clone(),
            None => {
                let error_msg = format!("Unknown tool: {}", tool_call.name);
                warn!("{}", error_msg);
                self.persist_tool_result(&tool_call.id, &error_msg, Some(1));
                return;
            }
        };

        // Branch on exec_type: "shell" (existing) vs "internal" (new).
        match tool_def.execution.exec_type.as_str() {
            "internal" => {
                self.execute_internal_tool(tool_call).await;
            }
            _ => {
                // Default: shell execution.
                self.execute_shell_tool(tool_call, &tool_def).await;
            }
        }
    }

    /// Execute a shell-based tool.
    async fn execute_shell_tool(
        &mut self,
        tool_call: &ToolCall,
        tool_def: &skill_loader::ToolDefinition,
    ) {
        // Render the command from the template.
        let command = match &tool_def.execution.command_template {
            Some(template) => tool_executor::render_template(template, &tool_call.input),
            None => {
                let error_msg = format!("Tool '{}' has no command template", tool_call.name);
                warn!("{}", error_msg);
                self.persist_tool_result(&tool_call.id, &error_msg, Some(1));
                return;
            }
        };

        debug!("Rendered command: {}", command);

        // Check destructive guard.
        if self.destructive_guard.is_destructive(&command) {
            info!("Command classified as destructive, requesting confirmation");

            let request_id = uuid::Uuid::new_v4().to_string();

            // Send confirmation request to shell.
            self.sender
                .send(EngineToShell::ConfirmRequest {
                    request_id: request_id.clone(),
                    command: command.clone(),
                    description: format!(
                        "Tool '{}' wants to execute a potentially destructive command",
                        tool_call.name
                    ),
                })
                .await
                .ok();

            // Wait for confirmation response.
            let (confirm_tx, confirm_rx) = oneshot::channel();
            self.pending_confirmations
                .insert(request_id.clone(), confirm_tx);

            // We need to poll the receiver for the confirmation response.
            let approved = self.wait_for_confirmation(confirm_rx).await;

            self.pending_confirmations.remove(&request_id);

            if !approved {
                info!("User rejected destructive command");
                let result_msg = "User cancelled this command.";
                self.persist_tool_result(&tool_call.id, result_msg, None);
                return;
            }

            info!("User approved destructive command");
        }

        // Send ToolStatus(Started).
        self.sender
            .send(EngineToShell::ToolStatus {
                tool_name: tool_call.name.clone(),
                status: ToolExecutionStatus::Started,
                description: format!("Running: {}", command),
            })
            .await
            .ok();

        // Execute the command.
        let timeout = tool_def
            .execution
            .timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(self.tool_executor.timeout);

        let is_streaming = tool_def.execution.streaming.unwrap_or(false);

        let result = if is_streaming {
            // Streaming mode: open ProgressView, stream stderr lines.
            let content_id = uuid::Uuid::new_v4().to_string();
            self.sender
                .send(EngineToShell::ContentOpen {
                    content_id: content_id.clone(),
                    content_type: types::ContentType::ProgressView {
                        operation: command.clone(),
                        total: None,
                    },
                    title: format!("Running: {}", tool_call.name),
                    content: String::new(),
                })
                .await
                .ok();

            let (line_tx, mut line_rx) = mpsc::channel::<String>(256);

            let sender = self.sender.clone();
            let cid = content_id.clone();
            let forward_handle = tokio::spawn(async move {
                while let Some(line) = line_rx.recv().await {
                    sender
                        .send(EngineToShell::ContentUpdate {
                            content_id: cid.clone(),
                            data: types::ContentUpdateData::AppendText {
                                text: format!("{}\n", line),
                            },
                        })
                        .await
                        .ok();
                }
            });

            let result = self
                .tool_executor
                .execute_streaming(&command, timeout, line_tx)
                .await;

            forward_handle.await.ok();

            self.sender
                .send(EngineToShell::ContentClose { content_id })
                .await
                .ok();

            result
        } else {
            self.tool_executor
                .execute_with_timeout(&command, timeout)
                .await
        };

        // Send ToolStatus(Completed).
        self.sender
            .send(EngineToShell::ToolStatus {
                tool_name: tool_call.name.clone(),
                status: ToolExecutionStatus::Completed {
                    success: result.success(),
                },
                description: if result.success() {
                    "Completed successfully".to_string()
                } else {
                    format!("Exited with code {}", result.exit_code)
                },
            })
            .await
            .ok();

        // Check if the tool result should be shown in a content panel.
        if !is_streaming {
            if let Some(ct) = &tool_def.execution.content_type {
                if ct == "diff" {
                    let raw_output = result.format_for_api();
                    let content_id = uuid::Uuid::new_v4().to_string();
                    self.sender
                        .send(EngineToShell::ContentOpen {
                            content_id,
                            content_type: types::ContentType::FilePreview {
                                file_path: String::new(),
                                language: "diff".to_string(),
                            },
                            title: format!("{} output", tool_call.name),
                            content: raw_output.clone(),
                        })
                        .await
                        .ok();
                }
            }
        }

        // Persist the tool result.
        let output = result.format_for_api();
        self.persist_tool_result(&tool_call.id, &output, Some(result.exit_code));
    }

    /// Execute an internal (non-shell) tool call for skill management,
    /// self-improvement, and filesystem operations.
    async fn execute_internal_tool(&mut self, tool_call: &ToolCall) {
        let function = tool_call
            .name
            .as_str();

        self.sender
            .send(EngineToShell::ToolStatus {
                tool_name: tool_call.name.clone(),
                status: ToolExecutionStatus::Started,
                description: format!("Running internal tool: {}", function),
            })
            .await
            .ok();

        let result = match function {
            // Skill management (Track C).
            "skill_install" => self.handle_skill_install(&tool_call.input).await,
            "skill_remove" => self.handle_skill_remove(&tool_call.input),
            "skill_list" => self.handle_skill_list(),
            "skill_search" => self.handle_skill_search(&tool_call.input).await,
            "skill_update" => self.handle_skill_update(&tool_call.input).await,
            "skill_info" => self.handle_skill_info(&tool_call.input),
            // Self-improvement (Track E).
            "self_improve" => {
                let result = self.handle_self_improve(&tool_call.input).await;
                self.skip_next_llm_iteration = true;
                result
            }
            "deploy_build" => self.handle_deploy_build(tool_call).await,
            "rollback" => self.handle_rollback(&tool_call.input).await,
            // Filesystem (Track F).
            "fs_read" => self.handle_fs_read(&tool_call.input).await,
            "fs_write" => self.handle_fs_write(tool_call).await,
            // Text editor (Track H).
            "edit_open" => self.handle_edit_open(&tool_call.input).await,
            "edit_insert" => self.handle_edit_insert(&tool_call.input).await,
            "edit_delete_lines" => self.handle_edit_delete_lines(&tool_call.input).await,
            "edit_replace_lines" => self.handle_edit_replace_lines(&tool_call.input).await,
            "edit_replace_text" => self.handle_edit_replace_text(&tool_call.input).await,
            "edit_append" => self.handle_edit_append(&tool_call.input).await,
            "edit_save" => self.handle_edit_save().await,
            "edit_undo" => self.handle_edit_undo().await,
            "edit_redo" => self.handle_edit_redo().await,
            "edit_diff" => self.handle_edit_diff().await,
            "edit_close" => self.handle_edit_close(&tool_call.input).await,
            _ => format!("Unknown internal tool function: {}", function),
        };

        self.sender
            .send(EngineToShell::ToolStatus {
                tool_name: tool_call.name.clone(),
                status: ToolExecutionStatus::Completed { success: true },
                description: "Completed".to_string(),
            })
            .await
            .ok();

        self.persist_tool_result(&tool_call.id, &result, Some(0));
    }

    // -----------------------------------------------------------------------
    // Skill management handlers (Track C)
    // -----------------------------------------------------------------------

    /// Handle the skill_install internal tool.
    async fn handle_skill_install(&mut self, input: &serde_json::Value) -> String {
        let name = input.get("name").and_then(|v| v.as_str());
        let url = input.get("url").and_then(|v| v.as_str());

        let result = if let Some(url) = url {
            self.skill_installer.install_from_url(url).await
        } else if let Some(name) = name {
            self.skill_installer
                .install_by_name(name, &self.registry_client)
                .await
        } else {
            return "Error: provide either 'name' or 'url' to install a skill.".to_string();
        };

        match result {
            Ok(skill_name) => {
                self.reload_skills();
                format!("Successfully installed skill '{}'.", skill_name)
            }
            Err(e) => format!("Failed to install skill: {}", e),
        }
    }

    /// Handle the skill_remove internal tool.
    fn handle_skill_remove(&mut self, input: &serde_json::Value) -> String {
        let name = match input.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return "Error: 'name' parameter is required.".to_string(),
        };

        match self.skill_installer.remove(name) {
            Ok(()) => {
                self.reload_skills();
                format!("Successfully removed skill '{}'.", name)
            }
            Err(e) => format!("Failed to remove skill: {}", e),
        }
    }

    /// Handle the skill_list internal tool.
    fn handle_skill_list(&self) -> String {
        let skills = self.skill_installer.list_all();
        if skills.is_empty() {
            return "No skills installed.".to_string();
        }

        let mut output = String::from("Installed skills:\n\n");
        for skill in &skills {
            let badge = if skill.builtin { "[builtin]" } else { "[installed]" };
            output.push_str(&format!(
                "  {} {} v{} — {} ({} tools)\n",
                badge, skill.name, skill.version, skill.description, skill.tool_count
            ));
        }
        output
    }

    /// Handle the skill_search internal tool.
    async fn handle_skill_search(&mut self, input: &serde_json::Value) -> String {
        let query = match input.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return "Error: 'query' parameter is required.".to_string(),
        };

        // Attempt to refresh the registry first.
        if let Err(e) = self.registry_client.refresh().await {
            warn!("Registry refresh failed: {}", e);
            // Continue — search cached data if available.
        }

        let results = self.registry_client.search(query);
        if results.is_empty() {
            return format!("No skills found matching '{}'.", query);
        }

        let mut output = format!("Found {} skill(s) matching '{}':\n\n", results.len(), query);
        for entry in &results {
            output.push_str(&format!(
                "  {} v{} — {}\n    by {} | tags: {}\n    repo: {}\n\n",
                entry.name,
                entry.version,
                entry.description,
                entry.author,
                entry.tags.join(", "),
                entry.repository,
            ));
        }
        output
    }

    /// Handle the skill_update internal tool.
    async fn handle_skill_update(&mut self, input: &serde_json::Value) -> String {
        let name = match input.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return "Error: 'name' parameter is required.".to_string(),
        };

        match self.skill_installer.update(name, &self.registry_client).await {
            Ok(skill_name) => {
                self.reload_skills();
                format!("Successfully updated skill '{}'.", skill_name)
            }
            Err(e) => format!("Failed to update skill: {}", e),
        }
    }

    /// Handle the skill_info internal tool.
    fn handle_skill_info(&self, input: &serde_json::Value) -> String {
        let name = match input.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => return "Error: 'name' parameter is required.".to_string(),
        };

        match self.skill_installer.get_info(name) {
            Some(info) => {
                let badge = if info.builtin { "builtin" } else { "installed" };
                format!(
                    "Skill: {}\nVersion: {}\nType: {}\nDescription: {}\nTools: {}",
                    info.name, info.version, badge, info.description, info.tool_count
                )
            }
            None => format!("Skill '{}' not found.", name),
        }
    }

    // -----------------------------------------------------------------------
    // Self-improvement handlers (Track E)
    // -----------------------------------------------------------------------

    /// Handle the `self_improve` tool: spawn a coding agent to modify source code.
    ///
    /// Currently bypasses the coding agent phase: copies pre-baked changes from
    /// the staging directory into the source tree, then proceeds to build/deploy.
    /// The staging directory (`self-improve-staging/`) mirrors the source tree
    /// layout and contains the modified files.
    async fn handle_self_improve(&mut self, input: &serde_json::Value) -> String {
        // Debug log to file for tracing self-improve flow.
        use std::io::Write as _;
        let mut dbg_log = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open("/tmp/self_improve_debug.log")
            .ok();
        macro_rules! dbg_log {
            ($($arg:tt)*) => {
                if let Some(ref mut f) = dbg_log {
                    let _ = writeln!(f, "[{:?}] {}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default(), format!($($arg)*));
                    let _ = f.flush();
                }
            };
        }
        dbg_log!("handle_self_improve called");

        let prompt = match input.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return "Error: 'prompt' parameter is required.".to_string(),
        };

        // Kill any existing session (in case one is lingering).
        if let Some(mut session) = self.self_improve_mgr.active_session.take() {
            info!("Killing previous coding agent session");
            if let Err(e) = session.kill().await {
                warn!("Failed to kill previous session: {}", e);
            }
        }

        // Open agent activity panel.
        let content_id = uuid::Uuid::new_v4().to_string();
        self.sender
            .send(EngineToShell::ContentOpen {
                content_id: content_id.clone(),
                content_type: types::ContentType::AgentActivity {
                    operation: "Self-improvement".to_string(),
                },
                title: "Coding Agent".to_string(),
                content: String::new(),
            })
            .await
            .ok();

        // Spawn the coding agent.
        let (event_tx, mut event_rx) = mpsc::channel::<self_improve::AgentEvent>(256);
        let session = match self_improve::CodingAgentSession::spawn(
            &self.self_improve_mgr.agent_config,
            &prompt,
            event_tx,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("Failed to spawn coding agent: {}", e);
                dbg_log!("SPAWN ERROR: {}", msg);
                warn!("{}", msg);
                self.sender
                    .send(EngineToShell::ContentUpdate {
                        content_id: content_id.clone(),
                        data: types::ContentUpdateData::AgentEvent {
                            event_type: "error".to_string(),
                            content: msg.clone(),
                            file_path: None,
                        },
                    })
                    .await
                    .ok();
                self.close_content(&content_id).await;
                return msg;
            }
        };
        self.self_improve_mgr.active_session = Some(session);
        dbg_log!("Agent spawned successfully, entering event loop");

        // Forward agent events to the UI.
        let mut agent_error: Option<String> = None;
        let mut event_count = 0u32;
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            dbg_log!("Event #{}: {:?}", event_count, std::mem::discriminant(&event));
            let (event_type, content, file_path) = match &event {
                self_improve::AgentEvent::Thinking { content } => {
                    ("thinking", content.clone(), None)
                }
                self_improve::AgentEvent::FileRead { path, content_preview } => {
                    let desc = content_preview
                        .as_deref()
                        .map(|p| format!("Reading {} ({})", path, p))
                        .unwrap_or_else(|| format!("Reading {}", path));
                    ("file_read", desc, Some(path.clone()))
                }
                self_improve::AgentEvent::FileEdit { path, diff_snippet } => {
                    let desc = diff_snippet
                        .as_deref()
                        .map(|d| format!("Editing {}: {}", path, d))
                        .unwrap_or_else(|| format!("Editing {}", path));
                    ("file_edit", desc, Some(path.clone()))
                }
                self_improve::AgentEvent::BashCommand { command, output_preview } => {
                    let desc = output_preview
                        .as_deref()
                        .map(|o| format!("$ {} → {}", command, o))
                        .unwrap_or_else(|| format!("$ {}", command));
                    ("bash", desc, None)
                }
                self_improve::AgentEvent::CodeSearch { pattern, match_count } => {
                    let desc = match match_count {
                        Some(n) => format!("Search '{}' → {} matches", pattern, n),
                        None => format!("Search '{}'", pattern),
                    };
                    ("search", desc, None)
                }
                self_improve::AgentEvent::Complete { summary, duration } => {
                    ("complete", format!("{} ({:.1}s)", summary, duration.as_secs_f64()), None)
                }
                self_improve::AgentEvent::Error { message } => {
                    agent_error = Some(message.clone());
                    ("error", message.clone(), None)
                }
                self_improve::AgentEvent::Timeout => {
                    agent_error = Some("Coding agent timed out".to_string());
                    ("error", "Coding agent timed out".to_string(), None)
                }
                self_improve::AgentEvent::Unknown { raw } => {
                    ("thinking", raw.clone(), None)
                }
            };

            self.sender
                .send(EngineToShell::ContentUpdate {
                    content_id: content_id.clone(),
                    data: types::ContentUpdateData::AgentEvent {
                        event_type: event_type.to_string(),
                        content,
                        file_path,
                    },
                })
                .await
                .ok();
        }

        dbg_log!("Event loop ended, total events: {}", event_count);

        // Wait for the agent process to exit.
        if let Some(mut session) = self.self_improve_mgr.active_session.take() {
            match session.wait().await {
                Ok(status) => {
                    if !status.success() && agent_error.is_none() {
                        agent_error = Some(format!("Coding agent exited with {}", status));
                    }
                }
                Err(e) => {
                    if agent_error.is_none() {
                        agent_error = Some(format!("Failed to wait for coding agent: {}", e));
                    }
                }
            }
        }

        // Close the agent activity panel.
        self.close_content(&content_id).await;

        if let Some(ref err) = agent_error {
            dbg_log!("Agent error: {}", err);
            return format!("Self-improvement failed: {}", err);
        }

        // Check if there are uncommitted changes in the source tree.
        let has_changes = self
            .self_improve_mgr
            .git
            .has_changes()
            .await
            .unwrap_or(false);

        dbg_log!("git has_changes: {}", has_changes);
        if !has_changes {
            return "Coding agent completed but made no changes to the source tree.".to_string();
        }

        info!("Coding agent completed with changes, proceeding to deploy_build");

        // Proceed directly to deploy_build (the next step in the pipeline).
        let auto_tool = ToolCall {
            id: uuid::Uuid::new_v4().to_string(),
            name: "deploy_build".to_string(),
            input: serde_json::json!({
                "component": "full",
                "message": format!("self-improvement: {}", prompt.chars().take(80).collect::<String>()),
            }),
        };
        self.handle_deploy_build(&auto_tool).await
    }

    /// Handle the `deploy_build` tool: commit, build, test, checkpoint, confirm, and deploy.
    ///
    /// All 6 steps stream their output into a single ProgressView panel.
    async fn handle_deploy_build(&mut self, tool_call: &ToolCall) -> String {
        let target_str = tool_call
            .input
            .get("component")
            .and_then(|v| v.as_str())
            .unwrap_or("full");
        let target = self_improve::builder::BuildTarget::from_str(target_str);

        // Open a single ProgressView panel for the entire pipeline.
        let content_id = uuid::Uuid::new_v4().to_string();
        self.sender
            .send(EngineToShell::ContentOpen {
                content_id: content_id.clone(),
                content_type: types::ContentType::ProgressView {
                    operation: "Self-Improvement Pipeline".to_string(),
                    total: Some(6),
                },
                title: "Self-Improvement Pipeline".to_string(),
                content: String::new(),
            })
            .await
            .ok();

        // ═══ Step 1/6: Git Commit ═══
        self.send_progress_text(&content_id, "\n═══ Step 1/6: Git Commit ═══\n").await;
        self.send_progress_update(&content_id, 1, "Committing changes...").await;

        let commit_msg = tool_call
            .input
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("self-improvement: automated changes");

        info!(commit_msg, "deploy_build: committing changes");
        match self.self_improve_mgr.git.commit_all(commit_msg).await {
            Ok(output) => {
                let trimmed = output.trim().to_string();
                info!("Committed: {}", trimmed);
                self.send_progress_text(&content_id, &format!("{}\n", trimmed)).await;
            }
            Err(e) => {
                if !e.contains("nothing to commit") {
                    self.send_progress_text(&content_id, &format!("FAILED: {}\n", e)).await;
                    self.close_content(&content_id).await;
                    return format!("Failed to commit changes: {}", e);
                }
                info!("Nothing new to commit, proceeding with build");
                self.send_progress_text(&content_id, "Nothing new to commit, proceeding.\n").await;
            }
        }

        // ═══ Step 2/6: Build ═══
        self.send_progress_text(&content_id, "\n═══ Step 2/6: Build ═══\n").await;
        self.send_progress_update(&content_id, 2, "Building...").await;

        let (progress_tx, mut progress_rx) = mpsc::channel::<String>(256);

        let sender = self.sender.clone();
        let cid = content_id.clone();
        let forward_handle = tokio::spawn(async move {
            while let Some(line) = progress_rx.recv().await {
                sender
                    .send(EngineToShell::ContentUpdate {
                        content_id: cid.clone(),
                        data: types::ContentUpdateData::AppendText {
                            text: format!("{}\n", line),
                        },
                    })
                    .await
                    .ok();
            }
        });

        let build_result = self
            .self_improve_mgr
            .builder
            .build_streaming(target.clone(), progress_tx)
            .await;

        forward_handle.await.ok();

        if !build_result.success {
            self.send_progress_text(&content_id, "\nBuild FAILED.\n").await;
            self.close_content(&content_id).await;
            return format!("Build failed.\n\n{}", format_build_result(&build_result));
        }
        self.send_progress_text(&content_id, "Build succeeded.\n").await;

        // ═══ Step 3/6: Tests ═══
        self.send_progress_text(&content_id, "\n═══ Step 3/6: Tests ═══\n").await;
        self.send_progress_update(&content_id, 3, "Running tests...").await;

        let (test_tx, mut test_rx) = mpsc::channel::<String>(256);

        let sender = self.sender.clone();
        let cid = content_id.clone();
        let test_forward = tokio::spawn(async move {
            while let Some(line) = test_rx.recv().await {
                sender
                    .send(EngineToShell::ContentUpdate {
                        content_id: cid.clone(),
                        data: types::ContentUpdateData::AppendText {
                            text: format!("{}\n", line),
                        },
                    })
                    .await
                    .ok();
            }
        });

        let test_result = self
            .self_improve_mgr
            .builder
            .test_streaming(target, test_tx)
            .await;

        test_forward.await.ok();

        let test_warning = if !test_result.success {
            warn!("Tests failed, but proceeding with deploy");
            let msg = format!(
                "Warning: tests failed ({} errors). Proceeding anyway.\n",
                test_result.errors.len()
            );
            self.send_progress_text(&content_id, &msg).await;
            msg
        } else {
            self.send_progress_text(&content_id, "All tests passed.\n").await;
            String::new()
        };

        // ═══ Step 4/6: Checkpoint ═══
        self.send_progress_text(&content_id, "\n═══ Step 4/6: Checkpoint ═══\n").await;
        self.send_progress_update(&content_id, 4, "Creating checkpoint...").await;

        let git_hash = self
            .self_improve_mgr
            .git
            .current_hash()
            .await
            .unwrap_or_else(|_| "unknown".to_string());

        let artifact_paths: Vec<PathBuf> = build_result
            .artifact_path
            .iter()
            .cloned()
            .collect();

        let checkpoint_msg = match self
            .self_improve_mgr
            .checkpoint
            .create(&git_hash, &artifact_paths)
        {
            Ok(ckpt) => format!("Checkpoint created: {}", ckpt.id),
            Err(e) => {
                warn!("Failed to create checkpoint: {}", e);
                format!("Warning: checkpoint creation failed: {}", e)
            }
        };
        self.send_progress_text(&content_id, &format!("{}\n", checkpoint_msg)).await;

        // ═══ Step 5/6: Confirmation ═══
        self.send_progress_text(&content_id, "\n═══ Step 5/6: Confirmation ═══\n").await;
        self.send_progress_update(&content_id, 5, "Waiting for confirmation...").await;

        // Close the panel before the confirmation dialog.
        self.close_content(&content_id).await;

        let request_id = uuid::Uuid::new_v4().to_string();
        self.sender
            .send(EngineToShell::ConfirmRequest {
                request_id: request_id.clone(),
                command: format!("Deploy {} (git: {})", target_str, &git_hash[..8.min(git_hash.len())]),
                description: format!(
                    "Build succeeded. {}\n{}\nDeploy this build?",
                    checkpoint_msg, test_warning
                ),
            })
            .await
            .ok();

        let (confirm_tx, confirm_rx) = oneshot::channel();
        self.pending_confirmations
            .insert(request_id.clone(), confirm_tx);
        let approved = self.wait_for_confirmation(confirm_rx).await;
        self.pending_confirmations.remove(&request_id);

        if !approved {
            return format!(
                "Build succeeded but deployment cancelled by user.\n{}\n{}",
                checkpoint_msg, test_warning
            );
        }

        // ═══ Step 6/6: Deploy ═══
        // Reopen the panel for deploy status.
        let deploy_content_id = uuid::Uuid::new_v4().to_string();
        self.sender
            .send(EngineToShell::ContentOpen {
                content_id: deploy_content_id.clone(),
                content_type: types::ContentType::ProgressView {
                    operation: "Deploying".to_string(),
                    total: Some(6),
                },
                title: "Self-Improvement Pipeline".to_string(),
                content: String::new(),
            })
            .await
            .ok();
        self.send_progress_text(&deploy_content_id, "\n═══ Step 6/6: Deploy ═══\n").await;
        self.send_progress_update(&deploy_content_id, 6, "Deploying...").await;

        let result_msg = if let Some(artifact_path) = &build_result.artifact_path {
            match self
                .self_improve_mgr
                .deployer
                .deploy(target_str, artifact_path)
                .await
            {
                Ok(msg) => {
                    self.send_progress_text(&deploy_content_id, &format!("{}\n", msg)).await;

                    let final_msg = format!(
                        "{}\n{}\n{}",
                        msg, checkpoint_msg, test_warning,
                    );

                    // For chat-shell/full targets, we need to pkill ourselves.
                    // Persist the result BEFORE killing, then spawn a delayed pkill.
                    if matches!(target_str, "chat-shell" | "chat_shell" | "full") {
                        self.send_progress_text(
                            &deploy_content_id,
                            "Restarting chat shell in 1 second...\n",
                        ).await;
                        self.close_content(&deploy_content_id).await;

                        // Persist the tool result now, before we die.
                        self.persist_tool_result(&tool_call.id, &final_msg, Some(0));

                        // Persist a final assistant message so the user sees the result.
                        let session_id = self.session_mgr.active_session_id().to_string();
                        self.history
                            .insert_message_for_session(
                                &MessageRole::Assistant,
                                &format!("Self-improvement complete. {}", final_msg),
                                None,
                                None,
                                None,
                                Some("complete"),
                                &session_id,
                            )
                            .ok();

                        // Spawn a delayed pkill — this kills the running process.
                        tokio::spawn(async {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            info!("Executing pkill levsha-chat for self-restart");
                            let _ = tokio::process::Command::new("pkill")
                                .arg("-x")
                                .arg("levsha-chat")
                                .status()
                                .await;
                        });

                        return final_msg;
                    }

                    final_msg
                }
                Err(e) => {
                    let msg = format!("Deploy failed: {}\n{}", e, checkpoint_msg);
                    self.send_progress_text(&deploy_content_id, &format!("FAILED: {}\n", e)).await;
                    msg
                }
            }
        } else {
            let msg = format!(
                "Build succeeded but no artifact path available for deployment.\n{}",
                checkpoint_msg
            );
            self.send_progress_text(&deploy_content_id, &msg).await;
            msg
        };

        self.close_content(&deploy_content_id).await;
        result_msg
    }

    /// Send an AppendText update to a content panel.
    async fn send_progress_text(&self, content_id: &str, text: &str) {
        self.sender
            .send(EngineToShell::ContentUpdate {
                content_id: content_id.to_string(),
                data: types::ContentUpdateData::AppendText {
                    text: text.to_string(),
                },
            })
            .await
            .ok();
    }

    /// Send a Progress update to a content panel.
    async fn send_progress_update(&self, content_id: &str, current: u64, message: &str) {
        self.sender
            .send(EngineToShell::ContentUpdate {
                content_id: content_id.to_string(),
                data: types::ContentUpdateData::Progress {
                    current,
                    message: message.to_string(),
                },
            })
            .await
            .ok();
    }

    /// Close a content panel.
    async fn close_content(&self, content_id: &str) {
        self.sender
            .send(EngineToShell::ContentClose {
                content_id: content_id.to_string(),
            })
            .await
            .ok();
    }

    /// Handle the `rollback` tool: restore a checkpoint and restart the service.
    async fn handle_rollback(&self, input: &serde_json::Value) -> String {
        // If no checkpoint_id given, use the latest checkpoint.
        let checkpoint = if let Some(checkpoint_id) = input.get("checkpoint_id").and_then(|v| v.as_str()) {
            match self.self_improve_mgr.checkpoint.restore(checkpoint_id) {
                Ok(ckpt) => ckpt,
                Err(e) => return format!("Error: {}", e),
            }
        } else {
            // Use the latest checkpoint.
            let checkpoints = self.self_improve_mgr.checkpoint.list();
            match checkpoints.into_iter().next() {
                Some(ckpt) => ckpt,
                None => return "No checkpoints available for rollback.".to_string(),
            }
        };

        let target = input
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("engine");

        // Find the right binary for the target.
        let binary_path = checkpoint.binaries.iter().find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains(target))
                .unwrap_or(false)
        });

        match binary_path {
            Some(path) => {
                match self
                    .self_improve_mgr
                    .deployer
                    .rollback(target, path)
                    .await
                {
                    Ok(msg) => format!(
                        "{}\nRestored from checkpoint: {} (git: {})",
                        msg, checkpoint.id, checkpoint.git_hash
                    ),
                    Err(e) => format!("Rollback failed: {}", e),
                }
            }
            None => format!(
                "No binary found for target '{}' in checkpoint {}",
                target, checkpoint.id
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Filesystem handlers (Track F)
    // -----------------------------------------------------------------------

    async fn handle_fs_read(&self, input: &serde_json::Value) -> String {
        let path = match input.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: 'path' parameter is required.".to_string(),
        };

        let content = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => return format!("Error reading file: {}", e),
        };

        // Check for binary content (null bytes in first 8KB).
        let check_len = content.len().min(8192);
        let is_binary = content[..check_len].contains(&0);

        if is_binary {
            return format!(
                "Binary file: {} ({} bytes). Cannot display binary content.",
                path,
                content.len()
            );
        }

        let text = String::from_utf8_lossy(&content).to_string();
        let line_count = text.lines().count();

        // Detect language from extension for content panel.
        let language = detect_language(path);

        if line_count > 30 {
            // Open in content panel.
            let content_id = uuid::Uuid::new_v4().to_string();
            let file_name = std::path::Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            self.sender
                .send(EngineToShell::ContentOpen {
                    content_id,
                    content_type: types::ContentType::FilePreview {
                        file_path: path.to_string(),
                        language: language.to_string(),
                    },
                    title: file_name,
                    content: text.clone(),
                })
                .await
                .ok();

            format!(
                "File opened in preview panel ({} lines, {} language).",
                line_count, language
            )
        } else {
            // Return inline for short files.
            format!("```{}\n{}\n```", language, text)
        }
    }

    async fn handle_fs_write(&mut self, tool_call: &ToolCall) -> String {
        let path = match tool_call.input.get("path").and_then(|v| v.as_str()) {
            Some(p) => p.to_string(),
            None => return "Error: 'path' parameter is required.".to_string(),
        };
        let content = match tool_call.input.get("content").and_then(|v| v.as_str()) {
            Some(c) => c.to_string(),
            None => return "Error: 'content' parameter is required.".to_string(),
        };

        // Check if file exists — if so, request confirmation for overwrite.
        if std::path::Path::new(&path).exists() {
            let request_id = uuid::Uuid::new_v4().to_string();
            self.sender
                .send(EngineToShell::ConfirmRequest {
                    request_id: request_id.clone(),
                    command: format!("Overwrite file: {}", path),
                    description: "File already exists; this will overwrite it.".to_string(),
                })
                .await
                .ok();

            let (confirm_tx, confirm_rx) = oneshot::channel();
            self.pending_confirmations
                .insert(request_id.clone(), confirm_tx);
            let approved = self.wait_for_confirmation(confirm_rx).await;
            self.pending_confirmations.remove(&request_id);

            if !approved {
                return "User cancelled file write.".to_string();
            }
        }

        // Ensure parent directory exists.
        if let Some(parent) = std::path::Path::new(&path).parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return format!("Error creating directory: {}", e);
            }
        }

        match std::fs::write(&path, &content) {
            Ok(()) => format!("Successfully wrote {} bytes to {}", content.len(), path),
            Err(e) => format!("Error writing file: {}", e),
        }
    }

    // -----------------------------------------------------------------------
    // Text editor handlers (Track H)
    // -----------------------------------------------------------------------

    async fn handle_edit_open(&mut self, input: &serde_json::Value) -> String {
        let path = match input.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return "Error: 'path' parameter is required.".to_string(),
        };

        let content_id = uuid::Uuid::new_v4().to_string();
        let state = match edit_state::EditState::open(path, content_id.clone()) {
            Ok(s) => s,
            Err(e) => return format!("Error: {}", e),
        };

        let line_count = state.line_count();
        let language = detect_language(path);
        let file_name = std::path::Path::new(path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        self.sender
            .send(EngineToShell::ContentOpen {
                content_id: content_id.clone(),
                content_type: types::ContentType::FilePreview {
                    file_path: path.to_string(),
                    language: language.to_string(),
                },
                title: file_name,
                content: state.content(),
            })
            .await
            .ok();

        self.edit_state = Some(state);

        format!("Opened {} for editing ({} lines)", path, line_count)
    }

    async fn handle_edit_insert(&mut self, input: &serde_json::Value) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        let line = match input.get("line").and_then(|v| v.as_u64()) {
            Some(l) => l as usize,
            None => return "Error: 'line' parameter is required.".to_string(),
        };

        let content_lines = match Self::parse_content_param(input, "content") {
            Some(lines) => lines,
            None => return "Error: 'content' parameter is required.".to_string(),
        };

        match state.insert_lines(line, content_lines) {
            Ok(msg) => {
                self.send_edit_content_update().await;
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_delete_lines(&mut self, input: &serde_json::Value) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        let start_line = match input.get("start_line").and_then(|v| v.as_u64()) {
            Some(l) => l as usize,
            None => return "Error: 'start_line' parameter is required.".to_string(),
        };

        let count = input
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        match state.delete_lines(start_line, count) {
            Ok(msg) => {
                self.send_edit_content_update().await;
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_replace_lines(&mut self, input: &serde_json::Value) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        let start_line = match input.get("start_line").and_then(|v| v.as_u64()) {
            Some(l) => l as usize,
            None => return "Error: 'start_line' parameter is required.".to_string(),
        };

        let count = match input.get("count").and_then(|v| v.as_u64()) {
            Some(c) => c as usize,
            None => return "Error: 'count' parameter is required.".to_string(),
        };

        let content_lines = match Self::parse_content_param(input, "content") {
            Some(lines) => lines,
            None => return "Error: 'content' parameter is required.".to_string(),
        };

        match state.replace_lines(start_line, count, content_lines) {
            Ok(msg) => {
                self.send_edit_content_update().await;
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_replace_text(&mut self, input: &serde_json::Value) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        let find = match input.get("find").and_then(|v| v.as_str()) {
            Some(f) => f,
            None => return "Error: 'find' parameter is required.".to_string(),
        };

        let replace = match input.get("replace").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return "Error: 'replace' parameter is required.".to_string(),
        };

        match state.replace_text(find, replace) {
            Ok(msg) => {
                self.send_edit_content_update().await;
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_append(&mut self, input: &serde_json::Value) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        let content_lines = match Self::parse_content_param(input, "content") {
            Some(lines) => lines,
            None => return "Error: 'content' parameter is required.".to_string(),
        };

        match state.append(content_lines) {
            Ok(msg) => {
                self.send_edit_content_update().await;
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_save(&mut self) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        match state.save() {
            Ok(msg) => msg,
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_undo(&mut self) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        match state.undo() {
            Ok(msg) => {
                self.send_edit_content_update().await;
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_redo(&mut self) -> String {
        let state = match self.edit_state.as_mut() {
            Some(s) => s,
            None => return "Error: No file is currently open.".to_string(),
        };

        match state.redo() {
            Ok(msg) => {
                self.send_edit_content_update().await;
                msg
            }
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_edit_diff(&self) -> String {
        let state = match &self.edit_state {
            Some(s) => s,
            None => return "No file is currently open.".to_string(),
        };

        let original = state.original_content();
        let current = state.content();

        if original == current {
            return "No changes.".to_string();
        }

        let content_id = uuid::Uuid::new_v4().to_string();
        let file_path = state.file_path.display().to_string();

        self.sender
            .send(EngineToShell::ContentOpen {
                content_id,
                content_type: types::ContentType::DiffView {
                    file_path: file_path.clone(),
                    old_content: original,
                    new_content: current,
                },
                title: format!("Diff: {}", file_path),
                content: String::new(),
            })
            .await
            .ok();

        format!("Diff shown in content panel.\n\n{}", state.diff())
    }

    async fn handle_edit_close(&mut self, input: &serde_json::Value) -> String {
        let force = input
            .get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let (can_close, content_id) = match &self.edit_state {
            Some(state) => match state.can_close(force) {
                Ok(()) => (true, Some(state.content_id.clone())),
                Err(e) => return format!("Error: {}", e),
            },
            None => return "Error: No file is currently open.".to_string(),
        };

        if can_close {
            if let Some(content_id) = content_id {
                self.sender
                    .send(EngineToShell::ContentClose { content_id })
                    .await
                    .ok();
            }
            self.edit_state = None;
        }

        "Closed editor.".to_string()
    }

    async fn send_edit_content_update(&self) {
        if let Some(state) = &self.edit_state {
            self.sender
                .send(EngineToShell::ContentUpdate {
                    content_id: state.content_id.clone(),
                    data: types::ContentUpdateData::ReplaceText {
                        text: state.content(),
                    },
                })
                .await
                .ok();
        }
    }

    fn parse_content_param(input: &serde_json::Value, key: &str) -> Option<Vec<String>> {
        match input.get(key) {
            Some(serde_json::Value::String(s)) => Some(s.lines().map(String::from).collect()),
            Some(serde_json::Value::Array(arr)) => {
                Some(arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            }
            _ => None,
        }
    }

    // -----------------------------------------------------------------------
    // Common helpers
    // -----------------------------------------------------------------------

    /// Attempt to auto-name a session if it still has the default name.
    async fn try_auto_name_session(&mut self, session_id: &str) {
        // Check if the session still has the default name.
        let session = match self.session_mgr.store().get_session(session_id) {
            Ok(Some(s)) => s,
            _ => return,
        };

        if session.name != self.config.sessions.default_name {
            return; // Already named by user.
        }

        // Get the first user message for this session.
        let messages = match self.history.get_messages_for_session(session_id) {
            Ok(m) => m,
            Err(_) => return,
        };

        let user_msg = match messages.iter().find(|m| m.role == MessageRole::User) {
            Some(m) => &m.content,
            None => return,
        };

        // Fire-and-forget: generate name via lightweight API call.
        let timeout = Duration::from_secs(10);
        if let Some(name) = ApiClient::generate_session_name(
            &self.config.api.base_url,
            &self.config.api.key,
            user_msg,
            timeout,
        )
        .await
        {
            if let Ok(()) = self.session_mgr.rename(session_id, &name) {
                info!("Auto-named session {} to '{}'", session_id, name);
                self.sender
                    .send(EngineToShell::SessionRenamed {
                        session_id: session_id.to_string(),
                        name,
                    })
                    .await
                    .ok();
            }
        }
    }

    /// Reload skills from both builtin and installed paths.
    fn reload_skills(&mut self) {
        let builtin_str = self.skill_installer.builtin_path.to_string_lossy().to_string();
        let installed_str = self.skill_installer.installed_path.to_string_lossy().to_string();
        self.skills = skill_loader::load_skills_multi(&[&builtin_str, &installed_str]);
        info!(
            "Reloaded skills: {} prompts, {} tools",
            self.skills.prompts.len(),
            self.skills.tools.len()
        );
    }

    /// Persist a tool result in the history database.
    fn persist_tool_result(&self, tool_use_id: &str, content: &str, exit_code: Option<i32>) {
        let session_id = self.session_mgr.active_session_id();
        if let Err(e) = self.history.insert_message_for_session(
            &MessageRole::ToolResult,
            content,
            Some(tool_use_id),
            None,
            exit_code,
            None,
            session_id,
        ) {
            error!("Failed to persist tool result: {}", e);
        }
    }

    /// Wait for a confirmation response, polling the main receiver meanwhile.
    async fn wait_for_confirmation(&mut self, confirm_rx: oneshot::Receiver<bool>) -> bool {
        tokio::select! {
            result = confirm_rx => {
                result.unwrap_or(false)
            }
            msg = self.receiver.recv() => {
                // Process intervening messages while waiting for confirmation.
                if let Some(msg) = msg {
                    match msg {
                        ShellToEngine::ConfirmResponse { request_id, approved } => {
                            if let Some(tx) = self.pending_confirmations.remove(&request_id) {
                                tx.send(approved).ok();
                            }
                            // If this was our confirmation, the select above should resolve.
                            // If not, return false (unexpected state).
                            return approved;
                        }
                        ShellToEngine::CancelStream => {
                            return false;
                        }
                        _ => {
                            // Ignore other messages while waiting for confirmation.
                            return false;
                        }
                    }
                }
                false
            }
        }
    }

    /// Handle a confirmation response from the shell.
    fn handle_confirm_response(&mut self, request_id: String, approved: bool) {
        if let Some(tx) = self.pending_confirmations.remove(&request_id) {
            debug!(
                "Confirm response for {}: {}",
                request_id,
                if approved { "approved" } else { "rejected" }
            );
            tx.send(approved).ok();
        } else {
            warn!("Received confirmation for unknown request_id: {}", request_id);
        }
    }

    /// Handle a cancel request from the shell.
    fn handle_cancel(&mut self) {
        if let Some(cancel) = &self.current_cancel {
            info!("Cancelling current stream");
            cancel.cancel();
        }
    }
}

/// Render the system prompt with current datetime.
fn render_system_prompt() -> String {
    let now = chrono_lite_now();
    SYSTEM_PROMPT.replace("{{datetime}}", &now)
}

/// Simple datetime string without pulling in the chrono crate.
fn chrono_lite_now() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Calculate a rough UTC datetime.
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;

    // Days since epoch to year/month/day (simplified).
    let (year, month, day) = days_to_ymd(days as i64);

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        year, month, day, hours, minutes
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Format a build result for display.
fn format_build_result(result: &self_improve::builder::BuildResult) -> String {
    let mut output = String::new();

    if result.success {
        output.push_str("Build succeeded.\n");
    } else {
        output.push_str("Build failed.\n");
    }

    if !result.warnings.is_empty() {
        output.push_str(&format!("\nWarnings ({}):\n", result.warnings.len()));
        for w in &result.warnings {
            output.push_str(&format!("  {}\n", w));
        }
    }

    if !result.errors.is_empty() {
        output.push_str(&format!("\nErrors ({}):\n", result.errors.len()));
        for e in &result.errors {
            output.push_str(&format!("  {}\n", e));
        }
    }

    if let Some(path) = &result.artifact_path {
        output.push_str(&format!("\nArtifact: {}\n", path.display()));
    }

    output
}

/// Detect language from file extension for syntax highlighting.
fn detect_language(path: &str) -> &str {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "md" => "markdown",
        "sh" | "bash" => "bash",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "h" | "hpp" => "cpp",
        "go" => "go",
        "html" | "htm" => "html",
        "css" => "css",
        "xml" => "xml",
        "sql" => "sql",
        "txt" => "text",
        "conf" | "cfg" | "ini" => "ini",
        _ => "text",
    }
}
