//! Skill discovery and loading.
//!
//! Discovers skills from /usr/share/levsha/skills/, parses YAML manifests,
//! loads prompt fragments and tool JSON schemas.

use serde::Deserialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// A tool definition loaded from a skill's JSON file.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub execution: ExecutionConfig,
}

/// How a tool is executed by the engine.
#[derive(Debug, Clone, Deserialize)]
pub struct ExecutionConfig {
    #[serde(rename = "type")]
    pub exec_type: String,
    pub command_template: Option<String>,
    pub function: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub run_as: Option<String>,
    #[serde(default)]
    pub streaming: Option<bool>,
    #[serde(default)]
    pub content_type: Option<String>,
}

/// Raw tool definition as read from JSON.
#[derive(Debug, Deserialize)]
struct RawToolDefinition {
    name: String,
    description: String,
    input_schema: serde_json::Value,
    execution: ExecutionConfig,
}

/// Skill manifest as read from skill.yaml.
#[derive(Debug, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    #[allow(dead_code)]
    pub version: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub prompt: Option<String>,
    pub tools: Option<Vec<String>>,
}

/// All loaded skills — their prompts and tool definitions.
#[derive(Debug, Clone)]
pub struct LoadedSkills {
    pub prompts: Vec<String>,
    pub tools: Vec<ToolDefinition>,
}

impl LoadedSkills {
    pub fn empty() -> Self {
        Self {
            prompts: Vec::new(),
            tools: Vec::new(),
        }
    }
}

/// Load all skills from the given directory.
///
/// Walks the directory for `*/skill.yaml` manifests, parses them,
/// reads prompt files and tool JSON definitions.
pub fn load_skills(path: &str) -> LoadedSkills {
    let skills_dir = Path::new(path);

    if !skills_dir.is_dir() {
        warn!("Skills directory does not exist: {}", path);
        return LoadedSkills::empty();
    }

    let mut prompts = Vec::new();
    let mut tools = Vec::new();
    let mut seen_tools: HashSet<String> = HashSet::new();

    let entries = match std::fs::read_dir(skills_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read skills directory: {}", e);
            return LoadedSkills::empty();
        }
    };

    for entry in entries.flatten() {
        let skill_dir = entry.path();
        if !skill_dir.is_dir() {
            continue;
        }

        let manifest_path = skill_dir.join("skill.yaml");
        if !manifest_path.exists() {
            debug!("No skill.yaml in {:?}, skipping", skill_dir);
            continue;
        }

        match load_single_skill(&skill_dir, &manifest_path) {
            Ok((skill_prompts, skill_tools)) => {
                let mut tool_count = 0;
                prompts.extend(skill_prompts);
                for tool in skill_tools {
                    if seen_tools.contains(&tool.name) {
                        warn!(
                            "Skipping duplicate tool '{}' from skill '{}'",
                            tool.name,
                            skill_dir.file_name().unwrap_or_default().to_string_lossy()
                        );
                        continue;
                    }
                    seen_tools.insert(tool.name.clone());
                    tools.push(tool);
                    tool_count += 1;
                }
                info!(
                    "Loaded skill '{}' with {} tools",
                    skill_dir.file_name().unwrap_or_default().to_string_lossy(),
                    tool_count
                );
            }
            Err(e) => {
                warn!("Failed to load skill from {:?}: {}", skill_dir, e);
            }
        }
    }

    info!(
        "Loaded {} skill prompt(s) and {} tool definition(s)",
        prompts.len(),
        tools.len()
    );

    LoadedSkills { prompts, tools }
}

/// Load skills from multiple directories. Builtin directories come first;
/// on name collision the earlier (builtin) skill wins.
pub fn load_skills_multi(paths: &[&str]) -> LoadedSkills {
    let mut prompts = Vec::new();
    let mut tools = Vec::new();
    let mut seen_tools: HashSet<String> = HashSet::new();

    for path in paths {
        let partial = load_skills(path);
        prompts.extend(partial.prompts);
        for tool in partial.tools {
            if seen_tools.contains(&tool.name) {
                info!(
                    "Skipping duplicate tool '{}' from path '{}'",
                    tool.name, path
                );
                continue;
            }
            seen_tools.insert(tool.name.clone());
            tools.push(tool);
        }
    }

    LoadedSkills { prompts, tools }
}

/// Load a single skill from its directory. Made `pub` so the installer
/// can validate skills before copying them into the installed directory.
pub fn load_single_skill(
    skill_dir: &PathBuf,
    manifest_path: &Path,
) -> Result<(Vec<String>, Vec<ToolDefinition>), Box<dyn std::error::Error>> {
    let manifest_content = std::fs::read_to_string(manifest_path)?;
    let manifest: SkillManifest = serde_yaml::from_str(&manifest_content)?;

    let mut prompts = Vec::new();
    let mut tools = Vec::new();

    // Load prompt file if specified.
    if let Some(prompt_path) = &manifest.prompt {
        let full_path = skill_dir.join(prompt_path);
        match std::fs::read_to_string(&full_path) {
            Ok(content) => {
                debug!(
                    "Loaded prompt for skill '{}' from {:?}",
                    manifest.name, full_path
                );
                prompts.push(content);
            }
            Err(e) => {
                warn!(
                    "Failed to read prompt file {:?} for skill '{}': {}",
                    full_path, manifest.name, e
                );
            }
        }
    }

    // Load tool definitions.
    if let Some(tool_paths) = &manifest.tools {
        for tool_path in tool_paths {
            let full_path = skill_dir.join(tool_path);
            match load_tool_definition(&full_path) {
                Ok(tool) => {
                    debug!("Loaded tool '{}' from {:?}", tool.name, full_path);
                    tools.push(tool);
                }
                Err(e) => {
                    warn!("Failed to load tool from {:?}: {}. Skipping.", full_path, e);
                }
            }
        }
    }

    Ok((prompts, tools))
}

/// Load a single tool definition from a JSON file.
fn load_tool_definition(path: &Path) -> Result<ToolDefinition, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let raw: RawToolDefinition = serde_json::from_str(&content)?;

    // Validate basics.
    if raw.name.is_empty() {
        return Err("Tool name is empty".into());
    }
    if raw.description.is_empty() {
        return Err(format!("Tool '{}' has empty description", raw.name).into());
    }
    if raw.execution.exec_type == "shell" && raw.execution.command_template.is_none() {
        return Err(format!("Shell tool '{}' has no command_template", raw.name).into());
    }

    Ok(ToolDefinition {
        name: raw.name,
        description: raw.description,
        input_schema: raw.input_schema,
        execution: raw.execution,
    })
}
