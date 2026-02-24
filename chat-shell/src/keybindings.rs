//! Keyboard shortcuts — global and context-specific.

use std::cell::RefCell;
use std::rc::Rc;

use glib::clone;
use gtk4::prelude::*;
use gtk4::EventControllerKey;

use levsha_engine::types::ShellToEngine;

use crate::chat_view::ChatView;
use crate::content_panel::ContentPanel;
use crate::input_bar::InputBar;
use crate::session_list::SessionListOverlay;
use crate::sidebar::SessionSidebar;
use crate::streaming::StreamState;
use crate::window::AppState;

/// Set up all keyboard shortcuts on the window.
pub fn setup(
    window: &libadwaita::ApplicationWindow,
    state: Rc<RefCell<AppState>>,
    input_bar: InputBar,
    chat_view: ChatView,
    content_panel: ContentPanel,
    session_list: SessionListOverlay,
    sidebar: SessionSidebar,
) {
    let key_ctrl = EventControllerKey::new();

    key_ctrl.connect_key_pressed(clone!(
        #[strong]
        state,
        #[strong]
        chat_view,
        #[strong]
        input_bar,
        #[strong]
        content_panel,
        #[strong]
        session_list,
        #[strong]
        sidebar,
        move |_, key, _keycode, modifier| {
            let ctrl = modifier.contains(gdk4::ModifierType::CONTROL_MASK);
            let shift = modifier.contains(gdk4::ModifierType::SHIFT_MASK);

            // Ctrl+C: cancel streaming.
            if ctrl && key == gdk4::Key::c {
                let st = state.borrow();
                if st.stream_state == StreamState::Streaming
                    || st.stream_state == StreamState::Waiting
                {
                    let tx = st.shell_tx.clone();
                    drop(st);
                    glib::spawn_future_local(async move {
                        let _ = tx.send(ShellToEngine::CancelStream).await;
                    });
                    return glib::Propagation::Stop;
                }
            }

            // Ctrl+F: toggle search overlay.
            if ctrl && key == gdk4::Key::f {
                chat_view.toggle_search();
                return glib::Propagation::Stop;
            }

            // Ctrl+L: clear visible chat.
            if ctrl && key == gdk4::Key::l {
                chat_view.clear();
                return glib::Propagation::Stop;
            }

            // Ctrl+W: close content panel if visible.
            if ctrl && key == gdk4::Key::w {
                if content_panel.is_visible() {
                    content_panel.hide();
                    return glib::Propagation::Stop;
                }
            }

            // Ctrl+Shift+P: toggle content panel.
            if ctrl && shift && key == gdk4::Key::p {
                content_panel.toggle();
                return glib::Propagation::Stop;
            }

            // Ctrl+N: new session.
            if ctrl && !shift && key == gdk4::Key::n {
                let tx = state.borrow().shell_tx.clone();
                glib::spawn_future_local(async move {
                    let _ = tx.send(ShellToEngine::SessionCreate { name: None }).await;
                });
                return glib::Propagation::Stop;
            }

            // Ctrl+Shift+S: open session list.
            if ctrl && shift && key == gdk4::Key::s {
                let tx = state.borrow().shell_tx.clone();
                glib::spawn_future_local(async move {
                    let _ = tx.send(ShellToEngine::SessionList).await;
                });
                return glib::Propagation::Stop;
            }

            // Ctrl+B: toggle sidebar.
            if ctrl && !shift && key == gdk4::Key::b {
                sidebar.toggle();
                sidebar.set_user_toggled(true);
                sidebar.send_toggle(sidebar.is_visible());
                return glib::Propagation::Stop;
            }

            // Ctrl+Tab: next session.
            if ctrl && !shift && key == gdk4::Key::Tab {
                let tx = state.borrow().shell_tx.clone();
                glib::spawn_future_local(async move {
                    let _ = tx.send(ShellToEngine::SessionNext).await;
                });
                return glib::Propagation::Stop;
            }

            // Ctrl+Shift+Tab: previous session.
            if ctrl && shift && key == gdk4::Key::Tab {
                let tx = state.borrow().shell_tx.clone();
                glib::spawn_future_local(async move {
                    let _ = tx.send(ShellToEngine::SessionPrev).await;
                });
                return glib::Propagation::Stop;
            }

            // Page Up: scroll up.
            if key == gdk4::Key::Page_Up {
                let adj = chat_view.scroll_window().vadjustment();
                adj.set_value(adj.value() - adj.page_size() * 0.8);
                return glib::Propagation::Stop;
            }

            // Page Down: scroll down.
            if key == gdk4::Key::Page_Down {
                let adj = chat_view.scroll_window().vadjustment();
                adj.set_value(adj.value() + adj.page_size() * 0.8);
                return glib::Propagation::Stop;
            }

            // Escape: close session list, then sidebar, then content panel, then focus input.
            if key == gdk4::Key::Escape {
                if session_list.is_visible() {
                    session_list.hide();
                    return glib::Propagation::Stop;
                }
                if sidebar.is_visible() {
                    sidebar.hide();
                    sidebar.set_user_toggled(true);
                    sidebar.send_toggle(false);
                    return glib::Propagation::Stop;
                }
                if content_panel.is_visible() {
                    content_panel.hide();
                    return glib::Propagation::Stop;
                }
                input_bar.grab_focus();
                return glib::Propagation::Stop;
            }

            glib::Propagation::Proceed
        }
    ));

    window.add_controller(key_ctrl);
}
