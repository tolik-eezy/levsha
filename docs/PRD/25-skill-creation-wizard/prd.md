# 25 — Skill Creation Wizard: Product Requirements

**Module:** Skill Creation Wizard
**Phase:** 3
**Status:** Draft

---

## 1. Overview

The Skill Creation Wizard is a built-in meta-skill that guides users through creating new skills entirely via chat conversation. Instead of manually writing `skill.yaml` manifests, tool JSON schemas, and LLM prompts, users describe what they want their skill to do, and the wizard generates all artifacts through an interactive, step-by-step dialogue.

This lowers the barrier to skill creation from "developer who understands Anthropic tool schemas" to "anyone who can describe what they want." The wizard handles template generation, tool schema building, prompt engineering, testing, and optional publishing to the community repository (Module 24).

---

## 2. Functional Requirements

### 2.1 Wizard Skill (SW-01)

| Field | Value |
|-------|-------|
| **ID** | SW-01 |
| **Priority** | P0 |
| **Requirement** | A built-in skill that orchestrates skill creation through guided conversation. |

**Details:**

- Activated by: "help me build a skill", "create a new skill", "skill wizard".
- The wizard is a state machine that walks the user through each creation step.
- Each step asks focused questions, confirms the answer, and moves to the next step.
- The user can go back to a previous step at any time ("go back to tools").
- The wizard maintains state across messages within the same session.
- Progress indicator shows the current step and total steps.

**Acceptance Criteria:**

- [ ] Wizard is available as a built-in skill.
- [ ] Wizard activates via natural language triggers.
- [ ] Wizard walks through creation steps in order.
- [ ] User can go back to previous steps.
- [ ] Progress is maintained across messages in the session.

### 2.2 Template Generation (SW-02)

| Field | Value |
|-------|-------|
| **ID** | SW-02 |
| **Priority** | P0 |
| **Requirement** | The wizard generates a complete skill scaffold: directory structure, skill.yaml manifest, tool JSON schemas, and LLM prompt file. |

**Details:**

- Generated scaffold structure:
  ```
  my-skill/
    skill.yaml         # Manifest with name, version, author, description, tools, prompt
    prompts/
      my-skill.md      # LLM prompt fragment
    tools/
      tool_one.json    # Tool schema
      tool_two.json    # Tool schema
  ```
- The `skill.yaml` is populated from wizard conversation.
- Default version is `0.1.0`.
- Author is read from system config or asked during the wizard.
- All generated files are written to a user-specified directory (default: `~/skills/<skill-name>/`).

**Acceptance Criteria:**

- [ ] Wizard generates a valid skill directory structure.
- [ ] Generated `skill.yaml` contains all required fields.
- [ ] Tool JSON schemas are valid Anthropic tool format.
- [ ] Prompt file is generated and referenced in the manifest.
- [ ] Files are written to the correct directory.

### 2.3 Tool Schema Builder (SW-03)

| Field | Value |
|-------|-------|
| **ID** | SW-03 |
| **Priority** | P0 |
| **Requirement** | The wizard interactively builds tool definitions by asking the user about tool name, description, parameters, and return schema. |

**Details:**

- For each tool, the wizard asks:
  1. "What should this tool be called?" (e.g., `resize_image`).
  2. "Describe what this tool does." (generates the description).
  3. "What parameters does it need?" (the wizard asks about each parameter: name, type, description, required).
  4. "What does it return?" (optional: return schema description).
- After each tool, the wizard shows a preview of the generated JSON schema and asks for confirmation.
- The user can add more tools or finalize.
- Generated schemas conform to the Anthropic tool use JSON Schema format.

**Acceptance Criteria:**

- [ ] Wizard asks structured questions for each tool.
- [ ] Generated tool schemas are valid Anthropic tool format.
- [ ] Schema preview is shown after each tool definition.
- [ ] User can confirm, edit, or redo each tool.

### 2.4 Prompt Engineering (SW-04)

| Field | Value |
|-------|-------|
| **ID** | SW-04 |
| **Priority** | P0 |
| **Requirement** | The wizard generates an effective LLM prompt fragment from the user's skill description and tool definitions. |

**Details:**

- The wizard asks: "Describe the overall purpose and behavior of this skill."
- From the description and the defined tools, the wizard generates a prompt that:
  - Explains the skill's purpose to the LLM.
  - Lists available tools with usage guidance.
  - Includes examples of when to use each tool.
  - Specifies any constraints or safety notes.
- The generated prompt is shown for review.
- The user can refine the prompt with feedback ("make it more concise", "add an example for error handling").

**Acceptance Criteria:**

- [ ] Wizard generates a prompt from skill description and tools.
- [ ] Generated prompt includes tool usage guidance.
- [ ] Prompt is shown for user review.
- [ ] User can refine the prompt iteratively.

### 2.5 Skill Testing (SW-05)

| Field | Value |
|-------|-------|
| **ID** | SW-05 |
| **Priority** | P1 |
| **Requirement** | The wizard tests generated tool schemas with simulated calls and validates the skill manifest. |

**Details:**

- After finalization, the wizard runs validation:
  1. Manifest validation (schema compliance, required fields).
  2. Tool schema validation (valid JSON Schema, required properties present).
  3. Prompt file existence and non-empty check.
  4. Simulated tool calls with sample inputs to verify schema accepts them.
- Test results are displayed as a pass/fail checklist.
- Failures include actionable error messages.

**Acceptance Criteria:**

