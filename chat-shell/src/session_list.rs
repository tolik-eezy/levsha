//! Session list overlay — modal overlay showing all sessions.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, SelectionMode};
use tokio::sync::mpsc;

use levsha_engine::types::{SessionInfo, ShellToEngine};

/// Internal state shared across callbacks.
struct Inner {
    overlay_box: gtk4::Box,
    revealer: gtk4::Revealer,
    list_box: gtk4::ListBox,
    shell_tx: mpsc::Sender<ShellToEngine>,
}

/// A modal overlay that displays the list of sessions.
#[derive(Clone)]
pub struct SessionListOverlay {
    inner: Rc<RefCell<Inner>>,
}

impl SessionListOverlay {
    pub fn new(shell_tx: mpsc::Sender<ShellToEngine>) -> Self {
        // Backdrop: semi-transparent overlay container.
        let overlay_box = gtk4::Box::new(Orientation::Vertical, 0);
        overlay_box.add_css_class("session-overlay-backdrop");
        overlay_box.set_halign(Align::Fill);
        overlay_box.set_valign(Align::Fill);
        overlay_box.set_hexpand(true);
        overlay_box.set_vexpand(true);

        // Card: centered container.
        let card = gtk4::Box::new(Orientation::Vertical, 0);
        card.add_css_class("session-overlay-card");
        card.set_halign(Align::Center);
        card.set_valign(Align::Center);

        // Header: title + new button.
        let header = gtk4::Box::new(Orientation::Horizontal, 0);
        header.add_css_class("session-overlay-header");
        header.set_hexpand(true);

        let title = gtk4::Label::new(Some("Sessions"));
        title.add_css_class("session-overlay-title");
        title.set_halign(Align::Start);
        title.set_hexpand(true);

        let new_btn = gtk4::Button::with_label("+ New");
        new_btn.add_css_class("session-overlay-new-btn");

        header.append(&title);
        header.append(&new_btn);

        // Scrollable list.
        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_max_content_height(400);
        scroll.set_propagate_natural_height(true);
        scroll.set_vexpand(true);

        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(SelectionMode::None);
        list_box.add_css_class("session-list");
        scroll.set_child(Some(&list_box));

        card.append(&header);
        card.append(&scroll);

        // Revealer for animation.
        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::Crossfade);
        revealer.set_transition_duration(200);
        revealer.set_child(Some(&card));
        revealer.set_reveal_child(false);
        revealer.set_halign(Align::Center);
        revealer.set_valign(Align::Center);

        overlay_box.append(&revealer);
        overlay_box.set_visible(false);

        // Close on backdrop click (click outside the card).
        // We add the gesture to overlay_box; clicks that land on the card's children
        // will be captured by those children's own controllers, so only backdrop clicks
        // propagate here.
        let backdrop_click = gtk4::GestureClick::new();
        let card_ref = card.clone();
        let revealer_ref = revealer.clone();
        let overlay_box_ref = overlay_box.clone();
        backdrop_click.connect_released(move |gesture, _, x, y| {
            // Check if the click lands inside the card by computing the card's
            // position relative to the overlay_box.
            if let Some(bounds) = card_ref.compute_bounds(&overlay_box_ref) {
                let cx = bounds.x() as f64;
                let cy = bounds.y() as f64;
                let cw = bounds.width() as f64;
                let ch = bounds.height() as f64;
                if x >= cx && x <= cx + cw && y >= cy && y <= cy + ch {
                    // Click is inside the card — don't close.
                    return;
                }
            }
            // Click was on the backdrop — close the overlay.
            revealer_ref.set_reveal_child(false);
            let ob = overlay_box_ref.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
                ob.set_visible(false);
            });
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });
        overlay_box.add_controller(backdrop_click);

        // New session button click.
        let tx_new = shell_tx.clone();
        let revealer_new = revealer.clone();
        let overlay_box_new = overlay_box.clone();
        new_btn.connect_clicked(move |_| {
            let tx = tx_new.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(ShellToEngine::SessionCreate { name: None }).await;
            });
            revealer_new.set_reveal_child(false);
            let ob = overlay_box_new.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
                ob.set_visible(false);
            });
        });

        let inner = Rc::new(RefCell::new(Inner {
            overlay_box,
            revealer,
            list_box,
            shell_tx,
        }));

        Self { inner }
    }

    /// Show the overlay with the given sessions.
    pub fn show(&self, sessions: &[SessionInfo]) {
        let inner = self.inner.borrow();

        // Clear existing rows.
        while let Some(child) = inner.list_box.first_child() {
            inner.list_box.remove(&child);
        }

        // Add rows for each session.
        for session in sessions {
            let row = build_session_row(session, &inner.shell_tx, self.clone());
            inner.list_box.append(&row);
        }

        inner.overlay_box.set_visible(true);
        inner.revealer.set_reveal_child(true);
    }

    /// Hide the overlay.
    pub fn hide(&self) {
        let inner = self.inner.borrow();
        inner.revealer.set_reveal_child(false);
        let ob = inner.overlay_box.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
            ob.set_visible(false);
        });
    }

    /// Returns whether the overlay is currently visible.
    pub fn is_visible(&self) -> bool {
        self.inner.borrow().overlay_box.is_visible()
    }

    /// Returns the top-level widget for embedding in the window overlay.
    pub fn widget(&self) -> gtk4::Box {
        self.inner.borrow().overlay_box.clone()
    }
}

