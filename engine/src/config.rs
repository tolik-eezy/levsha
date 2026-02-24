//! Configuration loading with dev-mode fallback support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::info;

/// Top-level configuration.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub api: ApiConfig,
    pub context: ContextConfig,
    pub execution: ExecutionConfig,
    pub skills: SkillsConfig,
    pub persistence: PersistenceConfig,
    /// Local model configuration (Track D).
    #[serde(default)]
    pub local_model: LocalModelConfig,
    /// Routing configuration (Track D).
    #[serde(default)]
    pub routing: RoutingConfig,
    /// Self-improvement configuration (Track E).
    #[serde(default)]
    pub self_improve: SelfImproveConfig,
    /// DeepSeek API configuration (OpenAI-compatible cloud backend).
    #[serde(default)]
    pub deepseek: DeepSeekConfig,
    /// Session configuration (Track J).
    #[serde(default)]
    pub sessions: SessionsConfig,
    /// Path from which this config was loaded (not serialized).
    #[serde(skip)]
    pub config_path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiConfig {
    pub key: String,
    pub model: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    /// Alias-to-model_id mapping (e.g. "opus" -> "claude-opus-4-20250514").
    #[serde(default)]
    pub models: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ContextConfig {
    pub max_tokens: usize,
    pub response_reserve: usize,
    pub chars_per_token: usize,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionConfig {
    pub command_timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct SkillsConfig {
    pub path: String,
    /// URL for the skill registry repository (Track C).
    #[serde(default)]
    pub registry_url: Option<String>,
    /// Path where user-installed skills are stored (Track C).
    #[serde(default)]
    pub installed_path: Option<String>,
    /// Path for registry cache (Track C).
    #[serde(default)]
    pub cache_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PersistenceConfig {
    pub db_path: String,
}

/// Configuration for the local LLM server (Track D).
#[derive(Debug, Deserialize)]
pub struct LocalModelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_server_binary")]
    pub server_binary: String,
    #[serde(default = "default_model_path")]
    pub model_path: String,
    #[serde(default = "default_models_dir")]
    pub models_dir: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_context_size")]
    pub context_size: u32,
    #[serde(default)]
    pub gpu_layers: u32,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_binary: default_server_binary(),
            model_path: default_model_path(),
            models_dir: default_models_dir(),
            port: default_port(),
            context_size: default_context_size(),
            gpu_layers: 0,
        }
    }
}

fn default_server_binary() -> String {
    "/usr/bin/llama-server".to_string()
}
fn default_model_path() -> String {
    "/var/lib/levsha/models/llama-3-8b-q4.gguf".to_string()
}
fn default_models_dir() -> String {
    "/var/lib/levsha/models".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_context_size() -> u32 {
    4096
}

/// Routing configuration for LLM backend selection (Track D).
#[derive(Debug, Deserialize)]
pub struct RoutingConfig {
    #[serde(default = "default_routing_mode")]
    pub mode: String,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            mode: default_routing_mode(),
        }
    }
}

fn default_routing_mode() -> String {
    "auto".to_string()
}

/// DeepSeek API configuration (OpenAI-compatible cloud backend).
#[derive(Debug, Deserialize)]
pub struct DeepSeekConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub key: String,
    #[serde(default = "default_deepseek_model")]
    pub model: String,
    #[serde(default = "default_deepseek_base_url")]
    pub base_url: String,
}

impl Default for DeepSeekConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            key: String::new(),
            model: default_deepseek_model(),
            base_url: default_deepseek_base_url(),
        }
    }
}

fn default_deepseek_model() -> String {
    "deepseek-chat".to_string()
}
fn default_deepseek_base_url() -> String {
    "https://api.deepseek.com".to_string()
}

/// Self-improvement system configuration (Track E).
#[derive(Debug, Deserialize)]
pub struct SelfImproveConfig {
    #[serde(default = "default_source_root")]
    pub source_root: String,
    #[serde(default = "default_checkpoint_dir")]
    pub checkpoint_dir: String,
    #[serde(default = "default_max_checkpoints")]
    pub max_checkpoints: usize,
    /// Coding agent backend: "claude-code" or "opencode".
    #[serde(default = "default_agent_backend")]
    pub agent_backend: String,
    /// Path to the coding agent binary.
    #[serde(default = "default_agent_binary")]
    pub agent_binary: String,
    /// Timeout for coding agent sessions, in seconds.
    #[serde(default = "default_agent_timeout")]
    pub agent_timeout: u64,
    /// Maximum number of agent turns (None = use agent default).
    #[serde(default)]
    pub agent_max_turns: Option<usize>,
    /// Model override for coding agent (e.g. "haiku", "sonnet"). None = agent default.
    #[serde(default)]
    pub agent_model: Option<String>,
    /// Environment variable name for the coding agent's API key.
    /// Defaults to "ANTHROPIC_API_KEY"; set to "DEEPSEEK_API_KEY" for DeepSeek+OpenCode.
    #[serde(default = "default_agent_api_key_env")]
    pub agent_api_key_env: String,
}

