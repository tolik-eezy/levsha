//! Chat view — scrollable message history with auto-scroll and search.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glib::clone;
use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, ScrolledWindow};

use levsha_engine::types::{MessageRole, ToolExecutionStatus};

use crate::message_widget;
use crate::search::SearchBar;

/// The chat view containing the scrollable message list.
#[derive(Clone)]
pub struct ChatView {
    scroll: ScrolledWindow,
    message_list: gtk4::Box,
    /// The currently-streaming message label (if any).
    streaming_label: Rc<RefCell<Option<gtk4::Label>>>,
    /// Raw text buffer for the streaming message.
    streaming_buffer: Rc<RefCell<String>>,
    /// Whether auto-scroll is active (user hasn't scrolled up).
    auto_scroll: Rc<Cell<bool>>,
    /// Typing indicator widget reference.
    typing_indicator: Rc<RefCell<Option<gtk4::Box>>>,
    /// Scroll-to-bottom overlay button.
    scroll_to_bottom_btn: gtk4::Button,
    /// The outer overlay that contains the scroll view + button.
    overlay: gtk4::Overlay,
    /// The outermost container (search bar + overlay).
    outer_box: gtk4::Box,
    /// The search bar.
    search_bar: SearchBar,
    /// Indices of message-row children that match the current query.
    match_indices: Rc<RefCell<Vec<i32>>>,
    /// Current match position (0-based index into match_indices).
    current_match_pos: Rc<Cell<usize>>,
}

impl ChatView {
    pub fn new() -> Self {
        let message_list = gtk4::Box::new(Orientation::Vertical, 0);
        message_list.add_css_class("message-list");
        message_list.set_valign(Align::End);
        message_list.set_hexpand(true);

        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Never)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .kinetic_scrolling(true)
            .build();
        scroll.add_css_class("chat-scroll");
        scroll.set_child(Some(&message_list));

        // Scroll-to-bottom button.
        let scroll_to_bottom_btn = gtk4::Button::with_label("\u{2193} New messages");
        scroll_to_bottom_btn.add_css_class("scroll-to-bottom");
        scroll_to_bottom_btn.set_halign(Align::Center);
        scroll_to_bottom_btn.set_valign(Align::End);
        scroll_to_bottom_btn.set_visible(false);

        // Overlay: scroll + button overlaid.
        let overlay = gtk4::Overlay::new();
        overlay.set_child(Some(&scroll));
        overlay.add_overlay(&scroll_to_bottom_btn);
        overlay.set_vexpand(true);

        // Search bar (hidden by default).
        let search_bar = SearchBar::new();

        // Outer box: search bar on top, overlay below.
        let outer_box = gtk4::Box::new(Orientation::Vertical, 0);
        outer_box.append(search_bar.widget());
        outer_box.append(&overlay);
        outer_box.set_vexpand(true);

        let auto_scroll = Rc::new(Cell::new(true));

        // Track scroll position to detect user scrolling up.
        let vadj = scroll.vadjustment();
        {
            let auto_scroll = auto_scroll.clone();
            let btn = scroll_to_bottom_btn.clone();
            vadj.connect_value_changed(move |adj| {
                let at_bottom = adj.value() >= adj.upper() - adj.page_size() - 50.0;
                auto_scroll.set(at_bottom);
                btn.set_visible(!at_bottom);
            });
        }

        // Scroll-to-bottom button action.
        {
            let scroll = scroll.clone();
            let auto_scroll = auto_scroll.clone();
            scroll_to_bottom_btn.connect_clicked(move |btn| {
                scroll_to_end(&scroll);
                auto_scroll.set(true);
                btn.set_visible(false);
            });
        }