/// Build a single session row widget.
fn build_session_row(
    session: &SessionInfo,
    shell_tx: &mpsc::Sender<ShellToEngine>,
    overlay: SessionListOverlay,
) -> gtk4::Box {
    let row = gtk4::Box::new(Orientation::Horizontal, 12);
    row.add_css_class("session-row");
    if session.is_active {
        row.add_css_class("session-row-active");
    }

    // Active indicator.
    let indicator = if session.is_active {
        let l = gtk4::Label::new(Some("\u{25CF}")); // filled circle
        l.add_css_class("session-indicator-active");
        l
    } else {
        let l = gtk4::Label::new(Some("\u{25CB}")); // empty circle
        l.add_css_class("session-indicator-inactive");
        l
    };
    indicator.set_valign(Align::Center);

    // Info column: name + preview.
    let info_box = gtk4::Box::new(Orientation::Vertical, 2);
    info_box.set_hexpand(true);

    let name_label = gtk4::Label::new(Some(&session.name));
    name_label.add_css_class("session-name");
    if session.is_active {
        name_label.add_css_class("session-name-active");
    }
    name_label.set_halign(Align::Start);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_label.set_max_width_chars(40);

    info_box.append(&name_label);

    if let Some(ref preview) = session.last_message_preview {
        let preview_label = gtk4::Label::new(Some(preview));
        preview_label.add_css_class("session-preview");
        preview_label.set_halign(Align::Start);
        preview_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        preview_label.set_max_width_chars(50);
        info_box.append(&preview_label);
    }

    // Time ago.
    let time_label = gtk4::Label::new(Some(&format_time_ago(session.updated_at)));
    time_label.add_css_class("session-time");
    time_label.set_valign(Align::Center);

    row.append(&indicator);
    row.append(&info_box);
    row.append(&time_label);

    // Click to switch session.
    let click = gtk4::GestureClick::new();
    let session_id = session.id.clone();
    let tx = shell_tx.clone();
    click.connect_released(move |gesture, _, _, _| {
        let tx = tx.clone();
        let sid = session_id.clone();
        let ov = overlay.clone();
        glib::spawn_future_local(async move {
            let _ = tx
                .send(ShellToEngine::SessionSwitch { session_id: sid })
                .await;
        });
        ov.hide();
        gesture.set_state(gtk4::EventSequenceState::Claimed);
    });
    row.add_controller(click);

    row
}

/// Format a unix timestamp into a human-readable "time ago" string.
pub fn format_time_ago(timestamp: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = now - timestamp;

    if diff < 0 {
        return "just now".to_string();
    }

    let seconds = diff;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    let days = hours / 24;

    if seconds < 60 {
        "just now".to_string()
    } else if minutes < 60 {
        if minutes == 1 {
            "1 min ago".to_string()
        } else {
            format!("{} min ago", minutes)
        }
    } else if hours < 24 {
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if days == 1 {
        "yesterday".to_string()
    } else {
        format!("{} days ago", days)
    }
}
