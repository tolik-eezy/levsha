# 25 — Skill Creation Wizard: Technical Plan

**Module:** Skill Creation Wizard
**Language:** Rust
**Phase:** 3

---

## 1. Crate Structure

```
engine/
  src/
    wizard/
      mod.rs              # Wizard state machine, orchestration
      state.rs            # WizardState enum, transitions
      schema_gen.rs       # Tool schema generator
      prompt_gen.rs       # LLM prompt generator
      validator.rs        # Skill validator (manifest, schemas, prompt)
      test_runner.rs      # Test runner (mock tool calls, schema validation)
      preview.rs          # Preview mode (temporary skill loading)
      publish.rs          # Publish client (calls Module 24 API)

skills/
  built-in/
    skill-wizard/
      skill.yaml
      prompts/
        skill-wizard.md
      tools/
        wizard_start.json
        wizard_add_tool.json
        wizard_set_prompt.json
        wizard_preview.json
        wizard_test.json
        wizard_finalize.json
        wizard_publish.json
```

---

## 2. Wizard State Machine

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WizardState {
    /// Initial state — waiting for skill name and description
    Start,
    /// Collecting skill description
    Describing,
    /// Defining tools one by one
    DefiningTools,
    /// Generating and refining the LLM prompt
    WritingPrompt,
    /// Generating the skill.yaml manifest
    GeneratingManifest,
    /// Running validation tests
    Testing,
    /// Skill files written to disk, offering preview
    Finalizing,
    /// Skill loaded in preview mode for testing
    Previewing,
    /// Offering to publish to community repository
    Publishing,
    /// Wizard complete
    Complete,
}

impl WizardState {
    /// Returns the step number (1-indexed) for the progress indicator
    pub fn step_number(&self) -> u8 {
        match self {
            WizardState::Start | WizardState::Describing => 1,
            WizardState::DefiningTools => 3,
            WizardState::WritingPrompt => 4,
            WizardState::GeneratingManifest => 5,
            WizardState::Testing => 5,
            WizardState::Finalizing => 6,
            WizardState::Previewing => 6,
            WizardState::Publishing => 6,
            WizardState::Complete => 6,
        }
    }

    pub fn total_steps() -> u8 {
        6
    }

    pub fn display_name(&self) -> &str {
        match self {
            WizardState::Start => "Getting started",
            WizardState::Describing => "Describing your skill",
            WizardState::DefiningTools => "Defining tools",
            WizardState::WritingPrompt => "Writing the prompt",
            WizardState::GeneratingManifest => "Generating manifest",
            WizardState::Testing => "Running tests",
            WizardState::Finalizing => "Finalizing",
            WizardState::Previewing => "Preview mode",
            WizardState::Publishing => "Publishing",
            WizardState::Complete => "Complete",
        }
    }

    /// Valid transitions from this state
    pub fn valid_transitions(&self) -> Vec<WizardState> {
        match self {
            WizardState::Start => vec![WizardState::Describing],
            WizardState::Describing => vec![WizardState::DefiningTools, WizardState::Start],
            WizardState::DefiningTools => vec![WizardState::WritingPrompt, WizardState::Describing],
            WizardState::WritingPrompt => vec![WizardState::GeneratingManifest, WizardState::DefiningTools],
            WizardState::GeneratingManifest => vec![WizardState::Testing, WizardState::WritingPrompt],
            WizardState::Testing => vec![WizardState::Finalizing, WizardState::DefiningTools],
            WizardState::Finalizing => vec![WizardState::Previewing, WizardState::Publishing, WizardState::Complete],
            WizardState::Previewing => vec![WizardState::Finalizing],
            WizardState::Publishing => vec![WizardState::Complete],
            WizardState::Complete => vec![],
        }
    }
}

pub struct WizardSession {
    pub state: WizardState,
    pub skill_name: Option<String>,
    pub description: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub prompt: Option<String>,
    pub manifest: Option<SkillManifest>,
    pub dependencies: Vec<String>,
    pub output_dir: PathBuf,
    pub preview_active: bool,
    pub test_results: Option<Vec<TestResult>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub schema_json: serde_json::Value,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,     // "string", "integer", "boolean", "array", "object"
    pub description: String,
    pub required: bool,
    pub items_type: Option<String>,  // For array types
}