        let match_indices: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(Vec::new()));
        let current_match_pos = Rc::new(Cell::new(0usize));

        let view = Self {
            scroll,
            message_list,
            streaming_label: Rc::new(RefCell::new(None)),
            streaming_buffer: Rc::new(RefCell::new(String::new())),
            auto_scroll,
            typing_indicator: Rc::new(RefCell::new(None)),
            scroll_to_bottom_btn,
            overlay,
            outer_box,
            search_bar,
            match_indices,
            current_match_pos,
        };

        // Wire search callbacks.
        view.setup_search();

        view
    }

    /// Returns the top-level widget (search bar + overlay containing scroll + button).
    pub fn widget(&self) -> &gtk4::Box {
        &self.outer_box
    }

    /// Returns a reference to the scroll window (for keybinding scroll).
    pub fn scroll_window(&self) -> &ScrolledWindow {
        &self.scroll
    }

    /// Toggle the search bar visibility.
    pub fn toggle_search(&self) {
        if self.search_bar.is_visible() {
            self.hide_search();
        } else {
            self.show_search();
        }
    }

    /// Show the search overlay.
    pub fn show_search(&self) {
        self.search_bar.show();
    }

    /// Hide the search overlay and clear highlights.
    pub fn hide_search(&self) {
        self.search_bar.hide();
        self.match_indices.borrow_mut().clear();
        self.current_match_pos.set(0);
        self.clear_search_highlights();
    }

    /// Add a complete message to the chat.
    pub fn add_message(&self, role: MessageRole, content: &str, animate: bool) {
        let widget = message_widget::create_message_widget(role, content);
        if animate {
            widget.add_css_class("message-appear");
        }
        self.message_list.append(&widget);
        self.maybe_auto_scroll();
    }

    /// Add an arbitrary widget (error, confirm) to the chat.
    pub fn add_widget(&self, widget: impl IsA<gtk4::Widget>) {
        widget.add_css_class("message-appear");
        self.message_list.append(&widget);
        self.maybe_auto_scroll();
    }

    /// Show the typing indicator (three pulsing dots).
    pub fn show_typing_indicator(&self) {
        if self.typing_indicator.borrow().is_some() {
            return;
        }
        let indicator = create_typing_indicator();
        self.message_list.append(&indicator);
        *self.typing_indicator.borrow_mut() = Some(indicator);
        self.maybe_auto_scroll();
    }

    /// Remove the typing indicator.
    pub fn remove_typing_indicator(&self) {
        if let Some(indicator) = self.typing_indicator.borrow_mut().take() {
            self.message_list.remove(&indicator);
        }
    }

    /// Begin a new streaming message — creates the assistant bubble.
    pub fn begin_streaming_message(&self) {
        let (row, label) = message_widget::create_streaming_message_widget();
        self.message_list.append(&row);
        row.add_css_class("message-appear");
        *self.streaming_label.borrow_mut() = Some(label);
        *self.streaming_buffer.borrow_mut() = String::new();
        self.maybe_auto_scroll();
    }

    /// Append a text chunk to the current streaming message.
    pub fn append_streaming_chunk(&self, chunk: &str) {
        let mut buffer = self.streaming_buffer.borrow_mut();
        buffer.push_str(chunk);
        if let Some(label) = self.streaming_label.borrow().as_ref() {
            label.set_markup(&message_widget::markdown_to_pango(&buffer));
        }
        drop(buffer);
        self.maybe_auto_scroll();
    }

    /// Finalize the streaming message.
    pub fn finalize_streaming_message(&self) {
        if let Some(label) = self.streaming_label.borrow_mut().take() {
            let buffer = self.streaming_buffer.borrow();
            if !buffer.is_empty() {
                label.set_markup(&message_widget::markdown_to_pango(&buffer));
            }
        }
        *self.streaming_buffer.borrow_mut() = String::new();
    }

    /// Add a tool status inline.
    pub fn add_tool_status(
        &self,
        tool_name: &str,
        status: &ToolExecutionStatus,
        description: &str,
    ) {
        let widget = create_tool_status_widget(tool_name, status, description);
        self.message_list.append(&widget);
        self.maybe_auto_scroll();
    }

    /// Clear all visible messages.
    pub fn clear(&self) {
        while let Some(child) = self.message_list.first_child() {
            self.message_list.remove(&child);
        }
    }

    /// Auto-scroll to bottom if the user hasn't scrolled up.
    fn maybe_auto_scroll(&self) {
        if self.auto_scroll.get() {
            let scroll = self.scroll.clone();
            glib::idle_add_local_once(move || {
                scroll_to_end(&scroll);
            });
        }
    }

    // ── Search Implementation ──────────────────────────────────────

    /// Wire up the search bar callbacks to this chat view.
    fn setup_search(&self) {
        let view = self.clone();
        self.search_bar.connect_search(clone!(
            #[strong]
            view,
            move |query| {
                view.perform_search(query);
            }
        ));

        let view = self.clone();
        self.search_bar.connect_next(clone!(
            #[strong]
            view,
            move || {
                view.navigate_match(1);
            }
        ));

        let view = self.clone();
        self.search_bar.connect_prev(clone!(
            #[strong]
            view,
            move || {
                view.navigate_match(-1);
            }
        ));

        let view = self.clone();
        self.search_bar.connect_close(clone!(
            #[strong]
            view,
            move || {
                view.hide_search();
            }
        ));
    }

    /// Search through all message children for the query.
    fn perform_search(&self, query: &str) {
        self.clear_search_highlights();

        if query.is_empty() {
            self.match_indices.borrow_mut().clear();
            self.current_match_pos.set(0);
            self.search_bar.set_match_count(0, 0);
            return;
        }

        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();
        let mut child_index = 0i32;
        let mut child_opt = self.message_list.first_child();

        while let Some(child) = child_opt {
            if child.has_css_class("message-row") {
                let text = collect_widget_text(&child);
                if text.to_lowercase().contains(&query_lower) {
                    matches.push(child_index);
                    child.add_css_class("search-match-highlight");
                }
            }
            child_opt = child.next_sibling();
            child_index += 1;
        }

        let total = matches.len();
        *self.match_indices.borrow_mut() = matches;

        if total > 0 {
            self.current_match_pos.set(0);
            self.search_bar.set_match_count(1, total);
            self.scroll_to_match(0);
        } else {
            self.current_match_pos.set(0);
            self.search_bar.set_match_count(0, 0);
        }
    }

    /// Navigate to the next (+1) or previous (-1) match.
    fn navigate_match(&self, direction: i32) {
        let matches = self.match_indices.borrow();
        let total = matches.len();
        if total == 0 {
            return;
        }

        let current = self.current_match_pos.get() as i32;
        let next = ((current + direction) % total as i32 + total as i32) % total as i32;
        self.current_match_pos.set(next as usize);
        self.search_bar.set_match_count(next as usize + 1, total);
        drop(matches);
        self.scroll_to_match(next as usize);
    }

    /// Scroll to bring the Nth match into view.
    fn scroll_to_match(&self, match_pos: usize) {
        let matches = self.match_indices.borrow();
        if match_pos >= matches.len() {
            return;
        }
        let target_index = matches[match_pos];
        drop(matches);

        // Walk to the target child.
        let mut child_opt = self.message_list.first_child();
        let mut idx = 0i32;
        while let Some(child) = child_opt {
            if idx == target_index {
                // Scroll the widget into view using the scroll adjustment.
                let scroll = self.scroll.clone();
                glib::idle_add_local_once(move || {
                    if let Some((_, y)) =
                        child.translate_coordinates(&child.parent().unwrap(), 0.0, 0.0)
                    {
                        let adj = scroll.vadjustment();
                        let page = adj.page_size();
                        // Center the match in the viewport.
                        adj.set_value(y - page / 3.0);
                    }
                });
                break;
            }
            child_opt = child.next_sibling();
            idx += 1;
        }
    }

    /// Remove highlight CSS class from all children.
    fn clear_search_highlights(&self) {
        let mut child_opt = self.message_list.first_child();
        while let Some(child) = child_opt {
            child.remove_css_class("search-match-highlight");
            child_opt = child.next_sibling();
        }
    }
}

