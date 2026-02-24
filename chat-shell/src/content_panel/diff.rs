//! Diff renderer — side-by-side diff with color coding.

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, ScrolledWindow};

use similar::{ChangeTag, TextDiff};

/// Renders a unified diff view with color-coded additions and removals.
#[derive(Clone)]
pub struct DiffRenderer {
    container: gtk4::Box,
    stats_label: gtk4::Label,
    diff_content: gtk4::Box,
    _scroll: ScrolledWindow,
}

impl DiffRenderer {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("diff-renderer");
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Stats header.
        let stats_label = gtk4::Label::new(None);
        stats_label.add_css_class("diff-stats");
        stats_label.set_halign(Align::Start);
        container.append(&stats_label);

        // Scroll area for diff lines.
        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .build();

        let diff_content = gtk4::Box::new(Orientation::Vertical, 0);
        diff_content.set_valign(Align::Start);
        scroll.set_child(Some(&diff_content));

        container.append(&scroll);

        Self {
            container,
            stats_label,
            diff_content,
            _scroll: scroll,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Compute and display a diff between old and new content.
    pub fn set_diff(&self, old: &str, new: &str) {
        // Clear existing diff lines.
        while let Some(child) = self.diff_content.first_child() {
            self.diff_content.remove(&child);
        }

        let diff = TextDiff::from_lines(old, new);

        let mut additions = 0usize;
        let mut removals = 0usize;
        let mut old_line = 1u32;
        let mut new_line = 1u32;

        for change in diff.iter_all_changes() {
            let (prefix, css_class, line_num_text) = match change.tag() {
                ChangeTag::Insert => {
                    additions += 1;
                    let ln = format!("{:>4}", new_line);
                    new_line += 1;
                    ("+", "diff-line-add", ln)
                }
                ChangeTag::Delete => {
                    removals += 1;
                    let ln = format!("{:>4}", old_line);
                    old_line += 1;
                    ("-", "diff-line-remove", ln)
                }
                ChangeTag::Equal => {
                    let ln = format!("{:>4}", old_line);
                    old_line += 1;
                    new_line += 1;
                    (" ", "diff-line-context", ln)
                }
            };

            let row = gtk4::Box::new(Orientation::Horizontal, 0);
            row.add_css_class("diff-line");
            row.add_css_class(css_class);

            let line_num = gtk4::Label::new(Some(&line_num_text));
            line_num.add_css_class("diff-line-number");
            line_num.set_xalign(1.0);
            row.append(&line_num);

            let prefix_label = gtk4::Label::new(Some(prefix));
            prefix_label.add_css_class("diff-prefix");
            prefix_label.set_xalign(0.5);
            row.append(&prefix_label);

            let content_text = change.value().trim_end_matches('\n');
            let content_label = gtk4::Label::new(Some(content_text));
            content_label.set_xalign(0.0);
            content_label.set_hexpand(true);
            content_label.set_selectable(true);
            content_label.set_focusable(false);
            content_label.set_ellipsize(gtk4::pango::EllipsizeMode::None);
            row.append(&content_label);

            self.diff_content.append(&row);
        }

        self.stats_label.set_text(&format!(
            "+{} -{} lines changed",
            additions, removals
        ));
    }
}