impl WizardSession {
    pub fn new(output_dir: PathBuf) -> Self {
        Self {
            state: WizardState::Start,
            skill_name: None,
            description: None,
            tools: Vec::new(),
            prompt: None,
            manifest: None,
            dependencies: Vec::new(),
            output_dir,
            preview_active: false,
            test_results: None,
        }
    }

    pub fn transition_to(&mut self, new_state: WizardState) -> Result<()> {
        if self.state.valid_transitions().contains(&new_state) {
            self.state = new_state;
            Ok(())
        } else {
            Err(WizardError::InvalidTransition {
                from: self.state.clone(),
                to: new_state,
            })
        }
    }
}
```

---

## 3. Tool Schema Generator

```rust
pub struct SchemaGenerator;

impl SchemaGenerator {
    /// Build an Anthropic-compatible tool JSON schema from a ToolDefinition
    pub fn generate(tool: &ToolDefinition) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &tool.parameters {
            let mut prop = serde_json::Map::new();
            prop.insert("type".into(), serde_json::Value::String(param.param_type.clone()));
            prop.insert("description".into(), serde_json::Value::String(param.description.clone()));

            // Handle array items type
            if param.param_type == "array" {
                if let Some(items_type) = &param.items_type {
                    prop.insert("items".into(), serde_json::json!({
                        "type": items_type
                    }));
                }
            }

            properties.insert(param.name.clone(), serde_json::Value::Object(prop));

            if param.required {
                required.push(serde_json::Value::String(param.name.clone()));
            }
        }

        serde_json::json!({
            "name": tool.name,
            "description": tool.description,
            "input_schema": {
                "type": "object",
                "properties": properties,
                "required": required,
            }
        })
    }

    /// Build a ToolDefinition from user-provided natural language description
    /// by asking the LLM to extract structured information
    pub async fn from_description(
        description: &str,
        backend: &dyn LlmBackend,
    ) -> Result<ToolDefinition> {
        let request = MessageRequest {
            model: "claude-sonnet-4-20250514".into(),
            max_tokens: 500,
            system: concat!(
                "Extract a tool definition from the user's description. ",
                "Return JSON with: name (snake_case), description, and parameters ",
                "(array of {name, type, description, required}). ",
                "Valid types: string, integer, number, boolean, array. ",
                "For array types, include items_type. ",
                "Reply with ONLY valid JSON."
            ).into(),
            messages: vec![Message {
                role: "user".into(),
                content: vec![ContentBlock::Text {
                    text: description.to_string(),
                }],
            }],
            tools: vec![],
            stream: false,
        };

        let response_text = backend.send_message(&request).await?
            .collect_text().await?;

        let parsed: serde_json::Value = serde_json::from_str(response_text.trim())?;

        let name = parsed["name"].as_str()
            .ok_or(WizardError::SchemaGenFailed("missing name".into()))?
            .to_string();

        let description = parsed["description"].as_str()
            .unwrap_or("")
            .to_string();

        let parameters: Vec<ToolParameter> = parsed["parameters"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|p| ToolParameter {
                name: p["name"].as_str().unwrap_or("").to_string(),
                param_type: p["type"].as_str().unwrap_or("string").to_string(),
                description: p["description"].as_str().unwrap_or("").to_string(),
                required: p["required"].as_bool().unwrap_or(false),
                items_type: p["items_type"].as_str().map(|s| s.to_string()),
            })
            .collect();

        let tool_def = ToolDefinition {
            name: name.clone(),
            description: description.clone(),
            parameters: parameters.clone(),
            schema_json: serde_json::Value::Null,  // Will be set below
        };

        // Generate the full schema
        let schema = Self::generate(&tool_def);

        Ok(ToolDefinition {
            schema_json: schema,
            ..tool_def
        })
    }
}
```

---

## 4. Prompt Generator

```rust
pub struct PromptGenerator;

