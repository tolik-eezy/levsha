//! Session sidebar -- persistent left panel for session navigation (Module 26).

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, SelectionMode};
use tokio::sync::mpsc;

use levsha_engine::types::{SessionInfo, ShellToEngine};

use crate::session_list::format_time_ago;

struct Inner {
    revealer: gtk4::Revealer,
    toggle_btn: gtk4::Button,
    list_box: gtk4::ListBox,
    shutdown_revealer: gtk4::Revealer,
    shell_tx: mpsc::Sender<ShellToEngine>,
    visible: bool,
    user_toggled: bool,
}

/// A persistent sidebar for session navigation.
#[derive(Clone)]
pub struct SessionSidebar {
    inner: Rc<RefCell<Inner>>,
}

impl SessionSidebar {
    pub fn new(shell_tx: mpsc::Sender<ShellToEngine>) -> Self {
        // Sidebar box: vertical container, 260px wide.
        let sidebar_box = gtk4::Box::new(Orientation::Vertical, 0);
        sidebar_box.add_css_class("sidebar-container");
        sidebar_box.set_width_request(260);

        // Header row.
        let header_row = gtk4::Box::new(Orientation::Horizontal, 0);
        header_row.add_css_class("sidebar-header");
        header_row.set_hexpand(true);

        let title_label = gtk4::Label::new(Some("Sessions"));
        title_label.add_css_class("sidebar-title");
        title_label.set_halign(Align::Start);
        title_label.set_hexpand(true);

        let close_btn = gtk4::Button::from_icon_name("window-close-symbolic");
        close_btn.add_css_class("sidebar-close-btn");

        header_row.append(&title_label);
        header_row.append(&close_btn);
        sidebar_box.append(&header_row);

        // Scrollable session list.
        let scroll = gtk4::ScrolledWindow::new();
        scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
        scroll.set_vexpand(true);
        scroll.add_css_class("sidebar-scroll");

        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(SelectionMode::None);
        list_box.add_css_class("sidebar-list");
        scroll.set_child(Some(&list_box));
        sidebar_box.append(&scroll);

        // New session button.
        let new_session_btn = gtk4::Button::with_label("+ New Session");
        new_session_btn.add_css_class("sidebar-new-session-btn");
        sidebar_box.append(&new_session_btn);

        // Separator.
        let separator = gtk4::Separator::new(Orientation::Horizontal);
        separator.add_css_class("sidebar-separator");
        sidebar_box.append(&separator);

        // Bottom zone.
        let bottom_zone = gtk4::Box::new(Orientation::Vertical, 0);
        bottom_zone.add_css_class("sidebar-bottom-zone");

        // Settings row.
        let settings_row = gtk4::Box::new(Orientation::Horizontal, 8);
        settings_row.add_css_class("sidebar-action-row");
        let settings_icon = gtk4::Image::from_icon_name("emblem-system-symbolic");
        settings_icon.add_css_class("sidebar-action-icon");
        let settings_label = gtk4::Label::new(Some("Settings"));
        settings_label.add_css_class("sidebar-action-label");
        settings_row.append(&settings_icon);
        settings_row.append(&settings_label);
        bottom_zone.append(&settings_row);

        // Power row.
        let power_row = gtk4::Box::new(Orientation::Horizontal, 8);
        power_row.add_css_class("sidebar-action-row");
        power_row.add_css_class("sidebar-power-row");
        let power_icon = gtk4::Image::from_icon_name("system-shutdown-symbolic");
        power_icon.add_css_class("sidebar-action-icon");
        let power_label = gtk4::Label::new(Some("Shut Down"));
        power_label.add_css_class("sidebar-action-label");
        power_row.append(&power_icon);
        power_row.append(&power_label);
        bottom_zone.append(&power_row);

        // Shutdown confirmation revealer.
        let shutdown_revealer = gtk4::Revealer::new();
        shutdown_revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        shutdown_revealer.set_transition_duration(200);
        shutdown_revealer.set_reveal_child(false);

        let shutdown_confirm = gtk4::Box::new(Orientation::Horizontal, 8);
        shutdown_confirm.add_css_class("sidebar-shutdown-confirm");

        let cancel_btn = gtk4::Button::with_label("Cancel");
        cancel_btn.add_css_class("sidebar-cancel-btn");
        cancel_btn.set_hexpand(true);

        let shutdown_btn = gtk4::Button::with_label("Shut Down");
        shutdown_btn.add_css_class("sidebar-shutdown-btn");
        shutdown_btn.set_hexpand(true);

        shutdown_confirm.append(&cancel_btn);
        shutdown_confirm.append(&shutdown_btn);
        shutdown_revealer.set_child(Some(&shutdown_confirm));
        bottom_zone.append(&shutdown_revealer);

        sidebar_box.append(&bottom_zone);

        // Revealer wrapping the sidebar box (slide animation).
        let revealer = gtk4::Revealer::new();
        revealer.set_transition_type(gtk4::RevealerTransitionType::SlideRight);
        revealer.set_transition_duration(250);
        revealer.set_child(Some(&sidebar_box));
        revealer.set_reveal_child(false);
        revealer.set_visible(false); // fully remove from layout when hidden

        // Toggle button (hamburger) -- placed as overlay by window.rs.
        let toggle_btn = gtk4::Button::from_icon_name("open-menu-symbolic");
        toggle_btn.add_css_class("sidebar-toggle-btn");
        toggle_btn.set_visible(true); // visible initially (sidebar starts hidden, so toggle shows)

        // -- Non-sidebar signals (don't need inner ref) --

        // New session button.
        let tx_new = shell_tx.clone();
        new_session_btn.connect_clicked(move |_| {
            let tx = tx_new.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(ShellToEngine::SessionCreate { name: None }).await;
            });
        });

        // Power row click -> reveal shutdown confirmation.
        let shutdown_revealer_ref = shutdown_revealer.clone();
        let power_click = gtk4::GestureClick::new();
        power_click.connect_released(move |gesture, _, _, _| {
            shutdown_revealer_ref.set_reveal_child(true);
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });
        power_row.add_controller(power_click);

        // Cancel button collapses shutdown confirmation.
        let shutdown_revealer_cancel = shutdown_revealer.clone();
        cancel_btn.connect_clicked(move |_| {
            shutdown_revealer_cancel.set_reveal_child(false);
        });

        // Shutdown button runs systemctl poweroff.
        shutdown_btn.connect_clicked(move |_| {
            let _ = std::process::Command::new("systemctl")
                .arg("poweroff")
                .spawn();
        });

        let inner = Rc::new(RefCell::new(Inner {
            revealer,
            toggle_btn,
            list_box,
            shutdown_revealer,
            shell_tx,
            visible: false,
            user_toggled: false,
        }));

        let sidebar = Self { inner };

        // Connect close button: hides sidebar, marks user-toggled, persists preference.
        let sidebar_close = sidebar.clone();
        close_btn.connect_clicked(move |_| {
            sidebar_close.hide();
            sidebar_close.set_user_toggled(true);
            sidebar_close.send_toggle(false);
        });

        // Connect toggle button: shows sidebar, marks user-toggled, persists preference.
        let sidebar_toggle = sidebar.clone();
        sidebar.toggle_button().connect_clicked(move |_| {
            sidebar_toggle.show();
            sidebar_toggle.set_user_toggled(true);
            sidebar_toggle.send_toggle(true);
        });

        sidebar
    }

    /// Returns the revealer widget for embedding in the window layout.
    pub fn widget(&self) -> gtk4::Revealer {
        self.inner.borrow().revealer.clone()
    }

    /// Returns the toggle button for overlay placement.
    pub fn toggle_button(&self) -> gtk4::Button {
        self.inner.borrow().toggle_btn.clone()
    }

    /// Show the sidebar.
    pub fn show(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.revealer.set_visible(true); // re-add to layout
        inner.revealer.set_reveal_child(true);
        inner.toggle_btn.set_visible(false);
        inner.visible = true;
    }

    /// Hide the sidebar.
    pub fn hide(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.revealer.set_reveal_child(false);
        inner.toggle_btn.set_visible(true);
        inner.visible = false;
        // Collapse shutdown confirmation when hiding.
        inner.shutdown_revealer.set_reveal_child(false);
        // After the slide animation (250ms), fully remove from layout
        // so it doesn't occupy horizontal space.
        let inner_ref = self.inner.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(260), move || {
            let inner = inner_ref.borrow();
            // Guard: only hide if sidebar is still supposed to be hidden
            // (handles rapid show/hide toggling).
            if !inner.visible {
                inner.revealer.set_visible(false);
            }
        });
    }

    /// Toggle sidebar visibility.
    pub fn toggle(&self) {
        if self.is_visible() {
            self.hide();
        } else {
            self.show();
        }
    }

    /// Returns whether the sidebar is currently visible.
    pub fn is_visible(&self) -> bool {
        self.inner.borrow().visible
    }

    /// Mark that the user manually toggled the sidebar.
    pub fn set_user_toggled(&self, val: bool) {
        self.inner.borrow_mut().user_toggled = val;
    }

    /// Check if the user has manually toggled the sidebar.
    pub fn user_toggled(&self) -> bool {
        self.inner.borrow().user_toggled
    }

    /// Send SidebarToggle to the engine to persist the visibility preference.
    pub fn send_toggle(&self, visible: bool) {
        let tx = self.inner.borrow().shell_tx.clone();
        glib::spawn_future_local(async move {
            let _ = tx.send(ShellToEngine::SidebarToggle { visible }).await;
        });
    }

    /// Refresh the session list with new data.
    pub fn update_sessions(&self, sessions: &[SessionInfo]) {
        let inner = self.inner.borrow();

        // Clear existing rows.
        while let Some(child) = inner.list_box.first_child() {
            inner.list_box.remove(&child);
        }

        // Add rows for each session.
        for session in sessions {
            let row = build_sidebar_row(session, &inner.shell_tx);
            inner.list_box.append(&row);
        }
    }
}

