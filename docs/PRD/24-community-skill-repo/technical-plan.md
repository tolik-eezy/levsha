# 24 — Community Skill Repository: Technical Plan

**Module:** Community Skill Repository
**Language:** Rust
**Phase:** 3

---

## 1. Crate Structure

```
engine/
  src/
    skills/
      mod.rs                # Extended: dependency resolution, update checking
      repository.rs         # Extended: HTTP registry client (replaces Git-only)
      manifest.rs           # Extended: rich metadata, version compatibility
      resolver.rs           # Extended: skill-to-skill dependency resolution
      registry_client.rs    # New: HTTP client for the community registry API
      update_checker.rs     # New: background update checker
      trust.rs              # New: trust level handling

chat-shell/
  src/
    ui/
      skill_detail.rs       # New: skill detail panel widget (split-view)
      category_browser.rs   # New: category grid panel widget (split-view)
    widgets/
      trust_badge.rs        # New: trust level badge rendering
      star_rating.rs        # New: star rating display widget

skills/
  built-in/
    skill-manager/
      tools/
        skill_search.json   # Modified: adds filters, trust level, sort
        skill_info.json     # Modified: returns rich metadata
        skill_rate.json     # New: rate a skill
        skill_publish.json  # New: publish a skill
        skill_browse.json   # New: browse categories
        skill_check_updates.json  # New: check for updates
```

### Dependencies

```toml
# Added to engine/Cargo.toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }  # HTTP client
semver = "1"           # Version comparison (already present from Module 11)
```

---

## 2. Registry HTTP Client