impl Default for SelfImproveConfig {
    fn default() -> Self {
        Self {
            source_root: default_source_root(),
            checkpoint_dir: default_checkpoint_dir(),
            max_checkpoints: default_max_checkpoints(),
            agent_backend: default_agent_backend(),
            agent_binary: default_agent_binary(),
            agent_timeout: default_agent_timeout(),
            agent_max_turns: None,
            agent_model: None,
            agent_api_key_env: default_agent_api_key_env(),
        }
    }
}

fn default_source_root() -> String {
    "/usr/src/levsha".to_string()
}
fn default_checkpoint_dir() -> String {
    "/var/lib/levsha/checkpoints".to_string()
}
fn default_max_checkpoints() -> usize {
    10
}
fn default_agent_backend() -> String {
    "claude-code".to_string()
}
fn default_agent_binary() -> String {
    "/usr/local/bin/claude".to_string()
}
fn default_agent_timeout() -> u64 {
    600
}
fn default_agent_api_key_env() -> String {
    "ANTHROPIC_API_KEY".to_string()
}

/// Session configuration (Track J).
#[derive(Debug, Deserialize)]
pub struct SessionsConfig {
    #[serde(default = "default_max_active")]
    pub max_active: usize,
    #[serde(default = "default_auto_name")]
    pub auto_name: bool,
    #[serde(default = "default_session_name")]
    pub default_name: String,
    /// Sidebar visibility preference: None = auto, Some = user override (Module 26).
    #[serde(default)]
    pub sidebar_visible: Option<bool>,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            max_active: default_max_active(),
            auto_name: default_auto_name(),
            default_name: default_session_name(),
            sidebar_visible: None,
        }
    }
}

fn default_max_active() -> usize {
    20
}
fn default_auto_name() -> bool {
    true
}
fn default_session_name() -> String {
    "New Session".to_string()
}

