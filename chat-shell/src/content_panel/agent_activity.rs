//! Agent activity renderer — streaming event log from the coding agent.

use std::cell::Cell;

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, ScrolledWindow};

/// Renders a streaming activity log from the coding agent subprocess.
#[derive(Clone)]
pub struct AgentActivityRenderer {
    container: gtk4::Box,
    scroll: ScrolledWindow,
    content_box: gtk4::Box,
    auto_scroll: std::rc::Rc<Cell<bool>>,
}

impl AgentActivityRenderer {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("agent-activity-renderer");
        container.set_hexpand(true);
        container.set_vexpand(true);

        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .build();

        let content_box = gtk4::Box::new(Orientation::Vertical, 0);
        content_box.set_valign(Align::Start);
        content_box.set_hexpand(true);
        scroll.set_child(Some(&content_box));

        container.append(&scroll);

        let auto_scroll = std::rc::Rc::new(Cell::new(true));

        // Track user scrolling to pause/resume auto-scroll.
        {
            let vadj = scroll.vadjustment();
            let auto_scroll_ref = auto_scroll.clone();
            vadj.connect_value_changed(move |adj| {
                let at_bottom =
                    adj.value() >= adj.upper() - adj.page_size() - 1.0;
                auto_scroll_ref.set(at_bottom);
            });
        }

