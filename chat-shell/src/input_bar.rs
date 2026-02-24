//! Input bar — multi-line text entry with dynamic height.

use std::cell::RefCell;
use std::rc::Rc;

use glib::clone;
use gtk4::prelude::*;
use gtk4::{Align, EventControllerKey, Orientation, WrapMode};

use levsha_engine::types::ShellToEngine;

use crate::chat_view::ChatView;
use crate::streaming::StreamState;
use crate::window::AppState;

/// The input bar at the bottom of the chat.
#[derive(Clone)]
pub struct InputBar {
    container: gtk4::Box,
    text_view: gtk4::TextView,
    send_button: gtk4::Button,
    state: Rc<RefCell<AppState>>,
    chat_view: ChatView,
}

impl InputBar {
    pub fn new(state: Rc<RefCell<AppState>>, chat_view: ChatView) -> Self {
        // Outer container with padding.
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("input-area");

        // Frame for the input (border, background).
        let frame = gtk4::Box::new(Orientation::Horizontal, 8);
        frame.add_css_class("input-frame");
        frame.set_hexpand(true);

        // Multi-line text view.
        let text_view = gtk4::TextView::builder()
            .wrap_mode(WrapMode::WordChar)
            .hexpand(true)
            .accepts_tab(false)
            .build();
        text_view.add_css_class("input-text");

        // Send button.
        let send_button = gtk4::Button::with_label("\u{2192}");
        send_button.add_css_class("send-button");
        send_button.set_valign(Align::End);
        send_button.set_sensitive(false);

        frame.append(&text_view);
        frame.append(&send_button);
        container.append(&frame);

        let input_bar = Self {
            container,
            text_view: text_view.clone(),
            send_button: send_button.clone(),
            state: state.clone(),
            chat_view: chat_view.clone(),
        };

        // Track buffer changes to enable/disable send button and constrain height.
        let buffer = text_view.buffer();
        let btn = send_button.clone();
        buffer.connect_changed(clone!(
            #[strong]
            btn,
            move |buf| {
                let has_text = buf.char_count() > 0;
                btn.set_sensitive(has_text);
            }
        ));

        // Send button clicked.
        let ib = input_bar.clone();
        send_button.connect_clicked(move |_| {
            ib.send_message();
        });

        // Key controller for Enter/Shift+Enter.
        let key_ctrl = EventControllerKey::new();
        let ib2 = input_bar.clone();
        key_ctrl.connect_key_pressed(move |_, key, _keycode, modifier| {
            let shift = modifier.contains(gdk4::ModifierType::SHIFT_MASK);

            if key == gdk4::Key::Return || key == gdk4::Key::KP_Enter {
                if shift {
                    // Shift+Enter: insert newline (let default handler do it).
                    return glib::Propagation::Proceed;
                }
                // Enter: send message.
                ib2.send_message();
                return glib::Propagation::Stop;
            }

            // Up arrow when input is empty: recall previous message.
            if key == gdk4::Key::Up {
                let buffer = ib2.text_view.buffer();
                if buffer.char_count() == 0 {
                    let st = ib2.state.borrow();
                    if !st.user_message_history.is_empty() {
                        let idx = st.history_index.map(|i| i.saturating_sub(1)).unwrap_or(st.user_message_history.len() - 1);
                        let msg = st.user_message_history[idx].clone();
                        drop(st);
                        ib2.state.borrow_mut().history_index = Some(idx);
                        buffer.set_text(&msg);
                        return glib::Propagation::Stop;
                    }
                }
            }

            // Down arrow: navigate forward in history.
            if key == gdk4::Key::Down {
                let buffer = ib2.text_view.buffer();
                let st = ib2.state.borrow();
                if let Some(idx) = st.history_index {
                    if idx + 1 < st.user_message_history.len() {
                        let msg = st.user_message_history[idx + 1].clone();
                        drop(st);
                        ib2.state.borrow_mut().history_index = Some(idx + 1);
                        buffer.set_text(&msg);
                    } else {
                        drop(st);
                        ib2.state.borrow_mut().history_index = None;
                        buffer.set_text("");
                    }
                    return glib::Propagation::Stop;
                }
            }

            glib::Propagation::Proceed
        });
        text_view.add_controller(key_ctrl);

        input_bar
    }

    /// Returns the top-level widget.
    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Grab focus to the input text view.
    pub fn grab_focus(&self) {
        self.text_view.grab_focus();
    }

    /// Enable or disable the input bar.
    pub fn set_enabled(&self, enabled: bool) {
        self.text_view.set_sensitive(enabled);
        self.send_button.set_sensitive(enabled && self.text_view.buffer().char_count() > 0);
        if enabled {
            self.container.remove_css_class("input-disabled");
        } else {
            self.container.add_css_class("input-disabled");
        }
    }

    /// Get the current text in the input.
    pub fn text(&self) -> String {
        let buffer = self.text_view.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        buffer.text(&start, &end, false).to_string()
    }

    /// Clear the input.
    pub fn clear(&self) {
        self.text_view.buffer().set_text("");
    }

    /// Send the current message.
    pub fn send_message(&self) {
        let text = self.text().trim().to_string();
        if text.is_empty() {
            return;
        }

        let st = self.state.borrow();
        if !st.input_enabled {
            return;
        }
        if st.stream_state != StreamState::Idle {
            return;
        }
        let tx = st.shell_tx.clone();
        drop(st);

        // Add user message to chat view.
        self.chat_view.add_message(
            levsha_engine::types::MessageRole::User,
            &text,
            true,
        );

        // Store in history.
        {
            let mut st = self.state.borrow_mut();
            st.last_user_message = Some(text.clone());
            st.user_message_history.push(text.clone());
            st.history_index = None;
            st.stream_state = StreamState::Waiting;
            st.input_enabled = false;
        }

        // Clear input and disable.
        self.clear();
        self.set_enabled(false);

        // Show typing indicator.
        self.chat_view.show_typing_indicator();

        // Send to engine.
        let msg = ShellToEngine::UserMessage { content: text };
        glib::spawn_future_local(async move {
            let _ = tx.send(msg).await;
        });
    }
}