impl PromptGenerator {
    /// Generate an LLM prompt from skill description and tool definitions
    pub fn generate(skill_name: &str, description: &str, tools: &[ToolDefinition]) -> String {
        let mut prompt = String::new();

        // Header
        prompt.push_str(&format!("# {}\n\n", to_title_case(skill_name)));
        prompt.push_str(&format!("{}\n\n", description));

        // Available tools section
        prompt.push_str("## Available Tools\n\n");
        for tool in tools {
            prompt.push_str(&format!(
                "- **{}** -- {}\n  Use when the user {}.\n\n",
                tool.name,
                tool.description,
                Self::infer_usage_context(&tool.description),
            ));
        }

        // Notes section
        prompt.push_str("## Notes\n\n");
        prompt.push_str("- Always confirm file paths with the user before modifying files.\n");
        prompt.push_str("- Report errors clearly with actionable suggestions.\n");

        prompt
    }

    /// Refine an existing prompt using LLM assistance
    pub async fn refine(
        current_prompt: &str,
        user_feedback: &str,
        backend: &dyn LlmBackend,
    ) -> Result<String> {
        let request = MessageRequest {
            model: "claude-sonnet-4-20250514".into(),
            max_tokens: 1000,
            system: concat!(
                "You are refining an LLM skill prompt. ",
                "Apply the user's feedback to improve the prompt. ",
                "Return ONLY the updated prompt text, no commentary."
            ).into(),
            messages: vec![Message {
                role: "user".into(),
                content: vec![ContentBlock::Text {
                    text: format!(
                        "Current prompt:\n\n{}\n\nFeedback: {}",
                        current_prompt, user_feedback
                    ),
                }],
            }],
            tools: vec![],
            stream: false,
        };

        let refined = backend.send_message(&request).await?
            .collect_text().await?;

        Ok(refined.trim().to_string())
    }

    fn infer_usage_context(description: &str) -> String {
        let desc_lower = description.to_lowercase();
        if desc_lower.contains("merge") || desc_lower.contains("combine") {
            "wants to combine or merge items".to_string()
        } else if desc_lower.contains("split") || desc_lower.contains("separate") {
            "wants to split or separate items".to_string()
        } else if desc_lower.contains("convert") || desc_lower.contains("transform") {
            "wants to convert or transform data".to_string()
        } else {
            format!("asks to {}", description.to_lowercase())
        }
    }
}

fn to_title_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + &c.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
```

---

## 5. Skill Validator

```rust
pub struct SkillValidator;

#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
}

impl SkillValidator {
    /// Run all validation tests on the wizard session
    pub fn validate(session: &WizardSession) -> Vec<TestResult> {
        let mut results = Vec::new();

        // 1. Manifest schema validation
        results.push(Self::validate_manifest(session));

        // 2. Tool schema validation (one per tool)
        for tool in &session.tools {
            results.push(Self::validate_tool_schema(tool));
        }

        // 3. Prompt file validation
        results.push(Self::validate_prompt(session));

        // 4. Simulated tool calls (one per tool)
        for tool in &session.tools {
            results.push(Self::simulate_tool_call(tool));
        }

        results
    }

    fn validate_manifest(session: &WizardSession) -> TestResult {
        let name = "Manifest schema valid".to_string();

        if session.skill_name.is_none() {
            return TestResult { name, passed: false, error: Some("Missing skill name".into()) };
        }
        if session.description.is_none() {
            return TestResult { name, passed: false, error: Some("Missing description".into()) };
        }
        if session.tools.is_empty() {
            return TestResult { name, passed: false, error: Some("No tools defined".into()) };
        }

        TestResult { name, passed: true, error: None }
    }

    fn validate_tool_schema(tool: &ToolDefinition) -> TestResult {
        let name = format!("Tool schema: {} -- valid", tool.name);

        // Check required fields
        if tool.name.is_empty() {
            return TestResult { name, passed: false, error: Some("Missing tool name".into()) };
        }
        if tool.description.is_empty() {
            return TestResult { name, passed: false, error: Some("Missing description".into()) };
        }

        // Validate JSON schema structure
        let schema = &tool.schema_json;
        if schema.get("name").is_none() {
            return TestResult { name, passed: false, error: Some("Missing 'name' in schema".into()) };
        }
        if schema.get("input_schema").is_none() {
            return TestResult { name, passed: false, error: Some("Missing 'input_schema' in schema".into()) };
        }

        // Validate parameter types
        for param in &tool.parameters {
            let valid_types = ["string", "integer", "number", "boolean", "array", "object"];
            if !valid_types.contains(&param.param_type.as_str()) {
                return TestResult {
                    name,
                    passed: false,
                    error: Some(format!("Invalid parameter type '{}' for '{}'", param.param_type, param.name)),
                };
            }
        }

        TestResult { name, passed: true, error: None }
    }

