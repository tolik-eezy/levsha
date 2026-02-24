//! Error display — styled inline error messages with retry.
//! Also handles destructive command confirmation widgets.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Orientation};

use levsha_engine::types::ShellToEngine;

use crate::chat_view::ChatView;
use crate::input_bar::InputBar;
use crate::streaming::StreamState;
use crate::window::AppState;

/// Create an error widget with optional retry button.
pub fn create_error_widget(
    message: &str,
    retryable: bool,
    state: Rc<RefCell<AppState>>,
    chat_view: ChatView,
    input_bar: InputBar,
) -> gtk4::Box {
    let container = gtk4::Box::new(Orientation::Vertical, 8);
    container.add_css_class("error-widget");
    container.set_halign(Align::Center);
    container.set_hexpand(false);
    container.set_size_request(720.min(720), -1);

    // Error header: icon + message.
    let header = gtk4::Box::new(Orientation::Horizontal, 8);
    header.set_halign(Align::Start);

    let icon = gtk4::Label::new(Some("\u{2717}"));
    icon.add_css_class("error-icon");

    let msg_label = gtk4::Label::new(Some(message));
    msg_label.add_css_class("error-message");
    msg_label.set_wrap(true);
    msg_label.set_xalign(0.0);
    msg_label.set_hexpand(true);

    header.append(&icon);
    header.append(&msg_label);
    container.append(&header);

    // Retry button.
    if retryable {
        let retry_btn = gtk4::Button::with_label("Retry");
        retry_btn.add_css_class("retry-button");
        retry_btn.set_halign(Align::Start);

        {
            let state = state.clone();
            let chat_view = chat_view.clone();
            let input_bar = input_bar.clone();
            retry_btn.connect_clicked(move |_| {
                let st = state.borrow();
                if let Some(last_msg) = st.last_user_message.clone() {
                    let tx = st.shell_tx.clone();
                    drop(st);

                    // Disable input and show typing indicator.
                    {
                        let mut st = state.borrow_mut();
                        st.stream_state = StreamState::Waiting;
                        st.input_enabled = false;
                    }
                    input_bar.set_enabled(false);
                    chat_view.show_typing_indicator();

                    // Re-send the last user message.
                    let msg = ShellToEngine::UserMessage {
                        content: last_msg,
                    };
                    glib::spawn_future_local(async move {
                        let _ = tx.send(msg).await;
                    });
                }
            });
        }

        container.append(&retry_btn);
    }

    container
}

/// Create a destructive command confirmation widget.
pub fn create_confirm_widget(
    request_id: &str,
    command: &str,
    description: &str,
    state: Rc<RefCell<AppState>>,
) -> gtk4::Box {
    let container = gtk4::Box::new(Orientation::Vertical, 8);
    container.add_css_class("confirm-widget");
    container.set_halign(Align::Center);
    container.set_hexpand(false);
    container.set_size_request(720.min(720), -1);

    // Title row: shield icon + text.
    let title_row = gtk4::Box::new(Orientation::Horizontal, 6);
    title_row.set_halign(Align::Start);
    title_row.set_valign(Align::Center);

    let shield_icon = gtk4::Label::new(Some("\u{26A0}"));
    shield_icon.add_css_class("confirm-shield-icon");

    let title = gtk4::Label::new(Some("Confirm destructive command"));
    title.add_css_class("confirm-title");

    title_row.append(&shield_icon);
    title_row.append(&title);
    container.append(&title_row);

    // Command.
    let cmd_label = gtk4::Label::new(Some(command));
    cmd_label.add_css_class("confirm-command");
    cmd_label.set_halign(Align::Start);
    cmd_label.set_selectable(true);
    container.append(&cmd_label);

    // Description.
    let desc_label = gtk4::Label::new(Some(description));
    desc_label.add_css_class("confirm-description");
    desc_label.set_wrap(true);
    desc_label.set_xalign(0.0);
    container.append(&desc_label);

    // Button row.
    let buttons = gtk4::Box::new(Orientation::Horizontal, 8);
    buttons.set_halign(Align::Start);

    let approve_btn = gtk4::Button::with_label("Yes, remove");
    approve_btn.add_css_class("confirm-approve");

    let deny_btn = gtk4::Button::with_label("Cancel");
    deny_btn.add_css_class("confirm-deny");

    buttons.append(&approve_btn);
    buttons.append(&deny_btn);
    container.append(&buttons);

    // Wire up buttons.
    {
        let state = state.clone();
        let req_id = request_id.to_string();
        let container_weak = container.downgrade();
        approve_btn.connect_clicked(move |_| {
            send_confirm_response(&state, &req_id, true);
            if let Some(c) = container_weak.upgrade() {
                c.set_sensitive(false);
            }
        });
    }

    {
        let state = state.clone();
        let req_id = request_id.to_string();
        let container_weak = container.downgrade();
        deny_btn.connect_clicked(move |_| {
            send_confirm_response(&state, &req_id, false);
            if let Some(c) = container_weak.upgrade() {
                c.set_sensitive(false);
            }
        });
    }

    container
}

fn send_confirm_response(
    state: &Rc<RefCell<AppState>>,
    request_id: &str,
    approved: bool,
) {
    let tx = state.borrow().shell_tx.clone();
    let msg = ShellToEngine::ConfirmResponse {
        request_id: request_id.to_string(),
        approved,
    };
    glib::spawn_future_local(async move {
        let _ = tx.send(msg).await;
    });
}
