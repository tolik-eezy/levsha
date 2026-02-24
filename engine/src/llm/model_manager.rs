//! Local model management (Track D).
//!
//! Scans and manages GGUF model files in the models directory.
//! Provides listing, selection, and deletion of local models.

use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Information about a local model file.
#[derive(Debug, Clone)]
pub struct LocalModelInfo {
    /// Model filename (without path).
    pub name: String,
    /// Full path to the model file.
    pub path: PathBuf,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// Manages local GGUF model files.
pub struct ModelManager {
    models_dir: PathBuf,
    current_model: Option<String>,
}

impl ModelManager {
    /// Create a new model manager for the given directory.
    pub fn new(models_dir: &str) -> Self {
        Self {
            models_dir: PathBuf::from(models_dir),
            current_model: None,
        }
    }

    /// List all available GGUF models in the models directory.
    pub fn list_models(&self) -> Vec<LocalModelInfo> {
        let mut models = Vec::new();

        if !self.models_dir.is_dir() {
            warn!("Models directory does not exist: {:?}", self.models_dir);
            return models;
        }

        let entries = match std::fs::read_dir(&self.models_dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read models directory: {}", e);
                return models;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                if let Ok(metadata) = entry.metadata() {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    models.push(LocalModelInfo {
                        name,
                        path,
                        size_bytes: metadata.len(),
                    });
                }
            }
        }

        models.sort_by(|a, b| a.name.cmp(&b.name));
        models
    }

    /// Get the current model name.
    pub fn current_model(&self) -> Option<&str> {
        self.current_model.as_deref()
    }

    /// Switch to a different model by name.
    pub fn switch_model(&mut self, name: &str) -> Result<PathBuf, String> {
        let model_path = self.models_dir.join(name);
        if !model_path.exists() {
            return Err(format!("Model '{}' not found in {:?}", name, self.models_dir));
        }
        self.current_model = Some(name.to_string());
        info!("Switched to local model: {}", name);
        Ok(model_path)
    }

    /// Delete a model file.
    pub fn delete_model(&self, name: &str) -> Result<(), String> {
        let model_path = self.models_dir.join(name);
        if !model_path.exists() {
            return Err(format!("Model '{}' not found", name));
        }

        // Validate path is actually under models_dir to prevent path traversal.
        let canonical = model_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve path: {}", e))?;
        let dir_canonical = self
            .models_dir
            .canonicalize()
            .map_err(|e| format!("Failed to resolve models dir: {}", e))?;

        if !canonical.starts_with(&dir_canonical) {
            return Err("Path traversal detected".to_string());
        }

        std::fs::remove_file(&canonical)
            .map_err(|e| format!("Failed to delete model: {}", e))?;
        info!("Deleted local model: {}", name);
        Ok(())
    }

    /// Get the models directory path.
    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }
}
