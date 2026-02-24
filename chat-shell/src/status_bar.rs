//! Status bar — displays model name, connection status, session name, and time.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Orientation};
use tokio::sync::mpsc;

use levsha_engine::types::ShellToEngine;

/// The status bar at the bottom of the screen.
#[derive(Clone)]
pub struct StatusBar {
    container: gtk4::Box,
    backend_label: gtk4::Label,
    connection_icon: gtk4::Label,
    connection_label: gtk4::Label,
    session_label: gtk4::Label,
    session_separator: gtk4::Label,
    time_label: gtk4::Label,
    state: Rc<RefCell<StatusState>>,
}

struct StatusState {
    connected: bool,
    backend: String,
}

impl StatusBar {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Horizontal, 0);
        container.add_css_class("status-bar");
        container.set_halign(Align::Center);
        container.set_hexpand(true);

        let state = Rc::new(RefCell::new(StatusState {
            connected: false,
            backend: "starting\u{2026}".to_string(),
        }));

        // Backend icon + label.
        let backend_icon = gtk4::Label::new(Some("\u{25C6}"));
        backend_icon.add_css_class("status-icon");
        let backend_label = gtk4::Label::new(Some("starting\u{2026}"));
        backend_label.set_halign(Align::Start);

        // Separator.
        let sep1 = gtk4::Label::new(Some(" \u{00B7} "));
        sep1.add_css_class("status-separator");

        // Connection icon + status.
        let connection_icon = gtk4::Label::new(Some("\u{25CB}"));
        connection_icon.add_css_class("status-icon");
        connection_icon.add_css_class("status-disconnected");
        let connection_label = gtk4::Label::new(Some("connecting"));
        connection_label.add_css_class("status-disconnected");
        connection_label.set_halign(Align::Start);

        // Separator before session label.
        let session_separator = gtk4::Label::new(Some(" \u{00B7} "));
        session_separator.add_css_class("status-separator");
        session_separator.set_visible(false);

        // Session label (clickable).
        let session_label = gtk4::Label::new(None);
        session_label.add_css_class("session-status-label");
        session_label.set_halign(Align::Start);
        session_label.set_visible(false);

        // Separator before time.
        let sep2 = gtk4::Label::new(Some(" \u{00B7} "));
        sep2.add_css_class("status-separator");

        // Time label.
        let time_label = gtk4::Label::new(Some("--:--"));
        time_label.set_halign(Align::Start);

        container.append(&backend_icon);
        container.append(&backend_label);
        container.append(&sep1);
        container.append(&connection_icon);
        container.append(&connection_label);
        container.append(&session_separator);
        container.append(&session_label);
        container.append(&sep2);
        container.append(&time_label);

        Self {
            container,
            backend_label,
            connection_icon,
            connection_label,
            session_label,
            session_separator,
            time_label,
            state,
        }
    }

    /// Returns the top-level widget.
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Update the connection status and backend name.
    pub fn set_connection_status(&self, connected: bool, backend: &str) {
        let mut st = self.state.borrow_mut();
        st.connected = connected;
        st.backend = backend.to_string();
        drop(st);

        self.backend_label.set_text(backend);

        if connected {
            self.connection_icon.set_text("\u{25CF}");
            self.connection_icon.remove_css_class("status-disconnected");
            self.connection_icon.add_css_class("status-connected");
            self.connection_label.set_text("connected");
            self.connection_label.remove_css_class("status-disconnected");
            self.connection_label.add_css_class("status-connected");
        } else {
            self.connection_icon.set_text("\u{25CB}");
            self.connection_icon.remove_css_class("status-connected");
            self.connection_icon.add_css_class("status-disconnected");
            self.connection_label.set_text("disconnected");
            self.connection_label.remove_css_class("status-connected");
            self.connection_label.add_css_class("status-disconnected");
        }
    }

    /// Update the connection status with additional detail text.
    pub fn set_connection_status_with_detail(&self, connected: bool, backend: &str, detail: &str) {
        self.set_connection_status(connected, backend);
        if !detail.is_empty() {
            let current = self.backend_label.text().to_string();
            self.backend_label
                .set_text(&format!("{} ({})", current, detail));
        }
    }

    /// Start a 1-second timer to update the clock.
    pub fn start_clock(&self) {
        // Update immediately.
        self.update_time();

        // Then every second.
        let time_label = self.time_label.clone();
        glib::timeout_add_seconds_local(1, move || {
            let now = chrono::Local::now();
            time_label.set_text(&now.format("%H:%M").to_string());
            glib::ControlFlow::Continue
        });
    }

    fn update_time(&self) {
        let now = chrono::Local::now();
        self.time_label.set_text(&now.format("%H:%M").to_string());
    }

    /// Update the session name displayed in the status bar.
    pub fn set_session_name(&self, name: Option<&str>) {
        match name {
            Some(n) => {
                self.session_label.set_text(n);
                self.session_label.set_visible(true);
                self.session_separator.set_visible(true);
            }
            None => {
                self.session_label.set_visible(false);
                self.session_separator.set_visible(false);
            }
        }
    }

    /// Update the backend status display.
    pub fn set_backend_status(&self, mode: &str, active: &str, _local: bool, _cloud: bool) {
        let display = if mode == "auto" {
            format!("{} (auto)", active)
        } else {
            active.to_string()
        };
        self.backend_label.set_text(&display);
    }

    /// Make the session label clickable — sends SessionList when clicked.
    pub fn connect_session_click(&self, shell_tx: mpsc::Sender<ShellToEngine>) {
        let click = gtk4::GestureClick::new();
        let tx = shell_tx;
        click.connect_released(move |gesture, _, _, _| {
            let tx = tx.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(ShellToEngine::SessionList).await;
            });
            gesture.set_state(gtk4::EventSequenceState::Claimed);
        });
        self.session_label.add_controller(click);
    }
}