```rust
pub struct RegistryHttpClient {
    base_url: String,
    client: reqwest::Client,
    device_id: String,
}

#[derive(Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub long_description: Option<String>,
    pub tags: Vec<String>,
    pub trust_level: TrustLevel,
    pub downloads: u64,
    pub avg_rating: f32,
    pub rating_count: u32,
    pub dependencies: Vec<SkillDependency>,
    pub compatible_versions: Option<VersionRange>,
    pub created_at: i64,
    pub updated_at: i64,
    pub manifest_url: String,
}

#[derive(Deserialize)]
pub struct VersionRange {
    pub min: String,
    pub max: String,
}

#[derive(Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub dep_type: DependencyType,  // "skill" or "package"
    pub version_constraint: Option<String>,  // e.g., ">=1.0.0, <2.0.0"
}

#[derive(Deserialize)]
pub enum DependencyType {
    #[serde(rename = "skill")]
    Skill,
    #[serde(rename = "package")]
    Package,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub query: Option<String>,
    pub tags: Option<Vec<String>>,
    pub trust_level: Option<TrustLevel>,
    pub compatible_version: Option<String>,
    pub sort: Option<SortOrder>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Deserialize)]
pub enum SortOrder {
    #[serde(rename = "downloads")]
    Downloads,
    #[serde(rename = "rating")]
    Rating,
    #[serde(rename = "newest")]
    Newest,
}

#[derive(Deserialize)]
pub struct SearchResult {
    pub skills: Vec<SkillMetadata>,
    pub total: u64,
}

#[derive(Deserialize)]
pub struct SkillVersion {
    pub version: String,
    pub changelog: Option<String>,
    pub published_at: i64,
    pub archive_url: String,
}

impl RegistryHttpClient {
    pub fn new(base_url: &str, device_id: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            device_id: device_id.to_string(),
        }
    }

    /// Search for skills with optional filters
    pub async fn search(&self, params: &SearchParams) -> Result<SearchResult> {
        let mut url = format!("{}/skills/search", self.base_url);
        let mut query_parts = Vec::new();

        if let Some(q) = &params.query {
            query_parts.push(format!("q={}", urlencoding::encode(q)));
        }
        if let Some(tags) = &params.tags {
            query_parts.push(format!("tags={}", tags.join(",")));
        }
        if let Some(trust) = &params.trust_level {
            query_parts.push(format!("trust={}", trust.as_str()));
        }
        if let Some(sort) = &params.sort {
            query_parts.push(format!("sort={}", sort.as_str()));
        }

        if !query_parts.is_empty() {
            url = format!("{}?{}", url, query_parts.join("&"));
        }

        let response = self.client.get(&url).send().await?;
        let result: SearchResult = response.json().await?;
        Ok(result)
    }

    /// Get full skill details including version history
    pub async fn get_skill(&self, skill_id: &str) -> Result<(SkillMetadata, Vec<SkillVersion>)> {
        let url = format!("{}/skills/{}", self.base_url, skill_id);
        let response = self.client.get(&url).send().await?;

        #[derive(Deserialize)]
        struct DetailResponse {
            skill: SkillMetadata,
            versions: Vec<SkillVersion>,
        }

        let detail: DetailResponse = response.json().await?;
        Ok((detail.skill, detail.versions))
    }

    /// Download a skill archive for installation
    pub async fn download_skill(&self, skill_id: &str, version: &str) -> Result<Vec<u8>> {
        let url = format!("{}/skills/{}/download?version={}", self.base_url, skill_id, version);
        let response = self.client.get(&url).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Submit a rating for an installed skill
    pub async fn rate_skill(&self, skill_id: &str, rating: u8) -> Result<()> {
        let url = format!("{}/skills/{}/rate", self.base_url, skill_id);
        let body = serde_json::json!({
            "device_id": self.device_id,
            "rating": rating,
        });
        let response = self.client.post(&url).json(&body).send().await?;
        if !response.status().is_success() {
            return Err(RegistryError::RatingFailed(response.status().to_string()));
        }
        Ok(())
    }

    /// Publish a skill to the registry
    pub async fn publish_skill(&self, archive: Vec<u8>, metadata: &SkillMetadata) -> Result<String> {
        let url = format!("{}/skills/publish", self.base_url);
        let form = reqwest::multipart::Form::new()
            .text("name", metadata.name.clone())
            .text("display_name", metadata.display_name.clone())
            .text("version", metadata.version.clone())
            .text("description", metadata.description.clone())
            .text("device_id", self.device_id.clone())
            .part("archive", reqwest::multipart::Part::bytes(archive)
                .file_name("skill.tar.gz")
                .mime_str("application/gzip")?);

        let response = self.client.post(&url).multipart(form).send().await?;
        let result: serde_json::Value = response.json().await?;
        Ok(result["skill_id"].as_str().unwrap_or("").to_string())
    }
}
```

---

## 3. Trust Level

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    #[serde(rename = "official")]
    Official,
    #[serde(rename = "verified")]
    Verified,
    #[serde(rename = "community")]
    Community,
}

impl TrustLevel {
    pub fn as_str(&self) -> &str {
        match self {
            TrustLevel::Official => "official",
            TrustLevel::Verified => "verified",
            TrustLevel::Community => "community",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            TrustLevel::Official => "Official",
            TrustLevel::Verified => "Verified",
            TrustLevel::Community => "Community",
        }
    }

    pub fn badge_icon(&self) -> &str {
        match self {
            TrustLevel::Official => "shield-check",
            TrustLevel::Verified => "check-circle",
            TrustLevel::Community => "circle",
        }
    }

    pub fn requires_warning(&self) -> bool {
        matches!(self, TrustLevel::Community)
    }
}
```

---

## 4. Dependency Resolver

```rust
pub struct DependencyResolver {
    registry: Arc<RegistryHttpClient>,
    installer: Arc<SkillInstaller>,
}

#[derive(Debug)]
pub struct ResolvedDependency {
    pub name: String,
    pub dep_type: DependencyType,
    pub version: String,
    pub already_installed: bool,
}

#[derive(Debug)]
pub struct DependencyTree {
    pub root: String,
    pub root_version: String,
    pub dependencies: Vec<ResolvedDependency>,
    pub install_order: Vec<String>,  // Topological order
}