/// Recursively collect visible text from a widget tree.
fn collect_widget_text(widget: &gtk4::Widget) -> String {
    let mut text = String::new();

    // Try to get text from a Label.
    if let Some(label) = widget.downcast_ref::<gtk4::Label>() {
        let label_text = label.text();
        if !label_text.is_empty() {
            text.push_str(label_text.as_str());
            text.push(' ');
        }
    }

    // Recurse into children.
    let mut child_opt = widget.first_child();
    while let Some(child) = child_opt {
        text.push_str(&collect_widget_text(&child));
        child_opt = child.next_sibling();
    }

    text
}

/// Scroll a ScrolledWindow to the bottom.
fn scroll_to_end(scroll: &ScrolledWindow) {
    let adj = scroll.vadjustment();
    adj.set_value(adj.upper() - adj.page_size());
}

/// Create a typing indicator widget (three dots).
fn create_typing_indicator() -> gtk4::Box {
    let container = gtk4::Box::new(Orientation::Horizontal, 8);
    container.add_css_class("typing-indicator");
    container.add_css_class("message-row");
    container.set_halign(Align::Start);

    // Create a wrapper to match assistant bubble style.
    let bubble = gtk4::Box::new(Orientation::Horizontal, 6);
    bubble.add_css_class("message-bubble");
    bubble.add_css_class("message-bubble-assistant");

    for _ in 0..3 {
        let dot = gtk4::Label::new(Some("\u{2022}"));
        dot.add_css_class("typing-dot");
        bubble.append(&dot);
    }

    container.append(&bubble);
    container
}

/// Create a tool status widget.
fn create_tool_status_widget(
    tool_name: &str,
    status: &ToolExecutionStatus,
    description: &str,
) -> gtk4::Box {
    let row = gtk4::Box::new(Orientation::Horizontal, 8);
    row.add_css_class("tool-status");
    row.set_halign(Align::Start);
    row.set_margin_start(32);
    row.set_margin_bottom(4);

    let (icon, css_class) = match status {
        ToolExecutionStatus::Started => ("\u{25D0}", "tool-status-started"),
        ToolExecutionStatus::Completed { success } => {
            if *success {
                ("\u{2713}", "tool-status-success")
            } else {
                ("\u{2717}", "tool-status-failure")
            }
        }
    };

    let icon_label = gtk4::Label::new(Some(icon));
    icon_label.add_css_class("tool-status-icon");
    icon_label.add_css_class(css_class);

    let text = gtk4::Label::new(Some(&format!("{}: {}", tool_name, description)));
    text.add_css_class("tool-status");
    text.set_wrap(true);
    text.set_xalign(0.0);

    row.append(&icon_label);
    row.append(&text);
    row
}