    fn validate_prompt(session: &WizardSession) -> TestResult {
        let name = "Prompt file present and non-empty".to_string();

        match &session.prompt {
            None => TestResult { name, passed: false, error: Some("No prompt generated".into()) },
            Some(prompt) if prompt.trim().is_empty() => {
                TestResult { name, passed: false, error: Some("Prompt is empty".into()) }
            }
            Some(_) => TestResult { name, passed: true, error: None },
        }
    }

    fn simulate_tool_call(tool: &ToolDefinition) -> TestResult {
        let name = format!("Simulated call: {} -- accepted", tool.name);

        // Generate sample inputs based on parameter types
        let mut sample_input = serde_json::Map::new();
        for param in &tool.parameters {
            if param.required {
                let sample_value = match param.param_type.as_str() {
                    "string" => serde_json::Value::String("sample_value".to_string()),
                    "integer" => serde_json::json!(42),
                    "number" => serde_json::json!(3.14),
                    "boolean" => serde_json::json!(true),
                    "array" => serde_json::json!(["item1", "item2"]),
                    "object" => serde_json::json!({}),
                    _ => serde_json::Value::Null,
                };
                sample_input.insert(param.name.clone(), sample_value);
            }
        }

        // Validate sample input against the schema
        let input_schema = &tool.schema_json["input_schema"];
        let required = input_schema["required"].as_array();

        if let Some(required_fields) = required {
            for field in required_fields {
                if let Some(field_name) = field.as_str() {
                    if !sample_input.contains_key(field_name) {
                        return TestResult {
                            name,
                            passed: false,
                            error: Some(format!("Required field '{}' not satisfied by sample", field_name)),
                        };
                    }
                }
            }
        }

        TestResult { name, passed: true, error: None }
    }
}
```

---

## 6. Test Runner

```rust
pub struct TestRunner {
    validator: SkillValidator,
}

impl TestRunner {
    /// Run a dry-run conversation to test the skill's prompt and tools
    pub async fn dry_run(
        session: &WizardSession,
        backend: &dyn LlmBackend,
    ) -> Result<DryRunResult> {
        let prompt = session.prompt.as_deref()
            .ok_or(WizardError::NoPromptGenerated)?;

        let tools: Vec<serde_json::Value> = session.tools.iter()
            .map(|t| t.schema_json.clone())
            .collect();

        // Send a test message using the skill's prompt and tools
        let request = MessageRequest {
            model: "claude-sonnet-4-20250514".into(),
            max_tokens: 200,
            system: prompt.to_string(),
            messages: vec![Message {
                role: "user".into(),
                content: vec![ContentBlock::Text {
                    text: format!(
                        "Test: invoke the {} tool with sample arguments.",
                        session.tools.first().map(|t| t.name.as_str()).unwrap_or("first")
                    ),
                }],
            }],
            tools,
            stream: false,
        };

        let response = backend.send_message(&request).await?;
        let text = response.collect_text().await?;
        let tool_calls = response.collect_tool_use().await?;

        Ok(DryRunResult {
            response_text: text,
            tool_calls_made: tool_calls.len(),
            tool_names: tool_calls.iter().map(|tc| tc.name.clone()).collect(),
        })
    }
}

#[derive(Debug)]
pub struct DryRunResult {
    pub response_text: String,
    pub tool_calls_made: usize,
    pub tool_names: Vec<String>,
}
```

---

## 7. Preview Mode

```rust
pub struct PreviewManager {
    skill_loader: Arc<Mutex<SkillLoader>>,
    active_preview: Option<String>,  // Skill name currently in preview
}

