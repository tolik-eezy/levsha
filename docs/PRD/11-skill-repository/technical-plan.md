# 11 — Skill Repository: Technical Plan

**Module:** Skill Install/Remove from Repositories
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

```
engine/
  src/
    skills/
      mod.rs              # Extended: hot-reload, installed skill tracking
      repository.rs       # Skill registry and Git-based install/remove
      manifest.rs         # Skill manifest parsing and validation
      resolver.rs         # Dependency resolution (system packages)

skills/
  built-in/
    skill-manager/
      skill.yaml
      prompts/
        skill-manager.md
      tools/
        skill_install.json
        skill_remove.json
        skill_list.json
        skill_search.json
        skill_update.json
        skill_info.json
```

### Dependencies

```toml
# Added to engine/Cargo.toml
git2 = "0.19"          # Git clone/pull (shared with self-improvement)
semver = "1"           # Version comparison
```

---

## 2. Skill Registry Client

```rust
pub struct RegistryClient {
    registry_url: String,
    cache_dir: PathBuf,    // /var/cache/levsha/skill-registry/
}

#[derive(Deserialize)]
pub struct RegistryIndex {
    pub skills: Vec<RegistryEntry>,
}

#[derive(Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub description: String,
    pub repository: String,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
}

impl RegistryClient {
    /// Fetch or update the cached registry index
    pub fn refresh(&self) -> Result<RegistryIndex> {
        let cache_path = self.cache_dir.join("index.yaml");

        // Clone or pull the registry repo
        if cache_path.exists() {
            self.git_pull()?;
        } else {
            self.git_clone()?;
        }

        let content = std::fs::read_to_string(&cache_path)?;
        let index: RegistryIndex = serde_yaml::from_str(&content)?;
        Ok(index)
    }

    pub fn search(&self, query: &str) -> Result<Vec<RegistryEntry>> {
        let index = self.refresh()?;
        let query_lower = query.to_lowercase();
        Ok(index.skills.into_iter().filter(|s| {
            s.name.to_lowercase().contains(&query_lower)
                || s.description.to_lowercase().contains(&query_lower)
                || s.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
        }).collect())
    }

    pub fn find(&self, name: &str) -> Result<Option<RegistryEntry>> {
        let index = self.refresh()?;
        Ok(index.skills.into_iter().find(|s| s.name == name))
    }
}
```

---

## 3. Skill Installer

```rust
pub struct SkillInstaller {
    installed_path: PathBuf,   // /usr/share/levsha/skills/installed/
    staging_path: PathBuf,     // /tmp/levsha-skill-staging/
}

pub struct InstallResult {
    pub skill_name: String,
    pub version: String,
    pub tools_loaded: usize,
}

impl SkillInstaller {
    pub async fn install_from_url(&self, url: &str) -> Result<InstallResult> {
        // 1. Clone to staging
        let staging = self.staging_path.join(uuid::Uuid::new_v4().to_string());
        git_clone_shallow(url, &staging)?;

        // 2. Find and validate manifest
        let manifest_path = staging.join("skill.yaml");
        let manifest = SkillManifest::load_and_validate(&manifest_path)?;

        // 3. Check not already installed
        let dest = self.installed_path.join(&manifest.name);
        if dest.exists() {
            return Err(InstallError::AlreadyInstalled(manifest.name));
        }

        // 4. Install system dependencies
        if let Some(requires) = &manifest.requires {
            if let Some(packages) = &requires.packages {
                self.install_system_packages(packages).await?;
            }
        }

        // 5. Copy to installed directory
        copy_dir_recursive(&staging, &dest)?;

        // 6. Cleanup staging
        std::fs::remove_dir_all(&staging)?;

        Ok(InstallResult {
            skill_name: manifest.name,
            version: manifest.version,
            tools_loaded: manifest.tools.len(),
        })
    }

    pub async fn install_by_name(
        &self,
        name: &str,
        registry: &RegistryClient,
    ) -> Result<InstallResult> {
        let entry = registry.find(name)?
            .ok_or(InstallError::NotFound(name.to_string()))?;
        self.install_from_url(&entry.repository).await
    }

    pub fn remove(&self, name: &str) -> Result<()> {
        let path = self.installed_path.join(name);
        if !path.exists() {
            return Err(InstallError::NotInstalled(name.to_string()));
        }

        // Check if built-in
        let manifest = SkillManifest::load(&path.join("skill.yaml"))?;
        if manifest.builtin.unwrap_or(false) {
            return Err(InstallError::CannotRemoveBuiltin(name.to_string()));
        }

        std::fs::remove_dir_all(&path)?;
        Ok(())
    }

    pub fn list_installed(&self) -> Result<Vec<SkillManifest>> {
        let mut skills = Vec::new();
        for entry in std::fs::read_dir(&self.installed_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let manifest_path = entry.path().join("skill.yaml");
                if manifest_path.exists() {
                    skills.push(SkillManifest::load(&manifest_path)?);
                }
            }
        }
        Ok(skills)
    }

    pub async fn update(&self, name: &str, registry: &RegistryClient) -> Result<InstallResult> {
        let entry = registry.find(name)?
            .ok_or(InstallError::NotFound(name.to_string()))?;

        let current = self.installed_path.join(name).join("skill.yaml");
        let current_manifest = SkillManifest::load(&current)?;

        if semver::Version::parse(&entry.version)? <= semver::Version::parse(&current_manifest.version)? {
            return Err(InstallError::AlreadyUpToDate(name.to_string()));
        }

        // Backup current, install new
        let backup = self.staging_path.join(format!("{}-backup", name));
        copy_dir_recursive(&self.installed_path.join(name), &backup)?;

        self.remove(name)?;
        match self.install_from_url(&entry.repository).await {
            Ok(result) => {
                std::fs::remove_dir_all(&backup)?;
                Ok(result)
            }
            Err(e) => {
                // Restore backup on failure
                copy_dir_recursive(&backup, &self.installed_path.join(name))?;
                std::fs::remove_dir_all(&backup)?;
                Err(e)
            }
        }
    }

    async fn install_system_packages(&self, packages: &[String]) -> Result<()> {
        let output = tokio::process::Command::new("dnf")
            .arg("install").arg("-y")
            .args(packages)
            .output()
            .await?;
        if !output.status.success() {
            return Err(InstallError::PackageInstallFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }
        Ok(())
    }
}
```

