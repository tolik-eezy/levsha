//! API key entry screens — first-boot full-screen and inline card.

use glib::clone;
use gtk4::prelude::*;
use gtk4::{Align, EventControllerKey, Orientation};

use levsha_engine::types::ShellToEngine;
use tokio::sync::mpsc;

/// Full-screen first-boot key entry screen.
#[derive(Clone)]
pub struct FirstBootScreen {
    container: gtk4::Box,
    entry: gtk4::PasswordEntry,
    status_label: gtk4::Label,
    error_label: gtk4::Label,
    shell_tx: mpsc::Sender<ShellToEngine>,
}

impl FirstBootScreen {
    pub fn new(shell_tx: mpsc::Sender<ShellToEngine>) -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("first-boot-screen");
        container.set_halign(Align::Center);
        container.set_valign(Align::Center);
        container.set_hexpand(true);
        container.set_vexpand(true);

        let content = gtk4::Box::new(Orientation::Vertical, 0);
        content.add_css_class("first-boot-content");
        content.set_halign(Align::Center);
        content.set_size_request(400, -1);

        // Logo.
        let logo_bytes = include_bytes!("../../assets/logo2.png");
        let gbytes = glib::Bytes::from_static(logo_bytes);
        if let Ok(texture) = gdk4::Texture::from_bytes(&gbytes) {
            let picture = gtk4::Picture::for_paintable(&texture);
            picture.add_css_class("first-boot-logo");
            picture.set_size_request(80, 80);
            picture.set_halign(Align::Center);
            picture.set_margin_bottom(24);
            content.append(&picture);
        }

        // Title.
        let title = gtk4::Label::new(Some("Welcome to Levsha OS"));
        title.add_css_class("first-boot-title");
        title.set_halign(Align::Center);
        title.set_margin_bottom(8);
        content.append(&title);

        // Description.
        let desc = gtk4::Label::new(Some(
            "To get started, enter your Anthropic API key.\nThis key is stored locally and never shared.",
        ));
        desc.add_css_class("first-boot-description");
        desc.set_halign(Align::Center);
        desc.set_justify(gtk4::Justification::Center);
        desc.set_wrap(true);
        desc.set_margin_bottom(24);
        content.append(&desc);

        // URL hint.
        let url_label = gtk4::Label::new(Some("console.anthropic.com"));
        url_label.add_css_class("first-boot-url");
        url_label.set_halign(Align::Center);
        url_label.set_margin_bottom(32);
        content.append(&url_label);

        // Password entry.
        let entry_container = gtk4::Box::new(Orientation::Vertical, 0);
        entry_container.add_css_class("key-input-container");
        entry_container.set_halign(Align::Center);

        let entry = gtk4::PasswordEntry::builder()
            .show_peek_icon(true)
            .placeholder_text("sk-ant-...")
            .build();
        entry.add_css_class("key-input-entry");
        entry_container.append(&entry);
        content.append(&entry_container);

        // Submit button.
        let submit_btn = gtk4::Button::with_label("Connect");
        submit_btn.add_css_class("first-boot-submit");
        submit_btn.set_halign(Align::Center);
        submit_btn.set_margin_top(16);
        submit_btn.set_size_request(200, -1);
        content.append(&submit_btn);

        // Status label (hidden initially).
        let status_label = gtk4::Label::new(None);
        status_label.add_css_class("first-boot-status");
        status_label.set_halign(Align::Center);
        status_label.set_margin_top(16);
        status_label.set_visible(false);
        content.append(&status_label);

        // Error label (hidden initially).
        let error_label = gtk4::Label::new(None);
        error_label.add_css_class("first-boot-error-text");
        error_label.set_halign(Align::Center);
        error_label.set_margin_top(8);
        error_label.set_visible(false);
        error_label.set_wrap(true);
        content.append(&error_label);

        container.append(&content);

        let screen = Self {
            container,
            entry: entry.clone(),
            status_label,
            error_label,
            shell_tx,
        };