impl PreviewManager {
    /// Load a wizard-generated skill temporarily for testing
    pub fn start_preview(&mut self, session: &WizardSession) -> Result<()> {
        let skill_name = session.skill_name.as_deref()
            .ok_or(WizardError::NoSkillName)?;

        // Build a temporary LoadedSkill from the wizard session
        let skill = LoadedSkill {
            name: skill_name.to_string(),
            version: "0.1.0-preview".to_string(),
            description: session.description.clone().unwrap_or_default(),
            prompt: session.prompt.clone().unwrap_or_default(),
            tools: session.tools.iter().map(|t| t.schema_json.clone()).collect(),
            builtin: false,
            preview: true,
        };

        // Inject into the skill loader
        let mut loader = self.skill_loader.lock().unwrap();
        loader.inject_skill(skill)?;
        loader.recompose_system_prompt();
        loader.rebuild_tool_registry();

        self.active_preview = Some(skill_name.to_string());
        Ok(())
    }

    /// Unload the preview skill and restore the previous state
    pub fn end_preview(&mut self) -> Result<()> {
        if let Some(skill_name) = self.active_preview.take() {
            let mut loader = self.skill_loader.lock().unwrap();
            loader.remove_skill(&skill_name)?;
            loader.recompose_system_prompt();
            loader.rebuild_tool_registry();
        }
        Ok(())
    }

    pub fn is_previewing(&self) -> bool {
        self.active_preview.is_some()
    }

    pub fn preview_skill_name(&self) -> Option<&str> {
        self.active_preview.as_deref()
    }
}
```

---

## 8. Publish Client

```rust
pub struct WizardPublishClient {
    registry: Arc<RegistryHttpClient>,  // From Module 24
}

impl WizardPublishClient {
    /// Package and publish a wizard-generated skill
    pub async fn publish(&self, session: &WizardSession) -> Result<PublishResult> {
        let skill_name = session.skill_name.as_deref()
            .ok_or(WizardError::NoSkillName)?;

        // 1. Write files to temp directory for packaging
        let temp_dir = tempfile::tempdir()?;
        self.write_skill_files(session, temp_dir.path())?;

        // 2. Create tar.gz archive
        let archive = self.package_skill(temp_dir.path())?;

        // 3. Build metadata
        let metadata = SkillMetadata {
            id: String::new(),  // Assigned by server
            name: skill_name.to_string(),
            display_name: to_title_case(skill_name),
            author: self.get_author()?,
            version: "0.1.0".to_string(),
            description: session.description.clone().unwrap_or_default(),
            long_description: None,
            tags: Vec::new(),
            trust_level: TrustLevel::Community,
            downloads: 0,
            avg_rating: 0.0,
            rating_count: 0,
            dependencies: Vec::new(),
            compatible_versions: None,
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            manifest_url: String::new(),
        };

        // 4. Upload to registry
        let skill_id = self.registry.publish_skill(archive, &metadata).await?;

        Ok(PublishResult {
            skill_id,
            trust_level: TrustLevel::Community,
        })
    }

    fn write_skill_files(&self, session: &WizardSession, dir: &Path) -> Result<()> {
        let skill_name = session.skill_name.as_deref().unwrap();

        // Create directory structure
        std::fs::create_dir_all(dir.join("prompts"))?;
        std::fs::create_dir_all(dir.join("tools"))?;

        // Write prompt file
        let prompt_filename = format!("{}.md", skill_name);
        std::fs::write(
            dir.join("prompts").join(&prompt_filename),
            session.prompt.as_deref().unwrap_or(""),
        )?;

        // Write tool schemas
        let mut tool_paths = Vec::new();
        for tool in &session.tools {
            let filename = format!("{}.json", tool.name);
            let path = format!("tools/{}", filename);
            std::fs::write(
                dir.join("tools").join(&filename),
                serde_json::to_string_pretty(&tool.schema_json)?,
            )?;
            tool_paths.push(path);
        }

        // Write skill.yaml manifest
        let manifest = SkillManifest {
            name: skill_name.to_string(),
            version: "0.1.0".to_string(),
            description: session.description.clone().unwrap_or_default(),
            author: self.get_author()?,
            builtin: Some(false),
            prompt: format!("prompts/{}", prompt_filename),
            tools: tool_paths,
            requires: if session.dependencies.is_empty() {
                None
            } else {
                Some(SkillRequires {
                    packages: Some(session.dependencies.clone()),
                })
            },
        };

        let yaml = serde_yaml::to_string(&manifest)?;
        std::fs::write(dir.join("skill.yaml"), yaml)?;

        Ok(())
    }

