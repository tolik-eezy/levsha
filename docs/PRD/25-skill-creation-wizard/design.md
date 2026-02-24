# 25 — Skill Creation Wizard: Design Specification

**Module:** Skill Creation Wizard
**Phase:** 3

---

## 1. Wizard Conversation Flow

The wizard guides the user through skill creation with clear step-by-step prompts. Each step is announced with a progress banner.

### Step Banner

```
User: help me build a skill for PDF editing

Levsha:
  ┌─ Skill Wizard ──────────────────── Step 1 of 6 ───┐
  │                                                      │
  │  Let's build your skill! First, what should we       │
  │  call it? (e.g., "pdf-tools", "email-manager")       │
  │                                                      │
  └──────────────────────────────────────────────────────┘

User: pdf-tools

Levsha:
  ┌─ Skill Wizard ──────────────────── Step 2 of 6 ───┐
  │                                                      │
  │  Great name! Now describe what pdf-tools should      │
  │  do in a sentence or two.                            │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### Step Banner Styling

| Element | Style |
|---------|-------|
| Banner background | `$bg-surface` (#FDFBF7) |
| Banner border | 1px `$accent-copper` (#C67A52) left border (3px) |
| Banner cornerRadius | 8 |
| Banner shadow | 0 1px 3px rgba(58,50,40,0.06) |
| Banner padding | 16px |
| Header "Skill Wizard" | IBM Plex Sans 13px, weight 600, `$accent-copper` |
| Step indicator | IBM Plex Sans 13px, weight 500, `$text-tertiary` (#8C7E6E), right-aligned |
| Body text | IBM Plex Sans 14px, `$text-primary` (#3A3228), line-height 1.6 |
| Example text | IBM Plex Mono 13px, `$text-secondary` (#6B5D4F) |

---

## 2. Tool Definition Flow

The wizard asks structured questions for each tool, then shows a preview of the generated schema.

### Interactive Questions

```
Levsha:
  ┌─ Skill Wizard ──────────────────── Step 3 of 6 ───┐
  │                                                      │
  │  Let's define the tools for pdf-tools.               │
  │  Tell me about the first tool — what should it do?   │
  │                                                      │
  └──────────────────────────────────────────────────────┘

User: A tool to merge two or more PDF files into one

Levsha: I'll create a "merge_pdfs" tool. Let me ask about
        the parameters:

  1. What inputs does it need?
     I'm thinking:
     • input_files (list of file paths)
     • output_file (path for the merged result)

     Does that look right, or do you want to change anything?

User: Yes, that looks right

Levsha: Here's the generated tool schema:
```

### Schema Preview (Split-View Panel)

The generated JSON schema is displayed in the split-view content panel with syntax highlighting.

```
┌─ Tool: merge_pdfs ─────────────────────────────────── [×] ┐
│                                                             │
│   1 │ {                                                     │
│   2 │   "name": "merge_pdfs",                               │
│   3 │   "description": "Merge two or more PDF files into    │
│   4 │     a single output PDF.",                             │
│   5 │   "input_schema": {                                    │
│   6 │     "type": "object",                                  │
│   7 │     "properties": {                                    │
│   8 │       "input_files": {                                 │
│   9 │         "type": "array",                               │
│  10 │         "items": { "type": "string" },                 │
│  11 │         "description": "List of PDF file paths to      │
│  12 │           merge, in order"                             │
│  13 │       },                                               │
│  14 │       "output_file": {                                 │
│  15 │         "type": "string",                              │
│  16 │         "description": "Output path for the merged     │
│  17 │           PDF"                                         │
│  18 │       }                                                │
│  19 │     },                                                 │
│  20 │     "required": ["input_files", "output_file"]         │
│  21 │   }                                                    │
│  22 │ }                                                      │
│                                                             │
│  ┌─────────────┐  ┌──────────┐  ┌──────────────┐          │
│  │   Confirm   │  │  Edit    │  │  Add Another │          │
│  └─────────────┘  └──────────┘  └──────────────┘          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Schema Preview Styling

