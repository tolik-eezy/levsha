//! Progress renderer — operation progress with determinate/indeterminate bar.

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, ScrolledWindow};

/// Renders a progress view for long-running operations.
#[derive(Clone)]
pub struct ProgressRenderer {
    container: gtk4::Box,
    operation_label: gtk4::Label,
    progress_bar: gtk4::ProgressBar,
    message_label: gtk4::Label,
    log_view: gtk4::TextView,
    log_scroll: ScrolledWindow,
    total: std::cell::Cell<Option<u64>>,
}

impl ProgressRenderer {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("progress-renderer");
        container.set_hexpand(true);
        container.set_vexpand(true);
        container.set_valign(Align::Fill);

        let operation_label = gtk4::Label::new(None);
        operation_label.add_css_class("progress-operation");
        operation_label.set_halign(Align::Start);
        container.append(&operation_label);

        let progress_bar = gtk4::ProgressBar::new();
        progress_bar.add_css_class("progress-bar");
        progress_bar.set_hexpand(true);
        container.append(&progress_bar);

        let message_label = gtk4::Label::new(None);
        message_label.add_css_class("progress-message");
        message_label.set_halign(Align::Start);
        message_label.set_wrap(true);
        container.append(&message_label);

        let log_view = gtk4::TextView::new();
        log_view.set_editable(false);
        log_view.set_cursor_visible(false);
        log_view.set_wrap_mode(gtk4::WrapMode::WordChar);
        log_view.add_css_class("progress-log");

        let log_scroll = ScrolledWindow::new();
        log_scroll.set_vexpand(true);
        log_scroll.set_hscrollbar_policy(PolicyType::Automatic);
        log_scroll.set_vscrollbar_policy(PolicyType::Automatic);
        log_scroll.set_child(Some(&log_view));
        container.append(&log_scroll);

        Self {
            container,
            operation_label,
            progress_bar,
            message_label,
            log_view,
            log_scroll,
            total: std::cell::Cell::new(None),
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Start a new progress operation.
    pub fn start(&self, operation: &str, total: Option<u64>) {
        self.operation_label.set_text(operation);
        self.total.set(total);
        self.message_label.set_text("");

        // Clear the log buffer for the new operation.
        let buf = self.log_view.buffer();
        buf.set_text("");

        if total.is_some() {
            self.progress_bar.set_fraction(0.0);
        } else {
            self.progress_bar.pulse();
        }
    }

    /// Append text to the log view and auto-scroll to the bottom.
    pub fn append_log(&self, text: &str) {
        let buf = self.log_view.buffer();
        let mut end_iter = buf.end_iter();
        buf.insert(&mut end_iter, text);

        // Auto-scroll to bottom.
        let end_mark = buf.create_mark(None, &buf.end_iter(), false);
        self.log_view.scroll_mark_onscreen(&end_mark);
        buf.delete_mark(&end_mark);
    }

    /// Update progress.
    pub fn update(&self, current: u64, message: &str) {
        self.message_label.set_text(message);

        if let Some(total) = self.total.get() {
            if total > 0 {
                let fraction = (current as f64) / (total as f64);
                self.progress_bar.set_fraction(fraction.clamp(0.0, 1.0));
            }
        } else {
            self.progress_bar.pulse();
        }
    }
}