---

## 4. Manifest Validation

```rust
#[derive(Deserialize, Serialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub builtin: Option<bool>,
    pub prompt: String,
    pub tools: Vec<String>,
    pub requires: Option<SkillRequires>,
}

#[derive(Deserialize, Serialize)]
pub struct SkillRequires {
    pub packages: Option<Vec<String>>,
}

impl SkillManifest {
    pub fn load_and_validate(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = serde_yaml::from_str(&content)
            .map_err(|e| ManifestError::ParseFailed(e.to_string()))?;

        // Validate required fields
        if manifest.name.is_empty() {
            return Err(ManifestError::MissingField("name".into()));
        }
        if manifest.version.is_empty() {
            return Err(ManifestError::MissingField("version".into()));
        }
        if manifest.tools.is_empty() {
            return Err(ManifestError::MissingField("tools".into()));
        }

        // Validate tool JSON files exist
        let base_dir = path.parent().unwrap();
        for tool_path in &manifest.tools {
            let tool_file = base_dir.join(tool_path);
            if !tool_file.exists() {
                return Err(ManifestError::MissingTool(tool_path.clone()));
            }
            // Validate tool JSON schema
            let tool_json = std::fs::read_to_string(&tool_file)?;
            serde_json::from_str::<serde_json::Value>(&tool_json)
                .map_err(|e| ManifestError::InvalidTool(tool_path.clone(), e.to_string()))?;
        }

        // Validate prompt file exists
        let prompt_file = base_dir.join(&manifest.prompt);
        if !prompt_file.exists() {
            return Err(ManifestError::MissingPrompt(manifest.prompt.clone()));
        }

        Ok(manifest)
    }
}
```

---

## 5. Hot-Reload Mechanism

```rust
impl SkillLoader {
    /// Re-scan the skills directory and reload all skills.
    /// Called after install, remove, or update.
    pub fn reload(&mut self) -> Result<ReloadResult> {
        let mut loaded = Vec::new();
        let mut errors = Vec::new();

        // Re-scan built-in and installed directories
        for dir in &[&self.builtin_path, &self.installed_path] {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    match self.load_skill(&entry.path()) {
                        Ok(skill) => loaded.push(skill),
                        Err(e) => errors.push((entry.path(), e)),
                    }
                }
            }
        }

        // Replace the current skill set atomically
        self.skills = loaded.iter().map(|s| (s.name.clone(), s.clone())).collect();

        // Recompose the system prompt with updated skill prompts
        self.recompose_system_prompt();

        // Rebuild the tool registry
        self.rebuild_tool_registry();

        Ok(ReloadResult {
            loaded_count: loaded.len(),
            error_count: errors.len(),
            errors,
        })
    }
}
```

---

## 6. Implementation Stages

**Stage 1 — Manifest & Validation (1 day)**
1. Implement `SkillManifest` parsing and validation.
2. Unit tests for valid and invalid manifests.

**Stage 2 — Registry Client (1 day)**
1. Implement `RegistryClient` (Git clone, index parsing, search).
2. Create test registry with sample skills.

**Stage 3 — Installer (2 days)**
1. Implement `SkillInstaller` (install from URL, install by name, remove).
2. System package dependency resolution.
3. Integration test: install skill from test repo.

**Stage 4 — Hot-Reload (1 day)**
1. Extend `SkillLoader` with `reload()` method.
2. Wire reload into install/remove/update flows.
3. Verify new skills are available on next message.

**Stage 5 — Update & Polish (1 day)**
1. Implement skill update with version comparison.
2. Create tool JSON schemas and skill manifest.
3. End-to-end test: install, use, update, remove.

---

## 7. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `manifest.rs` | Valid/invalid YAML, missing fields, missing tool files |
| `repository.rs` | Registry index parsing, search filtering |
| `resolver.rs` | Package dependency extraction |

### Integration Tests

| Test | Method |
|------|--------|
| Install from URL | Clone test repo, validate, install, verify skill loaded |
| Install by name | Mock registry, resolve name, install |
| Remove skill | Install then remove, verify skill unloaded |
| Update skill | Install v1, update to v2, verify new version |
| Hot-reload | Install skill, send message, verify new tool available |
| Invalid manifest | Attempt install with bad manifest, verify error |
| Built-in protection | Attempt to remove built-in skill, verify rejection |
| Rollback on failed update | Corrupt staging, verify old version preserved |