| Element | Style |
|---------|-------|
| Panel header background | `$bg-secondary` (#F5F1EA) |
| Panel header title | IBM Plex Mono 13px, weight 500, `$text-primary` |
| Code background | `$bg-code` (#F7F4EE) |
| Line numbers | IBM Plex Mono 13px, `$text-tertiary` (#8C7E6E), 48px column |
| Code text | IBM Plex Mono 13px, `$text-primary` |
| Syntax colors | Per `$syn-*` tokens: keys in `$syn-keyword` (copper), strings in `$syn-string` (green) |
| Confirm button | `$accent-copper` (#C67A52) bg, white text, cornerRadius 6 |
| Edit button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |
| Add Another button | `$bg-secondary` bg, `$accent-copper` text, cornerRadius 6 |

---

## 3. Prompt Preview (Split-View Panel)

After the wizard generates the LLM prompt, it is displayed in the content panel for review.

```
┌─ LLM Prompt: pdf-tools ───────────────────────────── [×] ┐
│                                                             │
│  # PDF Tools                                                │
│                                                             │
│  You have access to PDF manipulation tools. Use them        │
│  when the user asks to work with PDF files.                 │
│                                                             │
│  ## Available Tools                                         │
│                                                             │
│  - **merge_pdfs** — Merge two or more PDF files into one.  │
│    Use when the user wants to combine PDFs.                 │
│                                                             │
│  - **split_pdf** — Split a PDF at a specific page.          │
│    Use when the user wants to extract or separate pages.    │
│                                                             │
│  ## Notes                                                   │
│                                                             │
│  - Always confirm file paths with the user before           │
│    modifying files.                                         │
│  - If a PDF is encrypted, inform the user that it cannot    │
│    be processed without the password.                       │
│                                                             │
│  ┌────────────────┐  ┌──────────────────────┐              │
│  │   Looks Good   │  │  Refine with Notes  │              │
│  └────────────────┘  └──────────────────────┘              │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Prompt Preview Styling

| Element | Style |
|---------|-------|
| Code background | `$bg-code` (#F7F4EE) |
| Markdown headings | IBM Plex Sans 15px (h2) / 13px (h3), weight 600, `$text-primary` |
| Markdown body | IBM Plex Sans 14px, `$text-primary`, line-height 1.6 |
| Bold text | IBM Plex Sans 14px, weight 600, `$text-primary` |
| Bullet points | IBM Plex Sans 14px, `$text-secondary`, 16px left indent |
| Looks Good button | `$accent-copper` bg, white text, cornerRadius 6 |
| Refine button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |

---

## 4. Skill Manifest Preview (Split-View Panel)

Before finalization, the complete manifest is shown in the content panel.

```
┌─ skill.yaml ───────────────────────────────────────── [×] ┐
│                                                             │
│   1 │ name: pdf-tools                                       │
│   2 │ version: 0.1.0                                        │
│   3 │ description: "Merge, split, and extract pages from    │
│   4 │   PDF files"                                          │
│   5 │ author: user                                          │
│   6 │ prompt: prompts/pdf-tools.md                          │
│   7 │ tools:                                                │
│   8 │   - tools/merge_pdfs.json                             │
│   9 │   - tools/split_pdf.json                              │
│  10 │ requires:                                             │
│  11 │   packages:                                           │
│  12 │     - poppler-utils                                   │
│                                                             │
│  ┌──────────────┐  ┌──────────┐                            │
│  │   Finalize   │  │   Edit   │                            │
│  └──────────────┘  └──────────┘                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Manifest Preview Styling

Identical to the File Preview styling from Module 12:

| Element | Style |
|---------|-------|
| Code background | `$bg-code` (#F7F4EE) |
| Line numbers | IBM Plex Mono 13px, `$text-tertiary`, 48px column |
| YAML keys | IBM Plex Mono 13px, `$syn-keyword` (copper) |
| YAML values | IBM Plex Mono 13px, `$syn-string` (green) |
| Finalize button | `$accent-copper` bg, white text, cornerRadius 6 |
| Edit button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |

---

## 5. Test Results Display

After running validation, results are shown as a pass/fail checklist in chat.

```
Levsha:
  ┌─ Skill Wizard ──────────────────── Step 5 of 6 ───┐
  │                                                      │
  │  Running validation tests...                         │
  │                                                      │
  │  ✓ Manifest schema valid                             │
  │  ✓ Tool schema: merge_pdfs — valid                   │
  │  ✓ Tool schema: split_pdf — valid                    │
  │  ✓ Prompt file present and non-empty                 │
  │  ✓ Simulated call: merge_pdfs — accepted             │
  │  ✓ Simulated call: split_pdf — accepted              │
  │                                                      │
  │  All 6 tests passed!                                 │
  │                                                      │
  │  ┌──────────────────┐  ┌──────────────────────┐     │
  │  │  Preview Skill   │  │  Finalize & Save     │     │
  │  └──────────────────┘  └──────────────────────┘     │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### Test Failure Display

```
  ✓ Manifest schema valid
  ✗ Tool schema: merge_pdfs — invalid
      Missing required field: "description"
  ✓ Tool schema: split_pdf — valid
  ✓ Prompt file present and non-empty

  1 test failed. Fix the issue and try again.
```

### Test Result Styling

| Element | Style |
|---------|-------|
| Pass icon (✓) | `$accent-green` (#62B37B), Lucide `check` |
| Fail icon (✗) | `$accent-copper` (#C67A52), Lucide `x-circle` |
| Test name | IBM Plex Sans 13px, `$text-primary` |
| Error detail | IBM Plex Sans 12px, `$accent-copper`, 24px left indent |
| Summary (all passed) | IBM Plex Sans 14px, weight 600, `$accent-green` |
| Summary (failures) | IBM Plex Sans 14px, weight 600, `$accent-copper` |
| Preview button | `$bg-secondary` bg, `$accent-copper` text, cornerRadius 6 |
| Finalize button | `$accent-copper` bg, white text, cornerRadius 6 |

---

## 6. Preview Mode Indicator

When the skill is loaded in preview mode, a banner appears at the top of the chat.

```
┌──────────────────────────────────────────────────────────┐
│  [preview] pdf-tools loaded · Say "end preview" to stop  │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  User: merge report-q1.pdf and report-q2.pdf             │
│                                                          │
│  Levsha: I'll merge those PDFs for you.                  │
│                                                          │
│  ┌─ Tool Call: merge_pdfs ───────────────────────────┐   │
│  │  input_files: ["report-q1.pdf", "report-q2.pdf"]  │   │
│  │  output_file: "merged-report.pdf"                  │   │
│  └───────────────────────────────────────────────────┘   │
│                                                          │
```

### Preview Banner Styling

| Element | Style |
|---------|-------|
| Banner background | `$accent-copper` (#C67A52) at 10% opacity |
| Banner border | 1px bottom `$accent-copper` |
| Banner padding | 8px 16px |
| [preview] badge | IBM Plex Mono 11px, weight 600, `$accent-copper` bg, white text, cornerRadius 4, padding 2px 6px |
| Skill name | IBM Plex Sans 13px, weight 500, `$text-primary` |
| Instructions text | IBM Plex Sans 12px, `$text-secondary` |

### Tool Call Log Styling

| Element | Style |
|---------|-------|
| Container | `$bg-code` (#F7F4EE), 1px `$border-primary`, cornerRadius 6 |
| Header | IBM Plex Sans 12px, weight 600, `$text-tertiary`, uppercase |
| Parameter names | IBM Plex Mono 12px, `$syn-keyword` (copper) |
| Parameter values | IBM Plex Mono 12px, `$syn-string` (green) |

---

## 7. Publish Confirmation

After finalization, the wizard offers to publish.

```
Levsha:
  ┌─ Skill Wizard ──────────────────── Complete ───────┐
  │                                                      │
  │  Your skill has been saved to ~/skills/pdf-tools/    │
  │                                                      │
  │  ┌─ Files Created ──────────────────────────────┐    │
  │  │  skill.yaml                                   │    │
  │  │  prompts/pdf-tools.md                         │    │
  │  │  tools/merge_pdfs.json                        │    │
  │  │  tools/split_pdf.json                         │    │
  │  └───────────────────────────────────────────────┘    │
  │                                                      │
  │  Would you like to publish this skill to the         │
  │  community repository?                               │
  │                                                      │
  │  Published skills start at Community trust level     │
  │  (gray badge). You can request verification from     │
  │  the Levsha team after publishing.                   │
  │                                                      │
  │  ┌───────────────┐  ┌──────────────────────────┐    │
  │  │   Publish     │  │  No, just save locally   │    │
  │  └───────────────┘  └──────────────────────────┘    │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### Completion Styling

| Element | Style |
|---------|-------|
| File list container | `$bg-code` (#F7F4EE), 1px `$border-primary`, cornerRadius 6 |
| File names | IBM Plex Mono 13px, `$text-primary` |
| Trust level explanation | IBM Plex Sans 13px, `$text-secondary` |
| Publish button | `$accent-copper` bg, white text, cornerRadius 6 |
| Save locally button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |

---

## 8. Navigation Between Steps

The user can navigate back to previous steps at any time.

```
User: go back to the tools step

Levsha:
  ┌─ Skill Wizard ──────────────────── Step 3 of 6 ───┐
  │                                                      │
  │  Back to tool definitions. You have 2 tools:         │
  │                                                      │
  │  1. merge_pdfs — Merge two or more PDFs              │
  │  2. split_pdf — Split a PDF at a page number         │
  │                                                      │
  │  You can edit an existing tool, add a new one,       │
  │  or continue to the next step.                       │
  │                                                      │
  │  ┌──────────────┐  ┌─────────────┐  ┌──────────┐   │
  │  │  Add Tool    │  │  Edit Tool  │  │  Next    │   │
  │  └──────────────┘  └─────────────┘  └──────────┘   │
  │                                                      │
  └──────────────────────────────────────────────────────┘
```

### Navigation Button Styling

| Element | Style |
|---------|-------|
| Add Tool button | `$bg-secondary` bg, `$accent-copper` text, cornerRadius 6 |
| Edit Tool button | `$bg-secondary` bg, `$text-primary` text, cornerRadius 6 |
| Next button | `$accent-copper` bg, white text, cornerRadius 6 |
| Back navigation | Triggered by natural language ("go back", "previous step") |

---

## 9. Assets Used

| Asset | Usage |
|-------|-------|
| `logo2.png` | Levsha avatar in wizard messages |
| Lucide `wand-2` | Wizard skill icon |
| Lucide `check` | Passed test icon |
| Lucide `x-circle` | Failed test icon |
| Lucide `play` | Preview mode icon |
| Lucide `file-plus` | Template generation icon |
| Lucide `code-2` | Tool schema icon |
| Lucide `file-text` | Prompt file icon |
| Lucide `upload` | Publish icon |
| Lucide `arrow-left` | Navigate back indicator |
| Lucide `arrow-right` | Navigate forward indicator |

---

## 10. Dependencies

| Dependency | Direction | Description |
|------------|-----------|-------------|
| Chat Shell (02) | Modified | Wizard step banners, preview mode banner, test results display. |
| Intelligence Engine (03) | Modified | Wizard tool dispatch, temporary skill loading for preview. |
| Split-View (12) | Consumer | Schema preview, prompt preview, and manifest preview in content panel. |
| Community Repository (24) | Consumer | Publish integration for the finalized skill. |
| Skills System (04) | Upstream | Uses manifest format, validation, and loading framework. |