        Self {
            container,
            scroll,
            content_box,
            auto_scroll,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Add a new event entry to the activity stream.
    pub fn add_event(&self, event_type: &str, content: &str, file_path: Option<&str>) {
        // Add separator if not the first entry.
        if self.content_box.first_child().is_some() {
            let sep = gtk4::Separator::new(Orientation::Horizontal);
            sep.add_css_class("agent-event-separator");
            self.content_box.append(&sep);
        }

        let entry = gtk4::Box::new(Orientation::Vertical, 4);
        entry.add_css_class("agent-activity-entry");

        match event_type {
            "thinking" => {
                let prefix = gtk4::Label::new(Some("\u{1F4AD} Thinking"));
                prefix.add_css_class("agent-event-prefix");
                prefix.set_halign(Align::Start);
                entry.append(&prefix);

                if !content.is_empty() {
                    let text = gtk4::Label::new(Some(content));
                    text.add_css_class("agent-event-thinking-text");
                    text.set_halign(Align::Start);
                    text.set_wrap(true);
                    text.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                    text.set_xalign(0.0);
                    text.set_selectable(true);
                    text.set_focusable(false);
                    entry.append(&text);
                }
            }

            "file_read" => {
                let path_display = file_path.unwrap_or("unknown");
                let prefix_text = format!("\u{1F50D} Reading {}", path_display);
                let prefix = gtk4::Label::new(Some(&prefix_text));
                prefix.add_css_class("agent-event-prefix");
                prefix.set_halign(Align::Start);
                prefix.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
                entry.append(&prefix);

                if let Some(fp) = file_path {
                    let path_label = gtk4::Label::new(Some(fp));
                    path_label.add_css_class("agent-event-path");
                    path_label.set_halign(Align::Start);
                    path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
                    path_label.set_selectable(true);
                    path_label.set_focusable(false);
                    entry.append(&path_label);
                }

                if !content.is_empty() {
                    let code = gtk4::Label::new(Some(content));
                    code.add_css_class("agent-event-code");
                    code.set_halign(Align::Start);
                    code.set_wrap(true);
                    code.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                    code.set_xalign(0.0);
                    code.set_selectable(true);
                    code.set_focusable(false);
                    entry.append(&code);
                }
            }

            "file_edit" => {
                let path_display = file_path.unwrap_or("unknown");
                let prefix_text = format!("\u{270F}\u{FE0F} Editing {}", path_display);
                let prefix = gtk4::Label::new(Some(&prefix_text));
                prefix.add_css_class("agent-event-prefix");
                prefix.set_halign(Align::Start);
                prefix.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
                entry.append(&prefix);

                if let Some(fp) = file_path {
                    let path_label = gtk4::Label::new(Some(fp));
                    path_label.add_css_class("agent-event-path");
                    path_label.set_halign(Align::Start);
                    path_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
                    path_label.set_selectable(true);
                    path_label.set_focusable(false);
                    entry.append(&path_label);
                }

                // Show diff-like content with green for additions.
                if !content.is_empty() {
                    let diff_box = gtk4::Box::new(Orientation::Vertical, 0);
                    for line in content.lines() {
                        let line_label = gtk4::Label::new(Some(line));
                        line_label.set_halign(Align::Start);
                        line_label.set_xalign(0.0);
                        line_label.set_selectable(true);
                        line_label.set_focusable(false);
                        if line.starts_with('+') {
                            line_label.add_css_class("agent-event-added");
                        } else {
                            line_label.add_css_class("agent-event-code");
                        }
                        diff_box.append(&line_label);
                    }
                    entry.append(&diff_box);
                }
            }

            "bash_command" => {
                let prefix_text = format!("\u{25B6} Running: {}", content);
                let prefix = gtk4::Label::new(Some(&prefix_text));
                prefix.add_css_class("agent-event-prefix");
                prefix.set_halign(Align::Start);
                prefix.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                entry.append(&prefix);

                let cmd = gtk4::Label::new(Some(content));
                cmd.add_css_class("agent-event-code");
                cmd.set_halign(Align::Start);
                cmd.set_wrap(true);
                cmd.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                cmd.set_xalign(0.0);
                cmd.set_selectable(true);
                cmd.set_focusable(false);
                entry.append(&cmd);
            }

            "code_search" => {
                let prefix_text = format!("\u{1F50E} Searching: {}", content);
                let prefix = gtk4::Label::new(Some(&prefix_text));
                prefix.add_css_class("agent-event-prefix");
                prefix.set_halign(Align::Start);
                prefix.set_ellipsize(gtk4::pango::EllipsizeMode::End);
                entry.append(&prefix);
            }

            "complete" => {
                let prefix_text = format!("\u{2705} Agent complete ({})", content);
                let prefix = gtk4::Label::new(Some(&prefix_text));
                prefix.add_css_class("agent-event-complete");
                prefix.set_halign(Align::Start);
                entry.append(&prefix);
            }

            "error" => {
                let prefix_text = format!("\u{274C} Error: {}", content);
                let prefix = gtk4::Label::new(Some(&prefix_text));
                prefix.add_css_class("agent-event-error");
                prefix.set_halign(Align::Start);
                prefix.set_wrap(true);
                prefix.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                prefix.set_xalign(0.0);
                entry.append(&prefix);
            }

            "timeout" => {
                let prefix = gtk4::Label::new(Some("\u{23F0} Agent timed out"));
                prefix.add_css_class("agent-event-error");
                prefix.set_halign(Align::Start);
                entry.append(&prefix);
            }

            _ => {
                // Unknown event type — display raw content.
                let text = gtk4::Label::new(Some(content));
                text.add_css_class("agent-event-code");
                text.set_halign(Align::Start);
                text.set_wrap(true);
                text.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                text.set_xalign(0.0);
                text.set_selectable(true);
                text.set_focusable(false);
                entry.append(&text);
            }
        }

        self.content_box.append(&entry);

        // Auto-scroll to bottom.
        if self.auto_scroll.get() {
            let scroll = self.scroll.clone();
            glib::idle_add_local_once(move || {
                let vadj = scroll.vadjustment();
                vadj.set_value(vadj.upper() - vadj.page_size());
            });
        }
    }

    /// Remove all entries from the activity stream.
    pub fn clear(&self) {
        while let Some(child) = self.content_box.first_child() {
            self.content_box.remove(&child);
        }
        self.auto_scroll.set(true);
    }
}
