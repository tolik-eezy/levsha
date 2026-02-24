//! Text renderer — syntax-highlighted file preview with line numbers.

use gtk4::pango::WrapMode;
use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, ScrolledWindow};

use crate::message_widget::syntax;

/// Renders a text file with line numbers and syntax highlighting.
#[derive(Clone)]
pub struct TextRenderer {
    container: gtk4::Box,
    line_numbers: gtk4::Box,
    code_label: gtk4::Label,
    _scroll: ScrolledWindow,
    language: std::cell::RefCell<String>,
}

impl TextRenderer {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("text-renderer");
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Toolbar: info + wrap toggle.
        let toolbar = gtk4::Box::new(Orientation::Horizontal, 8);
        toolbar.set_margin_start(16);
        toolbar.set_margin_end(16);
        toolbar.set_margin_top(8);
        toolbar.set_margin_bottom(8);

        let wrap_btn = gtk4::ToggleButton::with_label("Wrap");
        wrap_btn.add_css_class("image-zoom-button");
        wrap_btn.set_halign(Align::End);
        wrap_btn.set_hexpand(true);
        toolbar.append(&wrap_btn);

        container.append(&toolbar);

        // Scroll area with line numbers + code content.
        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .build();

        let inner = gtk4::Box::new(Orientation::Horizontal, 0);

        // Line numbers column.
        let line_numbers = gtk4::Box::new(Orientation::Vertical, 0);
        line_numbers.add_css_class("text-line-numbers");
        line_numbers.set_valign(Align::Start);
        inner.append(&line_numbers);

        // Code content.
        let code_label = gtk4::Label::new(None);
        code_label.add_css_class("text-code-content");
        code_label.set_wrap(false);
        code_label.set_xalign(0.0);
        code_label.set_yalign(0.0);
        code_label.set_use_markup(true);
        code_label.set_selectable(true);
        code_label.set_focusable(false);
        code_label.set_hexpand(true);
        code_label.set_valign(Align::Start);
        inner.append(&code_label);

        scroll.set_child(Some(&inner));
        container.append(&scroll);

        // Wire wrap toggle.
        let label_ref = code_label.clone();
        wrap_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                label_ref.set_wrap(true);
                label_ref.set_wrap_mode(WrapMode::WordChar);
            } else {
                label_ref.set_wrap(false);
            }
        });

        Self {
            container,
            line_numbers,
            code_label,
            _scroll: scroll,
            language: std::cell::RefCell::new(String::new()),
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Set the content with syntax highlighting and line numbers.
    pub fn set_content(&self, text: &str, language: &str) {
        *self.language.borrow_mut() = language.to_string();
        self.update_line_numbers(text);

        let highlighted = syntax::highlight_code(language, text);
        self.code_label.set_markup(&highlighted);
    }

    /// Replace the content without changing language.
    pub fn replace_content(&self, text: &str) {
        let lang = self.language.borrow().clone();
        self.update_line_numbers(text);
        let highlighted = syntax::highlight_code(&lang, text);
        self.code_label.set_markup(&highlighted);
    }

    /// Append text to the existing content.
    pub fn append_content(&self, text: &str) {
        let current = self.code_label.text().to_string();
        let combined = if current.is_empty() {
            text.to_string()
        } else {
            format!("{}\n{}", current, text)
        };
        self.replace_content(&combined);
    }

    fn update_line_numbers(&self, text: &str) {
        // Clear existing line numbers.
        while let Some(child) = self.line_numbers.first_child() {
            self.line_numbers.remove(&child);
        }

        let line_count = text.lines().count().max(1);
        for i in 1..=line_count {
            let label = gtk4::Label::new(Some(&i.to_string()));
            label.add_css_class("line-number");
            label.set_xalign(1.0);
            self.line_numbers.append(&label);
        }
    }
}
