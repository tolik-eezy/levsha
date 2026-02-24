# 13 — User-Configurable API Key: Design Specification

**Module:** API Key Configuration
**Phase:** 2

---

## 1. First-Boot Key Entry Screen

When no API key is configured, the Chat Shell renders a special first-boot experience within the chat interface itself — not a separate wizard.

### First-Boot Layout

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│                                                          │
│                    ┌──────────┐                          │
│                    │  (flea   │                          │
│                    │  logo)   │                          │
│                    └──────────┘                          │
│                                                          │
│                   ◆ Levsha OS                            │
│                                                          │
│          Welcome to your operating system.               │
│                                                          │
│          To get started, enter your                      │
│          Anthropic API key below.                        │
│                                                          │
│          You can get one at:                             │
│          console.anthropic.com/settings/keys             │
│                                                          │
│                                                          │
│                                                          │
│                                                          │
├──────────────────────────────────────────────────────────┤
│  API key: sk-ant-•••••••••••••••••••••                   │
├──────────────────────────────────────────────────────────┤
│  ▸ setup   ▸ no key configured   ▸ 14:32                │
└──────────────────────────────────────────────────────────┘
```

### Design Elements

| Element | Style |
|---------|-------|
| Logo | `flea logo.png`, 100×100, cornerRadius 50 (circular), centered |
| Title "Levsha OS" | IBM Plex Sans 24px, weight 600, `$text-primary` (#3A3228) |
| Welcome text | IBM Plex Sans 15px, weight 400, `$text-secondary` (#6B5D4F), centered, max-width 480px |
| URL text | IBM Plex Mono 13px, `$accent-copper` (#C67A52) |
| Background | `$bg-primary` (#FBF8F3), full screen |
| Input field placeholder | "API key: sk-ant-..." in `$text-tertiary` (#8C7E6E) |
| Status bar | "setup" badge instead of model name, "no key configured" status |

---

## 2. Key Input Behavior

### Input States

| State | Display | Status Bar |
|-------|---------|------------|
| Empty | Placeholder text "sk-ant-..." | "no key configured" |
| Typing | Characters visible briefly, then masked: `sk-ant-api03-•••••` | "no key configured" |
| Validating | Input disabled, pulsing dots | "validating..." |
| Valid | Green checkmark appears | Transitions to normal |
| Invalid | Red error below input | "invalid key" |
| Network error | Orange warning below input | "cannot reach API" |

### Masking Behavior

- First 12 characters of the key are shown: `sk-ant-api03-`
- Remaining characters are masked with dots: `•••••••••`
- This allows the user to verify the key prefix without exposing the full key.

---

## 3. Validation Flow

```
User enters key and presses Enter
       │
       ▼
Input field disabled, status: "validating..."
       │
       ▼
Engine sends minimal test request:
  POST /v1/messages
  { model: "claude-haiku", messages: [{ role: "user", content: "hi" }], max_tokens: 1 }
       │
       ├── 200 OK → Key is valid
       │     │
       │     ▼
       │   Key saved to config.toml (600 permissions)
       │     │
       │     ▼
       │   Transition to normal chat with welcome message
       │
       ├── 401 Unauthorized → Invalid key
       │     │
       │     ▼
       │   Error message: "That key doesn't seem to be valid.
       │   Please check it and try again."
       │
       └── Network error → Can't reach API
             │
             ▼
           Error message: "I can't reach the Anthropic API.
           Check your internet connection and try again."
```

---

## 4. Validation Success Transition

After successful key validation, the first-boot screen smoothly transitions to the normal chat experience.

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│                    ┌──────────┐                          │
│                    │  (flea   │                          │
│                    │  logo)   │                          │
│                    └──────────┘                          │
│                                                          │
│                   ◆ Levsha OS                            │
│                                                          │
│          ✓ API key configured successfully.              │
│                                                          │
│          ─────────────────────────────────               │
│                                                          │
│          ┌─ Levsha ──────────────────────┐              │
│          │  Welcome. I'm your operating   │              │
│          │  system. Everything you need —  │              │
│          │  just ask.                      │              │
│          └────────────────────────────────┘              │
│                                                          │
├──────────────────────────────────────────────────────────┤
│  > _                                                     │
├──────────────────────────────────────────────────────────┤
│  ▸ claude-sonnet   ▸ connected   ▸ 14:32                │
└──────────────────────────────────────────────────────────┘
```

### Transition Animation

| Step | Duration | Effect |
|------|----------|--------|
| Success checkmark appears | 300ms | Fade in, `$accent-green` |
| Setup UI fades | 400ms | Opacity 1→0 |
| Welcome message slides in | 300ms | Slide up + fade in |
| Status bar updates | Instant | Model name + "connected" |

---

## 5. Key Change Flow (Post-Setup)

After initial setup, users can change the API key via chat.

```
User: change my API key

Levsha: Enter your new Anthropic API key below.
        Your current key will be replaced after validation.

        ┌────────────────────────────────────────────┐
        │  New API key: ________________________________│
        └────────────────────────────────────────────┘
```

The key change uses a special inline input widget (not the main chat input) to avoid the key appearing in conversation history.

### Key Change Input Styling

| Element | Style |
|---------|-------|
| Input container | `$bg-surface` (#FDFBF7), 1px `$border-primary`, cornerRadius 8 |
| Input label | IBM Plex Sans 13px, weight 500, `$text-secondary` |
| Input field | IBM Plex Mono 14px, `$text-primary`, no border |
| Focus state | 2px `$accent-copper` border |

---

## 6. Model Selection

Users can switch models via natural language. The status bar reflects the change.

### Model Display in Status Bar

```
▸ claude-sonnet        ← normal operation
▸ claude-opus          ← after "use opus"
▸ claude-haiku         ← after "use haiku"
```

### Model Selection Styling

| Element | Style |
|---------|-------|
| Model name in status bar | IBM Plex Mono 12px, weight 500, `$text-secondary` |
| Active model indicator | `$accent-copper` dot before name |

---

## 7. Error States

### Invalid Key Error

```
  ┌─ Error ──────────────────────────────────┐
  │  ⚠ That key doesn't seem to be valid.    │
  │                                            │
  │  Please check your API key and try again. │
  │  It should start with "sk-ant-".           │
  └────────────────────────────────────────────┘
```

### Styling

| Element | Style |
|---------|-------|
| Error container | `$danger-bg` (#FFF5F0), 1px `$accent-copper` border, cornerRadius 8 |
| Warning icon | Lucide `alert-circle`, `$accent-copper` |
| Error title | IBM Plex Sans 14px, weight 600, `$accent-copper` |
| Error body | IBM Plex Sans 13px, `$text-secondary` |

---

## 8. Assets Used

| Asset | Usage |
|-------|-------|
| `flea logo.png` | First-boot splash logo (centered, circular) |
| `logo2.png` | Levsha avatar in subsequent messages |
| Lucide `key` | Key configuration icon |
| Lucide `check-circle` | Validation success |
| Lucide `alert-circle` | Validation error |
| Lucide `loader` | Validation in progress (animated) |

---

## 9. Security Design

| Concern | Mitigation |
|---------|------------|
| Key in conversation history | Key input uses a dedicated widget; value is never added to the message history |
| Key in logs | Engine logs mask the key: `sk-ant-•••••` |
| Key in config file | File permissions set to 600, owned by `levsha` user |
| Key displayed on screen | Only first 12 characters shown, rest masked |
| Key change confirmation | New key validated before replacing old one |
