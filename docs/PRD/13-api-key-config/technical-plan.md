# 13 — User-Configurable API Key: Technical Plan

**Module:** API Key Configuration
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

Changes are distributed across the engine and chat-shell crates.

```
engine/
  src/
    config.rs             # Modified: key validation, model switching
    api/
      mod.rs              # Modified: key reload, model change at runtime
      validator.rs        # New: API key validation via test request

chat-shell/
  src/
    ui/
      first_boot.rs       # New: first-boot key entry screen
      key_input.rs        # New: secure key input widget (masked)
      model_selector.rs   # New: model change handling
    state/
      session.rs          # Modified: first-boot state detection
```

---

## 2. API Key Validation

```rust
pub struct KeyValidator {
    http_client: reqwest::Client,
    base_url: String,
}

pub enum ValidationResult {
    Valid { model_info: String },
    InvalidKey,
    NetworkError(String),
    RateLimited,
}

impl KeyValidator {
    pub async fn validate(&self, key: &str) -> ValidationResult {
        let request = serde_json::json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let response = match self.http_client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => return ValidationResult::NetworkError(e.to_string()),
        };

        match response.status().as_u16() {
            200 => ValidationResult::Valid {
                model_info: "claude-haiku-4-5".into(),
            },
            401 => ValidationResult::InvalidKey,
            429 => ValidationResult::RateLimited,
            _ => ValidationResult::NetworkError(
                format!("Unexpected status: {}", response.status())
            ),
        }
    }
}
```

---

## 3. Config Management

```rust
#[derive(Deserialize, Serialize)]
pub struct ApiConfig {
    pub key: String,
    pub model: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub models: HashMap<String, String>,
}

impl ApiConfig {
    pub fn has_key(&self) -> bool {
        !self.key.is_empty()
    }

    pub fn set_key(&mut self, key: &str, config_path: &Path) -> Result<()> {
        self.key = key.to_string();
        self.save(config_path)?;

        // Set restrictive permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(config_path, perms)?;
        }
        Ok(())
    }

    pub fn set_model(&mut self, model_alias: &str, config_path: &Path) -> Result<String> {
        let model_id = self.models.get(model_alias)
            .ok_or_else(|| ConfigError::UnknownModel(model_alias.to_string()))?
            .clone();
        self.model = model_id.clone();
        self.save(config_path)?;
        Ok(model_id)
    }

    pub fn masked_key(&self) -> String {
        if self.key.len() > 12 {
            format!("{}•••••••", &self.key[..12])
        } else {
            "•••••••".to_string()
        }
    }

    fn save(&self, config_path: &Path) -> Result<()> {
        let full_config = std::fs::read_to_string(config_path)?;
        let mut doc: toml::Table = toml::from_str(&full_config)?;
        if let Some(api) = doc.get_mut("api") {
            if let Some(table) = api.as_table_mut() {
                table.insert("key".into(), toml::Value::String(self.key.clone()));
                table.insert("model".into(), toml::Value::String(self.model.clone()));
            }
        }
        std::fs::write(config_path, toml::to_string_pretty(&doc)?)?;
        Ok(())
    }
}
```

---

## 4. First-Boot Screen (Chat Shell)

```rust
pub struct FirstBootScreen {
    container: gtk4::Box,
    logo: gtk4::Picture,
    title: gtk4::Label,
    description: gtk4::Label,
    key_input: KeyInputWidget,
    status_label: gtk4::Label,
    error_label: gtk4::Label,
}

impl FirstBootScreen {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
        container.set_halign(gtk4::Align::Center);
        container.set_valign(gtk4::Align::Center);
        container.set_width_request(480);

        // Logo (flea logo.png, 100x100, circular)
        let logo = gtk4::Picture::for_filename("/usr/share/levsha/assets/flea-logo.png");
        logo.set_content_fit(gtk4::ContentFit::Cover);
        logo.set_size_request(100, 100);

        // Title
        let title = gtk4::Label::new(Some("Levsha OS"));
        title.add_css_class("first-boot-title");

        // Description
        let description = gtk4::Label::new(Some(
            "Welcome to your operating system.\n\n\
             To get started, enter your Anthropic API key below.\n\n\
             You can get one at:\nconsole.anthropic.com/settings/keys"
        ));
        description.set_justify(gtk4::Justification::Center);
        description.add_css_class("first-boot-description");

        // Key input (masked)
        let key_input = KeyInputWidget::new();

        let status_label = gtk4::Label::new(None);
        let error_label = gtk4::Label::new(None);
        error_label.set_visible(false);
        error_label.add_css_class("first-boot-error");

        container.append(&logo);
        container.append(&title);
        container.append(&description);
        container.append(&key_input.widget());
        container.append(&status_label);
        container.append(&error_label);

        Self { container, logo, title, description, key_input, status_label, error_label }
    }
}
```