impl Config {
    /// Load configuration from the canonical path.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        let mut config: Config = toml::from_str(&contents)?;
        config.config_path = Some(path.to_path_buf());
        Ok(config)
    }

    /// Default config file path on the target system.
    pub fn default_path() -> &'static Path {
        Path::new("/etc/levsha/config.toml")
    }

    /// Load configuration with dev-mode fallback chain:
    /// 1. `LEVSHA_CONFIG` env var (custom path)
    /// 2. `./config.dev.toml` (local development)
    /// 3. `/etc/levsha/config.toml` (production)
    ///
    /// After loading, overrides `api.key` from `ANTHROPIC_API_KEY` env var if set.
    pub fn load_with_fallback() -> Result<Self, Box<dyn std::error::Error>> {
        let candidates: Vec<PathBuf> = vec![
            std::env::var("LEVSHA_CONFIG").ok().map(PathBuf::from),
            Some(PathBuf::from("./config.dev.toml")),
            Some(Self::default_path().to_path_buf()),
        ]
        .into_iter()
        .flatten()
        .collect();

        let mut config: Option<Self> = None;
        let mut tried: Vec<String> = Vec::new();

        for path in &candidates {
            match Self::load(path) {
                Ok(cfg) => {
                    info!("Loaded config from {}", path.display());
                    config = Some(cfg);
                    break;
                }
                Err(_) => {
                    tried.push(path.display().to_string());
                }
            }
        }

        let mut config = config.ok_or_else(|| -> Box<dyn std::error::Error> {
            format!(
                "No config file found. Tried: {}. \
                 Set LEVSHA_CONFIG env var or create config.dev.toml",
                tried.join(", ")
            )
            .into()
        })?;

        // Override API key from environment if set.
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            config.api.key = api_key;
        }

        // Override DeepSeek API key from environment if set.
        if let Ok(ds_key) = std::env::var("DEEPSEEK_API_KEY") {
            config.deepseek.key = ds_key;
        }

        Ok(config)
    }

    /// Check whether an API key is configured (non-empty and not a placeholder).
    /// When DeepSeek is enabled, checks the DeepSeek key instead.
    pub fn has_key(&self) -> bool {
        if self.deepseek.enabled {
            let key = self.deepseek.key.trim();
            return !key.is_empty();
        }
        let key = self.api.key.trim();
        !key.is_empty()
            && key != "YOUR_API_KEY_HERE"
            && key != "YOUR_ANTHROPIC_API_KEY_HERE"
            && key != "sk-ant-api-placeholder"
    }

    /// Return a masked version of the API key for display (e.g. "sk-ant-...XXXX").
    pub fn masked_key(&self) -> String {
        let key = &self.api.key;
        if key.len() <= 8 {
            return "****".to_string();
        }
        let prefix = &key[..7]; // "sk-ant-" or similar
        let suffix = &key[key.len() - 4..];
        format!("{}...{}", prefix, suffix)
    }

    /// Resolve a model alias to a model ID using the models map.
    pub fn resolve_model(&self, alias: &str) -> Option<String> {
        self.api.models.get(alias).cloned()
    }

    /// Return a user-friendly display name for the current model.
    pub fn current_display_name(&self) -> String {
        if self.deepseek.enabled {
            return format!("DeepSeek ({})", self.deepseek.model);
        }
        let model = &self.api.model;
        // Check if any alias maps to the current model; use the alias as base.
        for (alias, model_id) in &self.api.models {
            if model_id == model {
                return format_model_display_name(alias);
            }
        }
        // Fallback: derive from the model ID itself.
        format_model_display_name(model)
    }

    /// Save an API key to the config file at the given path.
    /// Reads existing config, updates the key, writes TOML, sets file permissions.
    pub fn save_api_key(key: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = load_toml_document(path)?;

        // Ensure [api] table exists.
        if !doc.contains_key("api") {
            doc.insert("api".to_string(), toml::Value::Table(toml::Table::new()));
        }
        if let Some(api_table) = doc.get_mut("api").and_then(|v| v.as_table_mut()) {
            api_table.insert("key".to_string(), toml::Value::String(key.to_string()));
        }

        write_toml_document(path, &doc)?;
        set_file_permissions(path);
        Ok(())
    }

    /// Save sidebar visibility preference to the config file.
    pub fn save_sidebar_preference(visible: bool, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = load_toml_document(path)?;

        // Ensure [sessions] table exists.
        if !doc.contains_key("sessions") {
            doc.insert("sessions".to_string(), toml::Value::Table(toml::Table::new()));
        }
        if let Some(sessions_table) = doc.get_mut("sessions").and_then(|v| v.as_table_mut()) {
            sessions_table.insert("sidebar_visible".to_string(), toml::Value::Boolean(visible));
        }

        write_toml_document(path, &doc)?;
        set_file_permissions(path);
        Ok(())
    }

    /// Save a model ID to the config file at the given path.
    pub fn save_model(model_id: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = load_toml_document(path)?;

        if !doc.contains_key("api") {
            doc.insert("api".to_string(), toml::Value::Table(toml::Table::new()));
        }
        if let Some(api_table) = doc.get_mut("api").and_then(|v| v.as_table_mut()) {
            api_table.insert("model".to_string(), toml::Value::String(model_id.to_string()));
        }

        write_toml_document(path, &doc)?;
        Ok(())
    }
}

/// Load a TOML document from a file path, or create a minimal empty document.
fn load_toml_document(path: &Path) -> Result<toml::Table, Box<dyn std::error::Error>> {
    if path.exists() {
        let contents = std::fs::read_to_string(path)?;
        let doc: toml::Table = contents.parse()?;
        Ok(doc)
    } else {
        Ok(toml::Table::new())
    }
}

/// Write a TOML document to a file path.
fn write_toml_document(path: &Path, doc: &toml::Table) -> Result<(), Box<dyn std::error::Error>> {
    let contents = toml::to_string(doc)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, contents)?;
    Ok(())
}

/// Set restrictive file permissions (0o600) on Unix systems.
#[cfg(unix)]
fn set_file_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut perms = metadata.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(path, perms).ok();
    }
}

#[cfg(not(unix))]
fn set_file_permissions(_path: &Path) {
    // No-op on non-Unix platforms.
}

/// Derive a user-friendly model display name from a model ID or alias.
fn format_model_display_name(model: &str) -> String {
    if model.contains("opus") {
        "Claude Opus".to_string()
    } else if model.contains("sonnet") {
        "Claude Sonnet".to_string()
    } else if model.contains("haiku") {
        "Claude Haiku".to_string()
    } else {
        model.to_string()
    }
}