        // Enter key triggers submission.
        let key_ctrl = EventControllerKey::new();
        let screen_clone = screen.clone();
        key_ctrl.connect_key_pressed(clone!(
            #[strong]
            screen_clone,
            move |_, key, _keycode, _modifier| {
                if key == gdk4::Key::Return || key == gdk4::Key::KP_Enter {
                    screen_clone.try_submit();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        entry.add_controller(key_ctrl);

        // Button click triggers submission.
        let screen_clone2 = screen.clone();
        submit_btn.connect_clicked(move |_| {
            screen_clone2.try_submit();
        });

        screen
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    fn try_submit(&self) {
        let key = self.entry.text().to_string();

        // Basic format validation.
        if !key.starts_with("sk-ant-") || key.len() <= 20 {
            self.show_error("Invalid key format. Key should start with 'sk-ant-' and be at least 20 characters.");
            return;
        }

        self.show_validating();
        self.set_enabled(false);

        let tx = self.shell_tx.clone();
        glib::spawn_future_local(async move {
            let _ = tx.send(ShellToEngine::SubmitApiKey { key }).await;
        });
    }

    pub fn show_validating(&self) {
        self.error_label.set_visible(false);
        self.status_label.set_text("Validating...");
        self.status_label.set_visible(true);
        self.status_label.remove_css_class("first-boot-success");
    }

    pub fn show_error(&self, msg: &str) {
        self.status_label.set_visible(false);
        self.error_label.set_text(msg);
        self.error_label.set_visible(true);
    }

    pub fn show_success(&self) {
        self.error_label.set_visible(false);
        self.status_label.set_text("Key validated successfully!");
        self.status_label.add_css_class("first-boot-success");
        self.status_label.set_visible(true);
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.entry.set_sensitive(enabled);
    }
}

/// Compact inline key entry card for mid-session key changes.
#[derive(Clone)]
pub struct InlineKeyEntry {
    container: gtk4::Box,
    entry: gtk4::PasswordEntry,
    status_label: gtk4::Label,
    error_label: gtk4::Label,
    shell_tx: mpsc::Sender<ShellToEngine>,
}

impl InlineKeyEntry {
    pub fn new(shell_tx: mpsc::Sender<ShellToEngine>) -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 8);
        container.add_css_class("key-change-card");
        container.set_halign(Align::Center);
        container.set_hexpand(false);
        container.set_size_request(720.min(720), -1);

        let title = gtk4::Label::new(Some("Update API Key"));
        title.add_css_class("first-boot-title");
        title.set_halign(Align::Start);
        container.append(&title);

        let desc = gtk4::Label::new(Some("Enter your new Anthropic API key below."));
        desc.add_css_class("first-boot-description");
        desc.set_halign(Align::Start);
        desc.set_margin_bottom(8);
        container.append(&desc);

        let entry = gtk4::PasswordEntry::builder()
            .show_peek_icon(true)
            .placeholder_text("sk-ant-...")
            .build();
        entry.add_css_class("key-input-entry");
        container.append(&entry);

        // Submit button.
        let submit_btn = gtk4::Button::with_label("Connect");
        submit_btn.add_css_class("first-boot-submit");
        submit_btn.set_halign(Align::Start);
        submit_btn.set_margin_top(8);
        container.append(&submit_btn);

        let status_label = gtk4::Label::new(None);
        status_label.add_css_class("first-boot-status");
        status_label.set_halign(Align::Start);
        status_label.set_visible(false);
        container.append(&status_label);

        let error_label = gtk4::Label::new(None);
        error_label.add_css_class("first-boot-error-text");
        error_label.set_halign(Align::Start);
        error_label.set_visible(false);
        error_label.set_wrap(true);
        container.append(&error_label);

        let card = Self {
            container,
            entry: entry.clone(),
            status_label,
            error_label,
            shell_tx,
        };

        // Enter key triggers submission.
        let key_ctrl = EventControllerKey::new();
        let card_clone = card.clone();
        key_ctrl.connect_key_pressed(clone!(
            #[strong]
            card_clone,
            move |_, key, _keycode, _modifier| {
                if key == gdk4::Key::Return || key == gdk4::Key::KP_Enter {
                    card_clone.try_submit();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        entry.add_controller(key_ctrl);

        // Button click triggers submission.
        let card_clone2 = card.clone();
        submit_btn.connect_clicked(move |_| {
            card_clone2.try_submit();
        });

        card
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    fn try_submit(&self) {
        let key = self.entry.text().to_string();

        if !key.starts_with("sk-ant-") || key.len() <= 20 {
            self.error_label.set_text("Invalid key format. Key should start with 'sk-ant-' and be at least 20 characters.");
            self.error_label.set_visible(true);
            return;
        }

        self.error_label.set_visible(false);
        self.status_label.set_text("Validating...");
        self.status_label.set_visible(true);
        self.entry.set_sensitive(false);

        let tx = self.shell_tx.clone();
        glib::spawn_future_local(async move {
            let _ = tx.send(ShellToEngine::SubmitApiKey { key }).await;
        });
    }
}