- [ ] Wizard validates manifest, tool schemas, and prompt.
- [ ] Simulated tool calls are tested against schemas.
- [ ] Results are displayed as a pass/fail checklist.
- [ ] Failures include actionable messages.

### 2.6 Preview Mode (SW-06)

| Field | Value |
|-------|-------|
| **ID** | SW-06 |
| **Priority** | P1 |
| **Requirement** | The wizard can load the generated skill temporarily for a test conversation, then unload it. |

**Details:**

- "Preview my skill" loads the skill into the engine's active skill context.
- The user can test the skill by sending messages that invoke its tools.
- The preview skill has a `[preview]` badge in the status bar.
- "End preview" unloads the skill and returns to the wizard.
- Preview mode does not persist — the skill is only loaded in memory.
- Tool calls during preview are logged and shown in a test report.

**Acceptance Criteria:**

- [ ] Wizard loads the skill temporarily.
- [ ] User can invoke the skill's tools during preview.
- [ ] Preview is clearly indicated in the UI.
- [ ] "End preview" unloads the skill.
- [ ] Tool call log is available after preview.

### 2.7 Publish Integration (SW-07)

| Field | Value |
|-------|-------|
| **ID** | SW-07 |
| **Priority** | P2 |
| **Requirement** | After creation and testing, the wizard offers to publish the skill to the community repository. |

**Details:**

- After finalization, the wizard asks: "Would you like to publish this skill to the community repository?"
- If yes, the wizard invokes the Module 24 publish flow:
  1. Validate the skill (same as CR-10).
  2. Package the skill archive.
  3. Upload to the registry.
- The skill is published at `community` trust level.
- The wizard explains what trust level means and how to request verification.

**Acceptance Criteria:**

- [ ] Wizard offers to publish after finalization.
- [ ] Publishing invokes the Module 24 publish flow.
- [ ] Published skill appears in the community registry.
- [ ] Trust level explanation is provided.

---

## 3. Wizard Flow

The complete wizard flow proceeds through these steps:

```
Step 1: Start
  "Help me build a skill for PDF editing"
  Wizard: "I'll help you create a PDF editing skill. Let's start
           with the basics — what should we call it?"
  User: "pdf-tools"

Step 2: Describe
  Wizard: "Great name. Describe what pdf-tools should do in a
           sentence or two."
  User: "It should let users merge, split, and extract pages
         from PDF files."

Step 3: Define Tools (repeatable)
  Wizard: "Let's define the tools. Tell me about the first one."
  User: "A tool to merge two PDFs into one"
  Wizard: [Generates schema, shows preview, asks for confirmation]
  User: "Looks good. Next tool: split a PDF at a page number."
  Wizard: [Generates schema, shows preview]
  User: "That's all the tools I need."

Step 4: Write Prompt
  Wizard: [Generates prompt from description and tools]
  "Here's the LLM prompt I've generated. Review it in the
   side panel."
  User: "Add a note about handling encrypted PDFs."
  Wizard: [Refines prompt, shows updated version]

Step 5: Generate Manifest
  Wizard: "Here's the complete skill.yaml. Want to add any
           system package dependencies?"
  User: "It needs poppler-utils"
  Wizard: [Updates manifest, shows final version]

Step 6: Test
  Wizard: "Running validation..."
  [Shows pass/fail checklist]
  "All tests passed! Want to preview the skill?"

Step 7: Preview (optional)
  User: "Yes, let me try it."
  Wizard: [Loads skill temporarily]
  User: "Merge report-q1.pdf and report-q2.pdf"
  [Tool is invoked in preview mode]
  User: "End preview"
  Wizard: [Unloads skill]

Step 8: Finalize
  Wizard: "Your skill is ready! Files have been saved to
           ~/skills/pdf-tools/. Would you like to publish it
           to the community repository?"
```

---

## 4. Wizard Tools

The wizard exposes 7 tools to the LLM:

| Tool | Description |
|------|-------------|
| `wizard_start` | Initialize a new skill creation session. Takes skill name and description. |
| `wizard_add_tool` | Add a tool definition to the skill. Takes tool name, description, and parameter specs. |
| `wizard_set_prompt` | Generate or update the LLM prompt for the skill. Takes description text. |
| `wizard_preview` | Load the skill temporarily for testing. Returns preview session ID. |
| `wizard_test` | Run validation tests on the skill. Returns pass/fail results. |
| `wizard_finalize` | Write all skill files to disk. Takes output directory. |
| `wizard_publish` | Publish the finalized skill to the community repository. |

---

## 5. Configuration

New config entries in `/etc/levsha/config.toml`:

```toml
[skills.wizard]
output_dir = "~/skills"                  # Default output directory for created skills
default_author = ""                       # Auto-filled in manifest (read from system config)
auto_test = true                          # Automatically run tests before finalization
```

---

## 6. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Skills System (04) | Upstream | Uses skill manifest format, loading, and validation. |
| Intelligence Engine (03) | Modified | Wizard tools are dispatched by the engine. Temporary skill loading for preview. |
| Community Repository (24) | Consumer | Publish integration calls Module 24's registry API. |
| Split-View (12) | Consumer | Schema previews and manifest display use the content panel. |

---

## 7. Out of Scope

- Visual tool builder or drag-and-drop skill design.
- Automated skill optimization or performance tuning.
- Skill marketplace pricing or monetization integration.
- GUI-based parameter editors.
- Automated code generation for custom tool handlers.
- Multi-language prompt generation (English only in Phase 3).