    fn package_skill(&self, dir: &Path) -> Result<Vec<u8>> {
        let mut archive = Vec::new();
        {
            let enc = flate2::write::GzEncoder::new(&mut archive, flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all(".", dir)?;
            tar.finish()?;
        }
        Ok(archive)
    }

    fn get_author(&self) -> Result<String> {
        // Read from system config
        let config_path = "/etc/levsha/config.toml";
        let config_str = std::fs::read_to_string(config_path)?;
        let config: toml::Value = toml::from_str(&config_str)?;
        Ok(config["skills"]["wizard"]["default_author"]
            .as_str()
            .unwrap_or("user")
            .to_string())
    }
}

pub struct PublishResult {
    pub skill_id: String,
    pub trust_level: TrustLevel,
}
```

---

## 9. Wizard Tool Handlers

```rust
impl WizardToolHandler {
    pub async fn handle_tool_call(
        &mut self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> Result<ToolResult> {
        match tool_name {
            "wizard_start" => {
                let name = input["name"].as_str().unwrap_or("").to_string();
                let description = input["description"].as_str().map(|s| s.to_string());

                self.session = WizardSession::new(self.output_dir.clone());
                self.session.skill_name = Some(name);
                self.session.description = description;
                self.session.transition_to(WizardState::Describing)?;

                Ok(ToolResult::text("Wizard session started. Ready to define tools."))
            }

            "wizard_add_tool" => {
                let tool_desc = input["description"].as_str()
                    .ok_or(WizardError::MissingField("description"))?;

                let tool_def = SchemaGenerator::from_description(tool_desc, &*self.backend).await?;
                let schema_preview = serde_json::to_string_pretty(&tool_def.schema_json)?;

                self.session.tools.push(tool_def);
                self.session.state = WizardState::DefiningTools;

                Ok(ToolResult::text(format!(
                    "Tool added. Schema:\n```json\n{}\n```",
                    schema_preview
                )))
            }

            "wizard_set_prompt" => {
                let description = input["description"].as_str().unwrap_or("");
                let feedback = input["feedback"].as_str();

                let prompt = if let (Some(existing), Some(fb)) = (&self.session.prompt, feedback) {
                    PromptGenerator::refine(existing, fb, &*self.backend).await?
                } else {
                    PromptGenerator::generate(
                        self.session.skill_name.as_deref().unwrap_or("skill"),
                        description,
                        &self.session.tools,
                    )
                };

                self.session.prompt = Some(prompt.clone());
                self.session.state = WizardState::WritingPrompt;

                Ok(ToolResult::text(format!("Prompt generated:\n\n{}", prompt)))
            }

            "wizard_test" => {
                let results = SkillValidator::validate(&self.session);
                self.session.test_results = Some(results.clone());
                self.session.state = WizardState::Testing;

                let summary: String = results.iter().map(|r| {
                    if r.passed {
                        format!("  ✓ {}", r.name)
                    } else {
                        format!("  ✗ {}\n      {}", r.name, r.error.as_deref().unwrap_or("Unknown error"))
                    }
                }).collect::<Vec<_>>().join("\n");

                let all_passed = results.iter().all(|r| r.passed);
                let count = results.len();
                let passed = results.iter().filter(|r| r.passed).count();

                Ok(ToolResult::text(format!(
                    "Test results:\n\n{}\n\n{} of {} tests passed.",
                    summary, passed, count
                )))
            }

            "wizard_preview" => {
                self.preview_mgr.start_preview(&self.session)?;
                self.session.state = WizardState::Previewing;
                self.session.preview_active = true;

                Ok(ToolResult::text(format!(
                    "Preview mode active for '{}'. The skill is now loaded. Say 'end preview' to stop.",
                    self.session.skill_name.as_deref().unwrap_or("skill")
                )))
            }

            "wizard_finalize" => {
                // End preview if active
                if self.session.preview_active {
                    self.preview_mgr.end_preview()?;
                    self.session.preview_active = false;
                }

                let output = input["output_dir"].as_str()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| self.session.output_dir.clone());

                let skill_name = self.session.skill_name.as_deref()
                    .ok_or(WizardError::NoSkillName)?;

                let skill_dir = output.join(skill_name);
                self.publish_client.write_skill_files(&self.session, &skill_dir)?;

                self.session.state = WizardState::Finalizing;

                Ok(ToolResult::text(format!(
                    "Skill files saved to {}",
                    skill_dir.display()
                )))
            }

            "wizard_publish" => {
                let result = self.publish_client.publish(&self.session).await?;
                self.session.state = WizardState::Complete;

                Ok(ToolResult::text(format!(
                    "Published as '{}' with {} trust level.",
                    result.skill_id,
                    result.trust_level.display_name()
                )))
            }

            _ => Err(WizardError::UnknownTool(tool_name.to_string())),
        }
    }
}
```

---

## 10. Implementation Stages

**Stage 1 -- Wizard State Machine & Session (1 day)**
1. Implement `WizardState` enum with transitions.
2. Implement `WizardSession` struct with all fields.
3. Create `skill-wizard` built-in skill directory with `skill.yaml`.
4. Unit tests for state transitions and validation.

**Stage 2 -- Schema Generator & Prompt Generator (2 days)**
1. Implement `SchemaGenerator::generate` for deterministic schema building.
2. Implement `SchemaGenerator::from_description` with LLM-assisted extraction.
3. Implement `PromptGenerator::generate` and `::refine`.
4. Unit tests for schema generation from various parameter types.
5. Integration test: generate tool schema from natural language description.

**Stage 3 -- Validator & Test Runner (1 day)**
1. Implement `SkillValidator::validate` with all test cases.
2. Implement simulated tool calls with sample inputs.
3. Implement `TestRunner::dry_run` for conversation-level testing.
4. Unit tests for pass/fail scenarios.

**Stage 4 -- Preview Mode & Tool Handlers (1.5 days)**
1. Implement `PreviewManager` with temporary skill injection.
2. Implement all 7 wizard tool handlers.
3. Wire tool handlers into the engine's tool dispatch.
4. Integration test: start wizard, add tools, preview, end preview.

**Stage 5 -- Publish Integration & Polish (1.5 days)**
1. Implement `WizardPublishClient` (packaging, upload).
2. Implement file writing (skill.yaml, tool JSONs, prompt).
3. Wire publish to Module 24 registry client.
4. Create tool JSON schemas for all 7 wizard tools.
5. End-to-end test: full wizard flow from start to publish.

---

## 11. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `state.rs` | State transitions, valid/invalid transitions, step numbers |
| `schema_gen.rs` | Schema generation for all parameter types, array items, required fields |
| `prompt_gen.rs` | Prompt template generation, title casing, usage context inference |
| `validator.rs` | Manifest validation, tool schema validation, prompt validation, simulated calls |

### Integration Tests

| Test | Method |
|------|--------|
| Full wizard flow | Start, describe, add 2 tools, generate prompt, test, finalize |
| Schema from description | Send natural language tool description, verify valid JSON Schema |
| Prompt refinement | Generate prompt, refine with feedback, verify changes applied |
| Validation failures | Create session with missing fields, verify correct errors |
| Preview mode | Start preview, verify skill loaded, end preview, verify unloaded |
| File generation | Finalize wizard, verify directory structure and file contents |
| Publish flow | Package and upload to mock registry, verify archive contents |
| State navigation | Go back to tools step, add tool, go forward, verify state |
| Dry run | Run test conversation with generated prompt/tools, verify tool invocation |

### Cross-Module Tests

| Test | Description |
|------|-------------|
| Wizard + split-view | Generate schema, verify preview displayed in content panel |
| Wizard + publish | Create and publish skill, verify it appears in Module 24 search results |
| Wizard + hot-reload | Finalize skill, install locally, verify available without restart |