---

## 5. Secure Key Input Widget

```rust
pub struct KeyInputWidget {
    entry: gtk4::PasswordEntry,
    container: gtk4::Box,
}

impl KeyInputWidget {
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.add_css_class("key-input-container");

        let entry = gtk4::PasswordEntry::new();
        entry.set_placeholder_text(Some("sk-ant-api03-..."));
        entry.set_show_peek_icon(false); // Don't allow unmasking
        entry.set_hexpand(true);
        entry.add_css_class("key-input-entry");

        container.append(&entry);
        Self { entry, container }
    }

    pub fn get_key(&self) -> String {
        self.entry.text().to_string()
    }

    pub fn set_sensitive(&self, sensitive: bool) {
        self.entry.set_sensitive(sensitive);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}
```

---

## 6. IPC Protocol Extensions

New message types for key configuration:

```rust
// Chat Shell -> Engine
#[derive(Serialize, Deserialize)]
pub enum KeyMessage {
    #[serde(rename = "set_api_key")]
    SetKey { key: String },
    #[serde(rename = "set_model")]
    SetModel { model: String },
}

// Engine -> Chat Shell
#[derive(Serialize, Deserialize)]
pub enum KeyResponse {
    #[serde(rename = "key_valid")]
    KeyValid,
    #[serde(rename = "key_invalid")]
    KeyInvalid { reason: String },
    #[serde(rename = "key_network_error")]
    KeyNetworkError { reason: String },
    #[serde(rename = "model_changed")]
    ModelChanged { model: String, display_name: String },
}
```

---

## 7. CSS for First-Boot Screen

```css
.first-boot-title {
    font-family: "IBM Plex Sans";
    font-size: 24px;
    font-weight: 600;
    color: #3A3228;
    margin-top: 16px;
}

.first-boot-description {
    font-family: "IBM Plex Sans";
    font-size: 15px;
    color: #6B5D4F;
    margin: 16px 0;
}

.key-input-container {
    background: #FDFBF7;
    border: 1px solid #EBE6DC;
    border-radius: 8px;
    padding: 8px 12px;
    margin: 8px 0;
}

.key-input-container:focus-within {
    border-color: #C67A52;
    border-width: 2px;
}

.key-input-entry {
    font-family: "IBM Plex Mono";
    font-size: 14px;
    color: #3A3228;
    border: none;
    background: transparent;
}

.first-boot-error {
    font-family: "IBM Plex Sans";
    font-size: 13px;
    color: #C67A52;
    background: #FFF5F0;
    border: 1px solid #C67A52;
    border-radius: 8px;
    padding: 8px 12px;
}
```

---

## 8. Implementation Stages

**Stage 1 — Config Changes (0.5 day)**
1. Update `config.toml` template with empty key.
2. Add `has_key()`, `set_key()`, `masked_key()` to config.
3. File permission setting.

**Stage 2 — Key Validation (0.5 day)**
1. Implement `KeyValidator` with test request.
2. Handle all response codes (200, 401, 429, network error).
3. Unit tests with mock HTTP server.

**Stage 3 — First-Boot Screen (1-2 days)**
1. `FirstBootScreen` widget with logo, description, key input.
2. `KeyInputWidget` with masking.
3. Validation flow (disable input, show progress, handle result).
4. Success transition animation to normal chat.

**Stage 4 — Model Selection (0.5 day)**
1. Model alias resolution from config.
2. Status bar model display.
3. IPC messages for model change.

**Stage 5 — Key Change Flow (0.5 day)**
1. Inline key input widget for post-setup key changes.
2. Validate new key before replacing old.
3. Engine reconnects with new key.

---

## 9. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `validator.rs` | Valid key, invalid key, network error, rate limited |
| `config.rs` | Key save/load, permissions, model resolution, masked display |

### Integration Tests

| Test | Method |
|------|--------|
| First-boot detection | Start with empty key, verify first-boot screen shown |
| Key validation success | Mock API returns 200, verify transition to normal chat |
| Key validation failure | Mock API returns 401, verify error message |
| Key persistence | Set key, restart engine, verify key loaded |
| Model switching | Change model via IPC, verify next API call uses new model |
| Key masking | Set key, verify only first 12 chars visible |
| File permissions | Set key, verify config file is 600 |
