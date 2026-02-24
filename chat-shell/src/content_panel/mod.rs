//! Content panel — split-view panel for file previews, diffs, images, progress, and agent activity.

mod agent_activity;
mod diff;
mod image;
mod progress;
mod text;

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Orientation};

use levsha_engine::types::{ContentType, ContentUpdateData};

use self::agent_activity::AgentActivityRenderer;
use self::diff::DiffRenderer;
use self::image::ImageRenderer;
use self::progress::ProgressRenderer;
use self::text::TextRenderer;

/// The content panel displayed on the right side of the split view.
#[derive(Clone)]
pub struct ContentPanel {
    container: gtk4::Box,
    title_label: gtk4::Label,
    stack: gtk4::Stack,
    text_renderer: TextRenderer,
    image_renderer: ImageRenderer,
    diff_renderer: DiffRenderer,
    progress_renderer: ProgressRenderer,
    agent_activity: AgentActivityRenderer,
    current_content_id: Rc<RefCell<Option<String>>>,
}

impl ContentPanel {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("content-panel");
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Header bar.
        let header = gtk4::Box::new(Orientation::Horizontal, 8);
        header.add_css_class("content-panel-header");

        let title_label = gtk4::Label::new(Some("Content"));
        title_label.add_css_class("content-panel-title");
        title_label.set_halign(Align::Start);
        title_label.set_hexpand(true);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        header.append(&title_label);

        let close_btn = gtk4::Button::with_label("\u{2715}");
        close_btn.add_css_class("content-panel-close");
        close_btn.set_halign(Align::End);
        header.append(&close_btn);

        container.append(&header);

        // Stack with four named children.
        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(200);
        stack.set_vexpand(true);
        stack.set_hexpand(true);

        let text_renderer = TextRenderer::new();
        let image_renderer = ImageRenderer::new();
        let diff_renderer = DiffRenderer::new();
        let progress_renderer = ProgressRenderer::new();
        let agent_activity = AgentActivityRenderer::new();

        stack.add_named(text_renderer.widget(), Some("text"));
        stack.add_named(image_renderer.widget(), Some("image"));
        stack.add_named(diff_renderer.widget(), Some("diff"));
        stack.add_named(progress_renderer.widget(), Some("progress"));
        stack.add_named(agent_activity.widget(), Some("agent_activity"));

        container.append(&stack);

        let panel = Self {
            container,
            title_label,
            stack,
            text_renderer,
            image_renderer,
            diff_renderer,
            progress_renderer,
            agent_activity,
            current_content_id: Rc::new(RefCell::new(None)),
        };

        // Close button hides the panel.
        {
            let p = panel.clone();
            close_btn.connect_clicked(move |_| {
                p.hide();
            });
        }

        panel
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Open content in the panel.
    pub fn open(&self, content_id: &str, content_type: &ContentType, title: &str, content: &str) {
        *self.current_content_id.borrow_mut() = Some(content_id.to_string());
        self.title_label.set_text(title);

        match content_type {
            ContentType::FilePreview { language, .. } => {
                self.text_renderer.set_content(content, language);
                self.stack.set_visible_child_name("text");
            }
            ContentType::DiffView {
                old_content,
                new_content,
                ..
            } => {
                self.diff_renderer.set_diff(old_content, new_content);
                self.stack.set_visible_child_name("diff");
            }
            ContentType::ImageView {
                mime_type, data, ..
            } => {
                self.image_renderer.set_image(data, mime_type);
                self.stack.set_visible_child_name("image");
            }
            ContentType::ProgressView {
                operation, total, ..
            } => {
                self.progress_renderer.start(operation, *total);
                self.stack.set_visible_child_name("progress");
            }
            ContentType::AgentActivity { .. } => {
                self.agent_activity.clear();
                self.stack.set_visible_child_name("agent_activity");
            }
        }

        self.show();
    }

    /// Update content if the content_id matches.
    pub fn update(&self, content_id: &str, data: &ContentUpdateData) {
        let current = self.current_content_id.borrow();
        if current.as_deref() != Some(content_id) {
            return;
        }
        drop(current);

        match data {
            ContentUpdateData::ReplaceText { text } => {
                self.text_renderer.replace_content(text);
            }
            ContentUpdateData::AppendText { text } => {
                if self.stack.visible_child_name().as_deref() == Some("progress") {
                    self.progress_renderer.append_log(text);
                } else {
                    self.text_renderer.append_content(text);
                }
            }
            ContentUpdateData::Progress { current, message } => {
                self.progress_renderer.update(*current, message);
            }
            ContentUpdateData::ReplaceDiff {
                old_content,
                new_content,
            } => {
                self.diff_renderer.set_diff(old_content, new_content);
            }
            ContentUpdateData::AgentEvent {
                event_type,
                content,
                file_path,
            } => {
                self.agent_activity
                    .add_event(event_type, content, file_path.as_deref());
            }
        }
    }

    /// Close the panel if content_id matches.
    pub fn close(&self, content_id: &str) {
        let current = self.current_content_id.borrow();
        if current.as_deref() == Some(content_id) {
            drop(current);
            *self.current_content_id.borrow_mut() = None;
            self.hide();
        }
    }

    /// Show the content panel.
    pub fn show(&self) {
        self.container.set_visible(true);
    }

    /// Hide the content panel.
    pub fn hide(&self) {
        self.container.set_visible(false);
    }

    /// Toggle visibility.
    pub fn toggle(&self) {
        if self.is_visible() {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Check if the panel is visible.
    pub fn is_visible(&self) -> bool {
        self.container.is_visible()
    }
}
