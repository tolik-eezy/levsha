//! Chat search overlay -- Ctrl+F find-in-chat with match navigation.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Orientation};

/// Search overlay state and widgets.
#[derive(Clone)]
pub struct SearchBar {
    /// The top-level container widget.
    container: gtk4::Box,
    /// The search text entry.
    entry: gtk4::Entry,
    /// Label showing "N of M" match count.
    count_label: gtk4::Label,
    /// Current match index (1-based for display).
    current_match: Rc<Cell<usize>>,
    /// Total match count.
    total_matches: Rc<Cell<usize>>,
    /// Callbacks.
    on_search: Rc<RefCell<Option<Box<dyn Fn(&str)>>>>,
    on_next: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    on_prev: Rc<RefCell<Option<Box<dyn Fn()>>>>,
    on_close: Rc<RefCell<Option<Box<dyn Fn()>>>>,
}

impl SearchBar {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Horizontal, 8);
        container.add_css_class("search-overlay");
        container.set_halign(Align::Fill);
        container.set_hexpand(true);
        container.set_visible(false);

        // Search entry.
        let entry = gtk4::Entry::new();
        entry.add_css_class("search-entry");
        entry.set_placeholder_text(Some("Search messages..."));
        entry.set_hexpand(true);

        // Match count label.
        let count_label = gtk4::Label::new(None);
        count_label.add_css_class("search-count");
        count_label.set_halign(Align::Center);

        // Navigation buttons.
        let prev_btn = gtk4::Button::with_label("\u{25B2}");
        prev_btn.add_css_class("search-nav-button");
        prev_btn.set_tooltip_text(Some("Previous match (Shift+Enter)"));

        let next_btn = gtk4::Button::with_label("\u{25BC}");
        next_btn.add_css_class("search-nav-button");
        next_btn.set_tooltip_text(Some("Next match (Enter)"));

        // Close button.
        let close_btn = gtk4::Button::with_label("\u{2715}");
        close_btn.add_css_class("search-nav-button");
        close_btn.set_tooltip_text(Some("Close (Escape)"));

        container.append(&entry);
        container.append(&count_label);
        container.append(&prev_btn);
        container.append(&next_btn);
        container.append(&close_btn);

        let on_search: Rc<RefCell<Option<Box<dyn Fn(&str)>>>> = Rc::new(RefCell::new(None));
        let on_next: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let on_prev: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));
        let on_close: Rc<RefCell<Option<Box<dyn Fn()>>>> = Rc::new(RefCell::new(None));

        // Wire up entry changed -> search callback.
        {
            let on_search = on_search.clone();
            entry.connect_changed(move |e| {
                let text = e.text();
                if let Some(cb) = on_search.borrow().as_ref() {
                    cb(text.as_str());
                }
            });
        }

        // Wire up Enter -> next match.
        {
            let on_next_for_activate = on_next.clone();
            entry.connect_activate(move |_| {
                if let Some(cb) = on_next_for_activate.borrow().as_ref() {
                    cb();
                }
            });

            // Use key controller for Shift+Enter (prev) and Escape (close).
            let key_ctrl = gtk4::EventControllerKey::new();
            let on_prev_for_key = on_prev.clone();
            let on_close_for_key = on_close.clone();
            key_ctrl.connect_key_pressed(move |_, key, _keycode, modifier| {
                if key == gdk4::Key::Escape {
                    if let Some(cb) = on_close_for_key.borrow().as_ref() {
                        cb();
                    }
                    return glib::Propagation::Stop;
                }

                if key == gdk4::Key::Return && modifier.contains(gdk4::ModifierType::SHIFT_MASK) {
                    if let Some(cb) = on_prev_for_key.borrow().as_ref() {
                        cb();
                    }
                    return glib::Propagation::Stop;
                }

                glib::Propagation::Proceed
            });
            entry.add_controller(key_ctrl);
        }

        // Button clicks.
        {
            let on_prev = on_prev.clone();
            prev_btn.connect_clicked(move |_| {
                if let Some(cb) = on_prev.borrow().as_ref() {
                    cb();
                }
            });
        }
        {
            let on_next = on_next.clone();
            next_btn.connect_clicked(move |_| {
                if let Some(cb) = on_next.borrow().as_ref() {
                    cb();
                }
            });
        }
        {
            let on_close = on_close.clone();
            close_btn.connect_clicked(move |_| {
                if let Some(cb) = on_close.borrow().as_ref() {
                    cb();
                }
            });
        }

        Self {
            container,
            entry,
            count_label,
            current_match: Rc::new(Cell::new(0)),
            total_matches: Rc::new(Cell::new(0)),
            on_search,
            on_next,
            on_prev,
            on_close,
        }
    }

    /// The top-level widget to insert into the UI.
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Show the search bar and focus the entry.
    pub fn show(&self) {
        self.container.set_visible(true);
        self.entry.grab_focus();
    }

    /// Hide the search bar and clear it.
    pub fn hide(&self) {
        self.container.set_visible(false);
        self.entry.set_text("");
        self.count_label.set_text("");
        self.current_match.set(0);
        self.total_matches.set(0);
    }

    /// Whether the search bar is currently visible.
    pub fn is_visible(&self) -> bool {
        self.container.is_visible()
    }

    /// Get the current search query.
    pub fn query(&self) -> String {
        self.entry.text().to_string()
    }

    /// Update the match count display.
    pub fn set_match_count(&self, current: usize, total: usize) {
        self.current_match.set(current);
        self.total_matches.set(total);
        if total == 0 {
            if self.entry.text().is_empty() {
                self.count_label.set_text("");
            } else {
                self.count_label.set_text("No matches");
            }
        } else {
            self.count_label
                .set_text(&format!("{} of {}", current, total));
        }
    }

    /// Set callback for when the search query changes.
    pub fn connect_search<F: Fn(&str) + 'static>(&self, f: F) {
        *self.on_search.borrow_mut() = Some(Box::new(f));
    }

    /// Set callback for "next match" navigation.
    pub fn connect_next<F: Fn() + 'static>(&self, f: F) {
        *self.on_next.borrow_mut() = Some(Box::new(f));
    }

    /// Set callback for "previous match" navigation.
    pub fn connect_prev<F: Fn() + 'static>(&self, f: F) {
        *self.on_prev.borrow_mut() = Some(Box::new(f));
    }

    /// Set callback for closing the search.
    pub fn connect_close<F: Fn() + 'static>(&self, f: F) {
        *self.on_close.borrow_mut() = Some(Box::new(f));
    }
}