/// Build a single sidebar session row.
fn build_sidebar_row(
    session: &SessionInfo,
    shell_tx: &mpsc::Sender<ShellToEngine>,
) -> gtk4::Box {
    let row = gtk4::Box::new(Orientation::Vertical, 2);
    row.add_css_class("sidebar-row");
    if session.is_active {
        row.add_css_class("sidebar-row-active");
    }

    let name_label = gtk4::Label::new(Some(&session.name));
    name_label.add_css_class("sidebar-session-name");
    if session.is_active {
        name_label.add_css_class("sidebar-session-name-active");
    }
    name_label.set_halign(Align::Start);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_label.set_max_width_chars(30);

    let time_label = gtk4::Label::new(Some(&format_time_ago(session.updated_at)));
    time_label.add_css_class("sidebar-session-time");
    time_label.set_halign(Align::Start);

    row.append(&name_label);
    row.append(&time_label);

    // Click to switch session.
    let click = gtk4::GestureClick::new();
    let session_id = session.id.clone();
    let tx = shell_tx.clone();
    click.connect_released(move |gesture, _, _, _| {
        let tx = tx.clone();
        let sid = session_id.clone();
        glib::spawn_future_local(async move {
            let _ = tx
                .send(ShellToEngine::SessionSwitch { session_id: sid })
                .await;
        });
        gesture.set_state(gtk4::EventSequenceState::Claimed);
    });
    row.add_controller(click);

    row
}