impl DependencyResolver {
    /// Resolve all dependencies for a skill, returning the full dependency tree
    pub async fn resolve(&self, skill_id: &str) -> Result<DependencyTree> {
        let (skill, _) = self.registry.get_skill(skill_id).await?;
        let mut visited = HashSet::new();
        let mut resolved = Vec::new();
        let mut order = Vec::new();

        self.resolve_recursive(&skill, &mut visited, &mut resolved, &mut order).await?;

        Ok(DependencyTree {
            root: skill.name.clone(),
            root_version: skill.version.clone(),
            dependencies: resolved,
            install_order: order,
        })
    }

    async fn resolve_recursive(
        &self,
        skill: &SkillMetadata,
        visited: &mut HashSet<String>,
        resolved: &mut Vec<ResolvedDependency>,
        order: &mut Vec<String>,
    ) -> Result<()> {
        if visited.contains(&skill.name) {
            // Circular dependency check
            return Err(DependencyError::Circular(skill.name.clone()));
        }
        visited.insert(skill.name.clone());

        for dep in &skill.dependencies {
            let already_installed = match dep.dep_type {
                DependencyType::Skill => self.installer.is_installed(&dep.name),
                DependencyType::Package => self.check_package_installed(&dep.name).await?,
            };

            // Check version constraints
            if let Some(constraint) = &dep.version_constraint {
                if already_installed {
                    let installed_version = self.get_installed_version(&dep.name, &dep.dep_type)?;
                    if !self.satisfies_constraint(&installed_version, constraint)? {
                        return Err(DependencyError::VersionConflict {
                            name: dep.name.clone(),
                            installed: installed_version,
                            required: constraint.clone(),
                        });
                    }
                }
            }

            resolved.push(ResolvedDependency {
                name: dep.name.clone(),
                dep_type: dep.dep_type.clone(),
                version: dep.version_constraint.clone().unwrap_or_default(),
                already_installed,
            });

            // Recursively resolve skill dependencies
            if !already_installed && matches!(dep.dep_type, DependencyType::Skill) {
                let (dep_skill, _) = self.registry.get_skill(&dep.name).await?;
                self.resolve_recursive(&dep_skill, visited, resolved, order).await?;
            }

            if !already_installed {
                order.push(dep.name.clone());
            }
        }

        // Add the skill itself last (topological order)
        order.push(skill.name.clone());
        visited.remove(&skill.name);

        Ok(())
    }

    fn satisfies_constraint(&self, version: &str, constraint: &str) -> Result<bool> {
        let ver = semver::Version::parse(version)?;
        let req = semver::VersionReq::parse(constraint)?;
        Ok(req.matches(&ver))
    }

    async fn check_package_installed(&self, package: &str) -> Result<bool> {
        let output = tokio::process::Command::new("rpm")
            .args(["-q", package])
            .output()
            .await?;
        Ok(output.status.success())
    }

    fn get_installed_version(&self, name: &str, dep_type: &DependencyType) -> Result<String> {
        match dep_type {
            DependencyType::Skill => {
                let manifest = self.installer.get_installed_manifest(name)?;
                Ok(manifest.version)
            }
            DependencyType::Package => {
                // Query rpm for installed version
                let output = std::process::Command::new("rpm")
                    .args(["-q", "--queryformat", "%{VERSION}", name])
                    .output()?;
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
        }
    }
}
```

---

## 5. Version Compatibility Checker

```rust
pub struct CompatibilityChecker {
    os_version: semver::Version,
}

impl CompatibilityChecker {
    pub fn new() -> Result<Self> {
        let version_str = std::fs::read_to_string("/etc/levsha/version")
            .unwrap_or_else(|_| "0.1.0".to_string());
        let os_version = semver::Version::parse(version_str.trim())?;
        Ok(Self { os_version })
    }

