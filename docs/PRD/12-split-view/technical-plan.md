# 12 — Split-View Content Rendering: Technical Plan

**Module:** Split-View Content Rendering (L3)
**Language:** Rust
**Phase:** 2

---

## 1. Crate Structure

The split-view is integrated into the existing `chat-shell/` crate as new modules.

```
chat-shell/
  src/
    ui/
      split_view/
        mod.rs              # SplitViewContainer (GtkPaned wrapper)
        panel.rs            # ContentPanel widget
        header.rs           # PanelHeader with title, controls, close
        file_preview.rs     # Syntax-highlighted file renderer
        diff_view.rs        # Diff renderer (unified/side-by-side)
        image_view.rs       # Image display with zoom
        progress_view.rs    # Build/task progress display
    ipc/
      content.rs            # ContentOpen/Update/Close message handling
```

### New Dependencies

```toml
# Added to chat-shell/Cargo.toml
gdk-pixbuf = "0.20"    # Image loading and display
```

---

## 2. SplitView Container

The top-level container uses `GtkPaned` to manage the chat and content panels.

```rust
pub struct SplitViewContainer {
    paned: gtk4::Paned,
    chat_view: ChatView,
    content_panel: ContentPanel,
    is_panel_open: Cell<bool>,
}

impl SplitViewContainer {
    pub fn new(chat_view: ChatView) -> Self {
        let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
        let content_panel = ContentPanel::new();

        paned.set_start_child(Some(&chat_view.widget()));
        paned.set_resize_start_child(true);
        paned.set_shrink_start_child(false);

        // Content panel starts hidden
        paned.set_end_child(Some(&content_panel.widget()));
        paned.set_resize_end_child(true);
        paned.set_shrink_end_child(false);

        // Minimum sizes
        chat_view.widget().set_width_request(300);
        content_panel.widget().set_width_request(0); // hidden initially

        Self {
            paned,
            chat_view,
            content_panel,
            is_panel_open: Cell::new(false),
        }
    }

    pub fn open_content(&self, content: ContentPayload) {
        self.content_panel.set_content(content);
        self.content_panel.widget().set_width_request(300);

        // Animate panel open
        let target_position = self.paned.allocated_width() / 2;
        self.animate_divider(target_position, Duration::from_millis(250));

        self.is_panel_open.set(true);
    }

    pub fn close_content(&self) {
        self.animate_divider(
            self.paned.allocated_width(),
            Duration::from_millis(200),
        );
        self.is_panel_open.set(false);
    }

    pub fn update_content(&self, update: ContentUpdate) {
        self.content_panel.update(update);
    }

    fn animate_divider(&self, target: i32, duration: Duration) {
        let paned = self.paned.clone();
        let start = paned.position();
        let start_time = std::time::Instant::now();

        glib::timeout_add_local(Duration::from_millis(16), move || {
            let elapsed = start_time.elapsed().as_millis() as f64;
            let total = duration.as_millis() as f64;
            let t = (elapsed / total).min(1.0);
            // ease-out: t * (2 - t)
            let eased = t * (2.0 - t);
            let pos = start + ((target - start) as f64 * eased) as i32;
            paned.set_position(pos);

            if t >= 1.0 {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}
```

---

## 3. Content Panel

```rust
pub struct ContentPanel {
    container: gtk4::Box,
    header: PanelHeader,
    stack: gtk4::Stack,     // Switches between content types
    file_preview: FilePreview,
    diff_view: DiffView,
    image_view: ImageView,
    progress_view: ProgressView,
}

pub enum ContentPayload {
    File {
        path: String,
        content: String,
        language: Option<String>,
    },
    Diff {
        diff_text: String,
        file_path: Option<String>,
    },
    Image {
        path: String,
    },
    Progress {
        title: String,
        total_steps: usize,
    },
}

pub enum ContentUpdate {
    ProgressStep { index: usize, state: StepState, label: String },
    ProgressPercent(f64),
    AppendOutput(String),
}

impl ContentPanel {
    pub fn set_content(&self, payload: ContentPayload) {
        match payload {
            ContentPayload::File { path, content, language } => {
                self.header.set_title(&path);
                self.file_preview.set_content(&content, language.as_deref());
                self.stack.set_visible_child_name("file");
            }
            ContentPayload::Diff { diff_text, file_path } => {
                self.header.set_title(
                    &file_path.as_deref().unwrap_or("Diff")
                );
                self.diff_view.set_diff(&diff_text);
                self.stack.set_visible_child_name("diff");
            }
            ContentPayload::Image { path } => {
                self.header.set_title(&path);
                self.image_view.load_image(&path);
                self.stack.set_visible_child_name("image");
            }
            ContentPayload::Progress { title, total_steps } => {
                self.header.set_title(&title);
                self.progress_view.reset(total_steps);
                self.stack.set_visible_child_name("progress");
            }
        }
    }
}
```

---

## 4. File Preview Implementation

