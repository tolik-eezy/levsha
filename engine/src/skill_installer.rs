//! Skill installer — installs, removes, updates, and validates skills.
//!
//! Skills can be installed from a git URL or by name via the registry.
//! Built-in skills cannot be removed or overwritten.

use crate::skill_loader::{self, SkillManifest};
use crate::skill_registry::RegistryClient;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Information about an installed skill.
#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub builtin: bool,
    pub tool_count: usize,
}

/// Handles skill installation, removal, and updates.
pub struct SkillInstaller {
    pub builtin_path: PathBuf,
    pub installed_path: PathBuf,
    pub staging_path: PathBuf,
}

impl SkillInstaller {
    pub fn new(builtin_path: PathBuf, installed_path: PathBuf, staging_path: PathBuf) -> Self {
        Self {
            builtin_path,
            installed_path,
            staging_path,
        }
    }

    /// Install a skill from a git URL.
    /// Returns the name of the installed skill.
    pub async fn install_from_url(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Create staging directory.
        std::fs::create_dir_all(&self.staging_path)?;

        // Derive a temporary name from the URL.
        let temp_name = url
            .rsplit('/')
            .next()
            .unwrap_or("skill")
            .trim_end_matches(".git");
        let staging_dir = self.staging_path.join(temp_name);

        // Clean up any previous staging attempt.
        if staging_dir.exists() {
            std::fs::remove_dir_all(&staging_dir)?;
        }

        // Clone the repository.
        info!("Cloning skill from {} to {:?}", url, staging_dir);
        let output = tokio::time::timeout(
            Duration::from_secs(60),
            Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    url,
                    &staging_dir.to_string_lossy(),
                ])
                .output(),
        )
        .await
        .map_err(|_| "Skill clone timed out after 60s")??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("git clone failed: {}", stderr).into());
        }

        // Validate the manifest.
        self.validate_manifest(&staging_dir)?;

        // Read the manifest to get the skill name.
        let manifest_path = staging_dir.join("skill.yaml");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let manifest: SkillManifest = serde_yaml::from_str(&manifest_content)?;
        let skill_name = manifest.name.clone();

        // Check if this would overwrite a builtin skill.
        if self.builtin_path.join(&skill_name).exists() {
            // Clean up staging.
            std::fs::remove_dir_all(&staging_dir).ok();
            return Err(format!(
                "Cannot install '{}': a built-in skill with that name already exists",
                skill_name
            )
            .into());
        }

        // Install any dnf dependencies if specified.
        self.install_deps(&staging_dir).await?;

        // Move from staging to installed.
        std::fs::create_dir_all(&self.installed_path)?;
        let final_dir = self.installed_path.join(&skill_name);
        if final_dir.exists() {
            std::fs::remove_dir_all(&final_dir)?;
        }
        rename_or_copy(&staging_dir, &final_dir)?;

        info!("Installed skill '{}'", skill_name);
        Ok(skill_name)
    }

    /// Install a skill by name using the registry.
    pub async fn install_by_name(
        &self,
        name: &str,
        registry: &RegistryClient,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let entry = registry
            .find(name)
            .ok_or_else(|| format!("Skill '{}' not found in the registry", name))?;

        self.install_from_url(&entry.repository).await
    }

    /// Remove an installed skill by name.
    pub fn remove(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Ensure it is not a builtin skill.
        if self.builtin_path.join(name).exists() {
            return Err(format!("Cannot remove built-in skill '{}'", name).into());
        }

        let skill_dir = self.installed_path.join(name);
        if !skill_dir.exists() {
            return Err(format!("Skill '{}' is not installed", name).into());
        }

        std::fs::remove_dir_all(&skill_dir)?;
        info!("Removed skill '{}'", name);
        Ok(())
    }

    /// Update an installed skill. Creates a backup, removes, re-installs,
    /// and restores the backup on failure.
    pub async fn update(
        &self,
        name: &str,
        registry: &RegistryClient,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Check that the skill exists and is not builtin.
        if self.builtin_path.join(name).exists() {
            return Err(format!("Cannot update built-in skill '{}'", name).into());
        }

        let skill_dir = self.installed_path.join(name);
        if !skill_dir.exists() {
            return Err(format!("Skill '{}' is not installed", name).into());
        }

        // Create a backup.
        let backup_dir = self.staging_path.join(format!("{}-backup", name));
        if backup_dir.exists() {
            std::fs::remove_dir_all(&backup_dir)?;
        }
        std::fs::create_dir_all(&self.staging_path)?;
        rename_or_copy(&skill_dir, &backup_dir)?;

        // Attempt to install the new version.
        match self.install_by_name(name, registry).await {
            Ok(result) => {
                // Success — remove the backup.
                std::fs::remove_dir_all(&backup_dir).ok();
                Ok(result)
            }
            Err(e) => {
                // Failed — restore the backup.
                warn!("Update failed for '{}', restoring backup: {}", name, e);
                if skill_dir.exists() {
                    std::fs::remove_dir_all(&skill_dir).ok();
                }
                rename_or_copy(&backup_dir, &skill_dir).ok();
                Err(e)
            }
        }
    }

    /// List all installed and built-in skills.
    pub fn list_all(&self) -> Vec<SkillInfo> {
        let mut skills = Vec::new();

        // Scan builtin skills.
        skills.extend(self.scan_directory(&self.builtin_path, true));

        // Scan installed skills.
        skills.extend(self.scan_directory(&self.installed_path, false));

        skills
    }

    /// Get detailed info for a single skill by name.
    pub fn get_info(&self, name: &str) -> Option<SkillInfo> {
        // Check builtin first.
        let builtin_dir = self.builtin_path.join(name);
        if builtin_dir.is_dir() {
            return self.read_skill_info(&builtin_dir, true);
        }

        // Check installed.
        let installed_dir = self.installed_path.join(name);
        if installed_dir.is_dir() {
            return self.read_skill_info(&installed_dir, false);
        }

        None
    }

    /// Validate that a skill directory contains a valid manifest.
    pub fn validate_manifest(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let manifest_path = path.join("skill.yaml");
        if !manifest_path.exists() {
            return Err("No skill.yaml found in skill directory".into());
        }

        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: SkillManifest = serde_yaml::from_str(&content)?;

        // Validate name: only lowercase alphanumeric and hyphens.
        let name_re = Regex::new(r"^[a-z][a-z0-9\-]*$").unwrap();
        if !name_re.is_match(&manifest.name) {
            return Err(format!(
                "Invalid skill name '{}': must be lowercase alphanumeric with hyphens",
                manifest.name
            )
            .into());
        }

        // Validate version if present.
        if let Some(version) = &manifest.version {
            if semver::Version::parse(version).is_err() {
                return Err(format!(
                    "Invalid semver version '{}' in skill manifest",
                    version
                )
                .into());
            }
        }

        // Validate prompt file exists if specified.
        if let Some(prompt_path) = &manifest.prompt {
            let full_path = path.join(prompt_path);
            if !full_path.exists() {
                return Err(format!(
                    "Prompt file '{}' referenced in manifest does not exist",
                    prompt_path
                )
                .into());
            }
        }

        // Validate tool JSON files parse correctly.
        if let Some(tool_paths) = &manifest.tools {
            for tool_path in tool_paths {
                let full_path = path.join(tool_path);
                if !full_path.exists() {
                    return Err(format!(
                        "Tool file '{}' referenced in manifest does not exist",
                        tool_path
                    )
                    .into());
                }
                let tool_content = std::fs::read_to_string(&full_path)?;
                let _: serde_json::Value = serde_json::from_str(&tool_content).map_err(|e| {
                    format!("Tool file '{}' is not valid JSON: {}", tool_path, e)
                })?;
            }
        }

        Ok(())
    }

    /// Scan a directory for skills and return their info.
    fn scan_directory(&self, dir: &Path, builtin: bool) -> Vec<SkillInfo> {
        let mut skills = Vec::new();

        if !dir.is_dir() {
            return skills;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read skill directory {:?}: {}", dir, e);
                return skills;
            }
        };

        for entry in entries.flatten() {
            let skill_dir = entry.path();
            if !skill_dir.is_dir() {
                continue;
            }
            if let Some(info) = self.read_skill_info(&skill_dir, builtin) {
                skills.push(info);
            }
        }

        skills
    }

    /// Read skill info from a skill directory.
    fn read_skill_info(&self, skill_dir: &Path, builtin: bool) -> Option<SkillInfo> {
        let manifest_path = skill_dir.join("skill.yaml");
        if !manifest_path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&manifest_path).ok()?;
        let manifest: SkillManifest = serde_yaml::from_str(&content).ok()?;

        // Count tools by trying to load them.
        let tool_count = match skill_loader::load_single_skill(
            &skill_dir.to_path_buf(),
            &manifest_path,
        ) {
            Ok((_, tools)) => tools.len(),
            Err(_) => 0,
        };

        Some(SkillInfo {
            name: manifest.name,
            version: manifest.version.unwrap_or_else(|| "0.0.0".to_string()),
            description: manifest
                .description
                .unwrap_or_else(|| "No description".to_string()),
            author: String::new(),
            builtin,
            tool_count,
        })
    }

    /// Install system dependencies via dnf if a `deps.yaml` file exists.
    async fn install_deps(&self, skill_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let deps_path = skill_dir.join("deps.yaml");
        if !deps_path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&deps_path)?;
        let deps: serde_yaml::Value = serde_yaml::from_str(&content)?;

        if let Some(packages) = deps.get("dnf").and_then(|v| v.as_sequence()) {
            let pkg_names: Vec<String> = packages
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();

            if pkg_names.is_empty() {
                return Ok(());
            }

            info!("Installing dnf dependencies: {:?}", pkg_names);
            let mut args = vec!["install", "-y"];
            let pkg_refs: Vec<&str> = pkg_names.iter().map(|s| s.as_str()).collect();
            args.extend(pkg_refs);

            let output = tokio::time::timeout(
                Duration::from_secs(120),
                Command::new("dnf").args(&args).output(),
            )
            .await
            .map_err(|_| "dnf install timed out after 120s")??;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("dnf install failed: {}", stderr);
                return Err(format!("dnf install failed: {}", stderr).into());
            }
        }

        Ok(())
    }
}

/// Move a directory, falling back to recursive copy if rename fails
/// (e.g., across filesystems).
fn rename_or_copy(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }

    // Fallback: recursive copy then remove source.
    debug!(
        "Rename failed, falling back to copy {:?} -> {:?}",
        src, dst
    );
    copy_dir_recursive(src, dst)?;
    std::fs::remove_dir_all(src)?;
    Ok(())
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