    /// Check if a skill is compatible with the current OS version
    pub fn is_compatible(&self, skill: &SkillMetadata) -> bool {
        match &skill.compatible_versions {
            None => true,  // No constraint = compatible with all
            Some(range) => {
                let min_ok = semver::Version::parse(&range.min)
                    .map(|min| self.os_version >= min)
                    .unwrap_or(true);
                let max_ok = semver::Version::parse(&range.max)
                    .map(|max| self.os_version <= max)
                    .unwrap_or(true);
                min_ok && max_ok
            }
        }
    }

    /// Get a human-readable compatibility message
    pub fn compatibility_message(&self, skill: &SkillMetadata) -> Option<String> {
        if self.is_compatible(skill) {
            None
        } else {
            let range = skill.compatible_versions.as_ref()?;
            Some(format!(
                "{} requires Levsha OS {} – {}, but you have {}",
                skill.name, range.min, range.max, self.os_version
            ))
        }
    }
}
```

---

## 6. Update Checker

```rust
pub struct UpdateChecker {
    registry: Arc<RegistryHttpClient>,
    installer: Arc<SkillInstaller>,
    check_interval: std::time::Duration,
}

#[derive(Debug)]
pub struct AvailableUpdate {
    pub skill_name: String,
    pub current_version: String,
    pub latest_version: String,
    pub changelog: Option<String>,
}

impl UpdateChecker {
    pub fn new(
        registry: Arc<RegistryHttpClient>,
        installer: Arc<SkillInstaller>,
        check_interval_secs: u64,
    ) -> Self {
        Self {
            registry,
            installer,
            check_interval: std::time::Duration::from_secs(check_interval_secs),
        }
    }

    /// Start the background update checker
    pub fn start(self: Arc<Self>, tx: mpsc::Sender<Vec<AvailableUpdate>>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(self.check_interval).await;

                match self.check_all_updates().await {
                    Ok(updates) if !updates.is_empty() => {
                        let _ = tx.send(updates).await;
                    }
                    Ok(_) => {}  // No updates
                    Err(e) => {
                        tracing::warn!("Update check failed: {}", e);
                    }
                }
            }
        });
    }

    /// Check all installed skills for available updates
    pub async fn check_all_updates(&self) -> Result<Vec<AvailableUpdate>> {
        let installed = self.installer.list_installed()?;
        let mut updates = Vec::new();

        for manifest in &installed {
            if let Ok(Some(update)) = self.check_skill_update(manifest).await {
                updates.push(update);
            }
        }

        Ok(updates)
    }

    async fn check_skill_update(&self, manifest: &SkillManifest) -> Result<Option<AvailableUpdate>> {
        let search = SearchParams {
            query: Some(manifest.name.clone()),
            ..Default::default()
        };

        let result = self.registry.search(&search).await?;
        let registry_skill = result.skills.iter().find(|s| s.name == manifest.name);

        if let Some(skill) = registry_skill {
            let current = semver::Version::parse(&manifest.version)?;
            let latest = semver::Version::parse(&skill.version)?;

            if latest > current {
                return Ok(Some(AvailableUpdate {
                    skill_name: manifest.name.clone(),
                    current_version: manifest.version.clone(),
                    latest_version: skill.version.clone(),
                    changelog: None,  // Fetched on demand from detail endpoint
                }));
            }
        }

        Ok(None)
    }
}
```

---

## 7. Skill Detail Panel Widget (Chat Shell)

```rust
pub struct SkillDetailPanel {
    container: gtk4::Box,
    header: gtk4::Box,
    content: gtk4::ScrolledWindow,
    install_button: gtk4::Button,
    rate_button: gtk4::Button,
}

impl SkillDetailPanel {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.add_css_class("skill-detail-panel");

        // Header bar
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        header.add_css_class("panel-header");
        header.set_height_request(36);