```rust
pub struct FilePreview {
    scrolled: gtk4::ScrolledWindow,
    text_view: gtk4::TextView,
    line_numbers: gtk4::DrawingArea,
    highlighter: syntect::highlighting::Highlighter,
}

impl FilePreview {
    pub fn set_content(&self, content: &str, language: Option<&str>) {
        let buffer = self.text_view.buffer();
        buffer.set_text("");

        // Apply syntax highlighting
        let syntax = language
            .and_then(|lang| self.syntax_set.find_syntax_by_token(lang))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, &self.theme_set.themes["levsha"]);
        for line in LinesWithEndings::from(content) {
            let ranges = h.highlight_line(line, &self.syntax_set).unwrap();
            let pango_markup = styled_ranges_to_pango(&ranges);
            let mut end = buffer.end_iter();
            buffer.insert_markup(&mut end, &pango_markup);
        }

        // Update line numbers
        let line_count = content.lines().count();
        self.line_numbers.set_content_height(line_count as i32 * 20); // 20px per line
        self.line_numbers.queue_draw();
    }
}
```

---

## 5. Diff View Implementation

```rust
pub struct DiffView {
    scrolled: gtk4::ScrolledWindow,
    text_view: gtk4::TextView,
}

impl DiffView {
    pub fn set_diff(&self, diff_text: &str) {
        let buffer = self.text_view.buffer();
        buffer.set_text("");

        // Create tags for diff styling
        let tag_table = buffer.tag_table();
        self.ensure_tags(&tag_table);

        for line in diff_text.lines() {
            let mut end = buffer.end_iter();
            let tag_name = match line.chars().next() {
                Some('+') => "added",
                Some('-') => "removed",
                Some('@') => "hunk-header",
                _ => "context",
            };
            buffer.insert_with_tags_by_name(&mut end, &format!("{}\n", line), &[tag_name]);
        }
    }

    fn ensure_tags(&self, table: &gtk4::TextTagTable) {
        if table.lookup("added").is_none() {
            let tag = gtk4::TextTag::builder()
                .name("added")
                .background("rgba(98,179,123,0.10)")  // $accent-green at 10%
                .foreground("#62B37B")
                .build();
            table.add(&tag);
        }
        if table.lookup("removed").is_none() {
            let tag = gtk4::TextTag::builder()
                .name("removed")
                .background("rgba(198,122,82,0.10)")  // $accent-copper at 10%
                .foreground("#C67A52")
                .build();
            table.add(&tag);
        }
        if table.lookup("hunk-header").is_none() {
            let tag = gtk4::TextTag::builder()
                .name("hunk-header")
                .background("#F5F1EA")  // $bg-secondary
                .foreground("#6B5D4F")  // $text-secondary
                .build();
            table.add(&tag);
        }
    }
}
```

---

## 6. IPC Protocol Extensions

New message types added to the L2↔L3 protocol:

```rust
// Engine -> Chat Shell
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentMessage {
    #[serde(rename = "content_open")]
    Open {
        content_type: ContentType,
        data: serde_json::Value,
        panel_width: Option<i32>,
    },
    #[serde(rename = "content_update")]
    Update {
        update_type: UpdateType,
        data: serde_json::Value,
    },
    #[serde(rename = "content_close")]
    Close,
}

#[derive(Serialize, Deserialize)]
pub enum ContentType {
    #[serde(rename = "file")]
    File,
    #[serde(rename = "diff")]
    Diff,
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "progress")]
    Progress,
}

// Chat Shell -> Engine
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentAction {
    #[serde(rename = "content_action")]
    Action {
        action: String,  // "approve", "cancel", "close"
        data: Option<serde_json::Value>,
    },
}
```

---

## 7. Implementation Stages

**Stage 1 — GtkPaned Layout (2-3 days)**
1. Wrap existing ChatView in a GtkPaned.
2. Create empty ContentPanel with header and close button.
3. Open/close animation.
4. Keyboard shortcuts (Ctrl+W, Escape).
5. Divider drag and minimum width enforcement.

**Stage 2 — File Preview (2-3 days)**
1. FilePreview widget with line numbers and syntax highlighting.
2. Reuse existing syntect integration from code blocks.
3. Scroll synchronization for line numbers.
4. Word wrap toggle.

**Stage 3 — Diff View (1-2 days)**
1. DiffView widget with color-coded lines.
2. Unified diff parsing.
3. Hunk navigation.

**Stage 4 — Image View (1 day)**
1. Image loading via gdk-pixbuf.
2. Scale-to-fit with aspect ratio.
3. Zoom controls.

**Stage 5 — Progress View (1 day)**
1. Progress bar widget.
2. Step list with completion states.
3. Streaming output log.

**Stage 6 — IPC Integration (1 day)**
1. Add ContentOpen/Update/Close to IPC protocol.
2. Wire engine content messages to panel opening.
3. Wire panel actions back to engine.

---

## 8. Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|-----------|
| `split_view/mod.rs` | Panel open/close state, divider position |
| `file_preview.rs` | Syntax detection, line number calculation |
| `diff_view.rs` | Diff line classification, tag application |
| `ipc/content.rs` | ContentMessage serialization/deserialization |

### Integration Tests

| Test | Method |
|------|--------|
| Panel open/close | Send ContentOpen via IPC, verify panel visible |
| File preview | Send file content, verify syntax highlighting |
| Diff rendering | Send diff text, verify color coding |
| Progress updates | Send sequential progress updates, verify display |
| Panel resize | Programmatic divider drag, verify minimum widths |
| Close via keyboard | Send Ctrl+W event, verify panel closes |