        let title = gtk4::Label::new(None);
        title.add_css_class("panel-header-title");
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);

        let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("panel-close-button");

        header.append(&title);
        header.append(&close_btn);

        // Scrollable content area
        let content = gtk4::ScrolledWindow::new();
        content.set_vexpand(true);

        // Action buttons
        let actions = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        actions.set_margin_top(16);
        actions.set_margin_bottom(16);
        actions.set_margin_start(16);

        let install_button = gtk4::Button::with_label("Install");
        install_button.add_css_class("skill-install-button");

        let rate_button = gtk4::Button::with_label("Rate");
        rate_button.add_css_class("skill-rate-button");

        actions.append(&install_button);
        actions.append(&rate_button);

        container.append(&header);
        container.append(&content);
        container.append(&actions);

        Self {
            container,
            header,
            content,
            install_button,
            rate_button,
        }
    }

    pub fn display_skill(&self, skill: &SkillMetadata, versions: &[SkillVersion]) {
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        content_box.set_margin_all(16);

        // Display name + trust badge + version
        let name_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        let badge = TrustBadge::new(skill.trust_level);
        let display_name = gtk4::Label::new(Some(&skill.display_name));
        display_name.add_css_class("skill-display-name");
        let version = gtk4::Label::new(Some(&format!("v{}", skill.version)));
        version.add_css_class("skill-version-pill");
        name_row.append(&badge.widget());
        name_row.append(&display_name);
        name_row.append(&version);
        content_box.append(&name_row);

        // Author + trust level text
        let author_label = gtk4::Label::new(Some(&format!(
            "by {} \u{00b7} {}",
            skill.author,
            skill.trust_level.display_name()
        )));
        author_label.add_css_class("skill-author");
        content_box.append(&author_label);

        // Rating + downloads
        let stats = StarRating::new(skill.avg_rating, skill.rating_count, skill.downloads);
        content_box.append(&stats.widget());

        // Separator
        let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        content_box.append(&sep);

        // Long description
        if let Some(desc) = &skill.long_description {
            let desc_label = gtk4::Label::new(Some(desc));
            desc_label.add_css_class("skill-long-description");
            desc_label.set_wrap(true);
            desc_label.set_xalign(0.0);
            content_box.append(&desc_label);
        }

        // Tags
        let tags_box = gtk4::FlowBox::new();
        for tag in &skill.tags {
            let tag_label = gtk4::Label::new(Some(tag));
            tag_label.add_css_class("skill-tag-pill");
            tags_box.insert(&tag_label, -1);
        }
        content_box.append(&tags_box);

        // Version history
        let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        content_box.append(&sep2);
        let version_header = gtk4::Label::new(Some("Version History"));
        version_header.add_css_class("skill-section-header");
        content_box.append(&version_header);

        for ver in versions.iter().take(5) {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            let v = gtk4::Label::new(Some(&format!("v{}", ver.version)));
            v.add_css_class("version-number");
            let date = gtk4::Label::new(Some(&format_timestamp(ver.published_at)));
            date.add_css_class("version-date");
            let changelog = gtk4::Label::new(ver.changelog.as_deref());
            changelog.add_css_class("version-changelog");
            row.append(&v);
            row.append(&date);
            row.append(&changelog);
            content_box.append(&row);
        }

        self.content.set_child(Some(&content_box));
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}
```

---

## 8. Updated Skill Manager Tools

### skill_search.json (Modified)

```json
{
  "name": "skill_search",
  "description": "Search the community skill registry for available skills. Supports text search, tag filtering, trust level filtering, and sort options.",
  "input_schema": {
    "type": "object",
    "properties": {
      "query": {
        "type": "string",
        "description": "Search query (matches name, description, tags)"
      },
      "tags": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Filter by tags"
      },
      "trust_level": {
        "type": "string",
        "enum": ["official", "verified", "community"],
        "description": "Filter by trust level"
      },
      "sort": {
        "type": "string",
        "enum": ["downloads", "rating", "newest"],
        "description": "Sort order for results"
      }
    },
    "required": ["query"]
  }
}
```

### skill_rate.json (New)

```json
{
  "name": "skill_rate",
  "description": "Rate an installed skill from 1 to 5 stars.",
  "input_schema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Name of the installed skill to rate"
      },
      "rating": {
        "type": "integer",
        "minimum": 1,
        "maximum": 5,
        "description": "Rating from 1 to 5 stars"
      }
    },
    "required": ["name", "rating"]
  }
}
```

---

## 9. Implementation Stages

**Stage 1 -- Registry HTTP Client (2 days)**
1. Implement `RegistryHttpClient` with search, detail, and download endpoints.
2. Add `reqwest` dependency with `rustls-tls` feature.
3. Implement `TrustLevel` enum and serialization.
4. Unit tests with mock HTTP server.

**Stage 2 -- Rich Metadata & Version Compatibility (1 day)**
1. Extend `SkillMetadata` struct with all new fields.
2. Implement `CompatibilityChecker` with semver comparison.
3. Update `SkillManifest` to include `compatible_versions` and `dependencies` fields.
4. Unit tests for version compatibility logic.

**Stage 3 -- Dependency Resolver (2 days)**
1. Implement `DependencyResolver` with topological sort.
2. Circular dependency detection.
3. Version constraint solver using `semver::VersionReq`.
4. Conflict detection for incompatible dependency versions.
5. Integration test: resolve multi-level dependency tree.

**Stage 4 -- Update Checker & Notifications (1 day)**
1. Implement `UpdateChecker` as a background `tokio::task`.
2. Wire update notifications to Module 21 notification system.
3. Implement manual update check and bulk update commands.
4. Integration test: detect and install available updates.

**Stage 5 -- Chat Shell UI (2 days)**
1. Implement `SkillDetailPanel` widget in split-view content panel.
2. Implement `CategoryBrowser` grid widget in split-view.
3. Implement `TrustBadge` and `StarRating` widgets.
4. Update search results formatting with trust badges and ratings.
5. Dependency tree display in chat before installation.

**Stage 6 -- Publishing & Rating (1 day)**
1. Implement skill packaging (tar.gz archive).
2. Implement publish flow with validation checklist.
3. Implement rating submission.
4. Create tool JSON schemas for new tools.
5. End-to-end test: search, install, rate, publish.

---

## 10. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `registry_client.rs` | HTTP request construction, response parsing, error handling |
| `trust.rs` | Trust level serialization, badge icon mapping, warning logic |
| `resolver.rs` | Topological sort, circular dependency detection, version constraint matching |
| `update_checker.rs` | Version comparison, update detection for installed skills |
| `manifest.rs` | Extended manifest parsing with dependencies and compatibility |

### Integration Tests

| Test | Method |
|------|--------|
| Search skills | Mock registry server, search with various filters, verify results |
| Get skill detail | Mock registry, fetch full metadata, verify all fields |
| Dependency resolution | Create skill graph with dependencies, resolve, verify install order |
| Circular dependency | Create A->B->A cycle, verify error detection |
| Version conflict | Create conflicting version requirements, verify error |
| Version compatibility | Test skill with min/max OS version, verify acceptance/rejection |
| Install with dependencies | Mock registry, install skill with 2 dependencies, verify all installed |
| Update detection | Install v1.0, mock v2.0 in registry, verify update detected |
| Rating submission | Mock registry, submit rating, verify request body |
| Publish flow | Package skill, mock registry, publish, verify upload |
| Trust level warning | Attempt install of community skill, verify warning triggered |
| Backward compatibility | Install skill via direct Git URL, verify Module 11 path still works |

### Cross-Module Tests

| Test | Description |
|------|-------------|
| Search + split-view | Search for skill, open detail in content panel, verify display |
| Install + notification | Install skill, trigger update check later, verify notification |
| Category browse + install | Browse category, select skill, install, verify loaded |
