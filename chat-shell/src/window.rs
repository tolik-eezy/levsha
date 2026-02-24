//! Main window — full-screen, no decorations.
//!
//! Four-region layout: sidebar (left), chat view (scrollable), input bar (fixed), status bar (fixed).
//! Creates the engine channels and spawns the engine on a tokio runtime.
//! Phase 2.0: Adds Stack for first-boot vs chat, Paned split-view with content panel.
//! Module 26: Adds persistent session sidebar with navigation.

use std::cell::RefCell;
use std::rc::Rc;

use glib::clone;
use gtk4::prelude::*;
use gtk4::{Align, Orientation};
use libadwaita as adw;
use tokio::sync::mpsc;

use levsha_engine::config::Config;
use levsha_engine::protocol::{self, ProtocolShellSide};
use levsha_engine::types::{
    ContentType, ContentUpdateData, EngineToShell, MessageRole, ShellToEngine,
    ToolExecutionStatus,
};
use levsha_engine::Engine;

use crate::boot_splash::BootSplash;
use crate::chat_view::ChatView;
use crate::content_panel::ContentPanel;
use crate::error_display;
use crate::input_bar::InputBar;
use crate::key_entry::{FirstBootScreen, InlineKeyEntry};
use crate::keybindings;
use crate::session_list::SessionListOverlay;
use crate::sidebar::SessionSidebar;
use crate::status_bar::StatusBar;
use crate::streaming::StreamState;

/// Shared application state accessible from callbacks.
pub struct AppState {
    pub shell_tx: mpsc::Sender<ShellToEngine>,
    pub stream_state: StreamState,
    pub last_user_message: Option<String>,
    pub user_message_history: Vec<String>,
    pub history_index: Option<usize>,
    pub input_enabled: bool,
    pub first_boot_screen: Option<FirstBootScreen>,
    pub inline_key_entry: Option<InlineKeyEntry>,
    pub current_session_name: Option<String>,
    /// Active self-improve content panel ID (when tool activity is routed to split view).
    pub self_improve_panel_id: Option<String>,
    /// When true, the next SessionList response only refreshes the sidebar (no overlay).
    pub sidebar_pending_refresh: bool,
}

pub struct ChatWindow;

impl ChatWindow {
    pub fn new(app: &adw::Application) -> adw::ApplicationWindow {
        // Create the engine channels.
        let (shell_side, engine_side) = protocol::create_channels();

        let ProtocolShellSide { sender, receiver } = shell_side;

        // Build shared state.
        let state = Rc::new(RefCell::new(AppState {
            shell_tx: sender,
            stream_state: StreamState::Idle,
            last_user_message: None,
            user_message_history: Vec::new(),
            history_index: None,
            input_enabled: true,
            first_boot_screen: None,
            inline_key_entry: None,
            current_session_name: None,
            self_improve_panel_id: None,
            sidebar_pending_refresh: false,
        }));

        // Build UI components.
        let chat_view = ChatView::new();
        let input_bar = InputBar::new(state.clone(), chat_view.clone());
        let status_bar = StatusBar::new();
        let content_panel = ContentPanel::new();
        let session_list = SessionListOverlay::new(state.borrow().shell_tx.clone());
        let sidebar = SessionSidebar::new(state.borrow().shell_tx.clone());

        // Content panel starts hidden.
        content_panel.hide();

        // Chat area: chat_view + input_bar (left side of paned).
        let chat_box = gtk4::Box::new(Orientation::Vertical, 0);
        chat_box.set_hexpand(true);
        chat_box.set_vexpand(true);
        chat_box.append(chat_view.widget());
        chat_box.append(input_bar.widget());

        // Stack for first-boot vs chat. The "chat" child is the chat_box.
        let stack = gtk4::Stack::new();
        stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
        stack.set_transition_duration(400);
        stack.set_hexpand(true);
        stack.set_vexpand(true);
        stack.add_named(&chat_box, Some("chat"));

        // Boot splash — shown first while engine initializes.
        let boot_splash = BootSplash::new();
        stack.add_named(boot_splash.widget(), Some("boot-splash"));
        stack.set_visible_child_name("boot-splash");

        // Safety timeout: dismiss splash after 10 seconds if engine never responds.
        {
            let stack_ref = stack.clone();
            let input_bar_ref = input_bar.clone();
            glib::timeout_add_local_once(
                std::time::Duration::from_secs(10),
                move || {
                    if stack_ref.visible_child_name().as_deref() == Some("boot-splash") {
                        stack_ref.set_visible_child_name("chat");
                        input_bar_ref.grab_focus();
                    }
                },
            );
        }

        // Paned split-view: left = stack, right = content panel.
        let paned = gtk4::Paned::new(Orientation::Horizontal);
        paned.add_css_class("main-paned");
        paned.set_start_child(Some(&stack));
        paned.set_end_child(Some(content_panel.widget()));
        paned.set_resize_start_child(true);
        paned.set_shrink_start_child(false);
        paned.set_resize_end_child(true);
        paned.set_shrink_end_child(true);
        paned.set_hexpand(true);
        paned.set_vexpand(true);

        // Main vertical box: paned + status bar.
        let main_box = gtk4::Box::new(Orientation::Vertical, 0);
        main_box.add_css_class("main-box");
        main_box.set_hexpand(true);
        main_box.append(&paned);
        main_box.append(status_bar.widget());

        // Outer horizontal box: sidebar + main_box.
        let outer_box = gtk4::Box::new(Orientation::Horizontal, 0);
        outer_box.append(&sidebar.widget());
        outer_box.append(&main_box);

        // Overlay: outer_box as base, session list + toggle button on top.
        let window_overlay = gtk4::Overlay::new();
        window_overlay.set_child(Some(&outer_box));
        window_overlay.add_overlay(&session_list.widget());

        // Sidebar toggle button as overlay (top-left when sidebar is hidden).
        let toggle_btn = sidebar.toggle_button();
        toggle_btn.set_halign(Align::Start);
        toggle_btn.set_valign(Align::Start);
        toggle_btn.set_margin_start(8);
        toggle_btn.set_margin_top(8);
        window_overlay.add_overlay(&toggle_btn);

        // Make session label clickable for opening session list.
        status_bar.connect_session_click(state.borrow().shell_tx.clone());

        // Build window.
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Levsha OS")
            .fullscreened(true)
            .decorated(false)
            .content(&window_overlay)
            .build();

        // Set up keyboard shortcuts.
        keybindings::setup(
            &window,
            state.clone(),
            input_bar.clone(),
            chat_view.clone(),
            content_panel.clone(),
            session_list.clone(),
            sidebar.clone(),
        );

        // Spawn the engine on a background tokio runtime.
        spawn_engine(engine_side);

        // Start receiving messages from the engine.
        start_engine_listener(
            receiver,
            state.clone(),
            chat_view.clone(),
            input_bar.clone(),
            status_bar.clone(),
            content_panel.clone(),
            session_list.clone(),
            stack.clone(),
            sidebar.clone(),
        );

        // Start the clock in the status bar.
        status_bar.start_clock();

        window.present();

        // Set split-view position: chat (left) = 35%, content panel (right) = 65%.
        // Must be done after present so the window has an allocated width.
        {
            let paned_ref = paned.clone();
            glib::idle_add_local_once(move || {
                let width = paned_ref.allocated_width();
                if width > 0 {
                    paned_ref.set_position((width as f64 * 0.35) as i32);
                }
            });
        }

        // Focus the input field AFTER the window is presented.
        // grab_focus() requires the widget to be mapped (visible on screen).
        let ib = input_bar.clone();
        glib::idle_add_local_once(move || {
            ib.grab_focus();
        });

        window
    }
}

/// Spawn the engine on a separate tokio runtime.
///
/// Loads configuration (with dev-mode fallback), creates the Engine, and
/// runs its main event loop. If initialization fails, sends an Error message
/// to the shell so the user sees what went wrong.
fn spawn_engine(engine_side: protocol::ProtocolEngineSide) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(async move {
            // Clone the sender before moving engine_side — we need it for error reporting
            // if Engine::new() fails (which consumes engine_side).
            let error_sender = engine_side.sender.clone();

            // Load configuration with dev-mode fallback.
            let config = match Config::load_with_fallback() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to load config: {}", e);
                    let _ = error_sender
                        .send(EngineToShell::Error {
                            message: format!("Failed to load configuration: {}", e),
                            retryable: false,
                        })
                        .await;
                    return;
                }
            };

            // Create and run the engine.
            match Engine::new(config, engine_side) {
                Ok(engine) => {
                    engine.run().await;
                }
                Err(e) => {
                    tracing::error!("Engine initialization failed: {}", e);
                    let _ = error_sender
                        .send(EngineToShell::Error {
                            message: format!("Engine initialization failed: {}", e),
                            retryable: false,
                        })
                        .await;
                }
            }
        });
    });
}

/// Listen for engine messages on the glib main context.
fn start_engine_listener(
    mut receiver: mpsc::Receiver<EngineToShell>,
    state: Rc<RefCell<AppState>>,
    chat_view: ChatView,
    input_bar: InputBar,
    status_bar: StatusBar,
    content_panel: ContentPanel,
    session_list: SessionListOverlay,
    stack: gtk4::Stack,
    sidebar: SessionSidebar,
) {
    glib::spawn_future_local(clone!(
        #[strong]
        state,
        #[strong]
        chat_view,
        #[strong]
        input_bar,
        #[strong]
        status_bar,
        #[strong]
        content_panel,
        #[strong]
        session_list,
        #[strong]
        stack,
        #[strong]
        sidebar,
        async move {
            while let Some(msg) = poll_receiver(&mut receiver).await {
                handle_engine_message(
                    msg,
                    &state,
                    &chat_view,
                    &input_bar,
                    &status_bar,
                    &content_panel,
                    &session_list,
                    &stack,
                    &sidebar,
                );
            }
        }
    ));
}

/// Poll the tokio receiver from glib's async context.
async fn poll_receiver(rx: &mut mpsc::Receiver<EngineToShell>) -> Option<EngineToShell> {
    // glib::spawn_future_local runs on the glib main loop.
    // We need to yield to glib periodically. tokio channels are compatible with
    // any async runtime since they use cooperative yielding.
    rx.recv().await
}

/// Handle a single message from the engine.
fn handle_engine_message(
    msg: EngineToShell,
    state: &Rc<RefCell<AppState>>,
    chat_view: &ChatView,
    input_bar: &InputBar,
    status_bar: &StatusBar,
    content_panel: &ContentPanel,
    session_list: &SessionListOverlay,
    stack: &gtk4::Stack,
    sidebar: &SessionSidebar,
) {
    match msg {
        EngineToShell::HistoryMessage {
            role,
            content,
            timestamp: _,
        } => {
            chat_view.add_message(role, &content, false);
        }

        EngineToShell::StreamChunk { content } => {
            let mut st = state.borrow_mut();
            if st.stream_state == StreamState::Idle || st.stream_state == StreamState::Waiting {
                // First chunk: transition to streaming.
                st.stream_state = StreamState::Streaming;
                drop(st);
                chat_view.remove_typing_indicator();
                chat_view.begin_streaming_message();
            } else {
                drop(st);
            }
            chat_view.append_streaming_chunk(&content);
        }

        EngineToShell::StreamEnd => {
            let mut st = state.borrow_mut();
            st.stream_state = StreamState::Idle;
            st.input_enabled = true;
            st.self_improve_panel_id = None;
            drop(st);
            chat_view.finalize_streaming_message();
            chat_view.remove_typing_indicator();
            input_bar.set_enabled(true);
            input_bar.grab_focus();
        }

        EngineToShell::Error { message, retryable } => {
            let mut st = state.borrow_mut();
            st.stream_state = StreamState::Idle;
            st.input_enabled = true;
            drop(st);
            chat_view.remove_typing_indicator();
            chat_view.finalize_streaming_message();
            let error_widget = error_display::create_error_widget(
                &message,
                retryable,
                state.clone(),
                chat_view.clone(),
                input_bar.clone(),
            );
            chat_view.add_widget(error_widget);
            input_bar.set_enabled(true);
            input_bar.grab_focus();
        }

        EngineToShell::ConnectionStatus { connected, backend } => {
            status_bar.set_connection_status(connected, &backend);
            // Dismiss boot splash on first successful connection.
            if connected {
                if stack.visible_child_name().as_deref() == Some("boot-splash") {
                    stack.set_visible_child_name("chat");
                    input_bar.grab_focus();
                }
                // Request initial session list to populate sidebar (once engine is ready).
                state.borrow_mut().sidebar_pending_refresh = true;
                let tx = state.borrow().shell_tx.clone();
                glib::spawn_future_local(async move {
                    let _ = tx.send(ShellToEngine::SessionList).await;
                });
            }
        }

        EngineToShell::ToolStatus {
            tool_name,
            status,
            description,
        } => {
            let in_self_improve = state.borrow().self_improve_panel_id.is_some();

            if is_self_improve_tool(&tool_name) || in_self_improve {
                // Route self-improve tool activity to the split-view panel.
                if !in_self_improve {
                    let panel_id = "self-improve".to_string();
                    content_panel.open(
                        &panel_id,
                        &ContentType::ProgressView {
                            operation: "Self-Improvement".to_string(),
                            total: None,
                        },
                        "Self-Improvement",
                        "",
                    );
                    state.borrow_mut().self_improve_panel_id = Some(panel_id);
                }

                let panel_id = state.borrow().self_improve_panel_id.clone().unwrap();

                let icon = match &status {
                    ToolExecutionStatus::Started => "\u{25D0}",
                    ToolExecutionStatus::Completed { success } => {
                        if *success {
                            "\u{2713}"
                        } else {
                            "\u{2717}"
                        }
                    }
                };
                let line = format!("{} {}: {}\n", icon, tool_name, description);
                content_panel.update(
                    &panel_id,
                    &ContentUpdateData::AppendText { text: line },
                );
            } else {
                chat_view.add_tool_status(&tool_name, &status, &description);
            }
        }

        EngineToShell::ConfirmRequest {
            request_id,
            command,
            description,
        } => {
            let confirm_widget = error_display::create_confirm_widget(
                &request_id,
                &command,
                &description,
                state.clone(),
            );
            chat_view.add_widget(confirm_widget);
        }

        // ── Phase 2.0: API Key Management ──

        EngineToShell::KeyRequired => {
            let tx = state.borrow().shell_tx.clone();
            let screen = FirstBootScreen::new(tx);
            stack.add_named(screen.widget(), Some("first-boot"));
            stack.set_visible_child_name("first-boot");
            state.borrow_mut().first_boot_screen = Some(screen);
        }

        EngineToShell::KeyValidationResult {
            success,
            error_message,
        } => {
            let st = state.borrow();
            if let Some(ref screen) = st.first_boot_screen {
                if success {
                    screen.show_success();
                    let stack_clone = stack.clone();
                    let input_bar_clone = input_bar.clone();
                    glib::timeout_add_local_once(
                        std::time::Duration::from_millis(800),
                        move || {
                            stack_clone.set_visible_child_name("chat");
                            input_bar_clone.grab_focus();
                        },
                    );
                } else {
                    let msg = error_message
                        .as_deref()
                        .unwrap_or("Invalid API key. Please try again.");
                    screen.show_error(msg);
                    screen.set_enabled(true);
                }
            }
        }

        EngineToShell::ModelChanged {
            model_id: _,
            display_name,
        } => {
            status_bar.set_connection_status_with_detail(true, &display_name, "");
        }

        EngineToShell::RequestKeyChange => {
            let tx = state.borrow().shell_tx.clone();
            let card = InlineKeyEntry::new(tx);
            chat_view.add_widget(card.widget().clone());
            state.borrow_mut().inline_key_entry = Some(card);
        }

        // ── Phase 2.0: Content Panel ──

        EngineToShell::ContentOpen {
            content_id,
            content_type,
            title,
            content,
        } => {
            // Engine-initiated content panels supersede the self-improve panel.
            state.borrow_mut().self_improve_panel_id = None;
            content_panel.open(&content_id, &content_type, &title, &content);
        }

        EngineToShell::ContentUpdate { content_id, data } => {
            content_panel.update(&content_id, &data);
        }

        EngineToShell::ContentClose { content_id } => {
            content_panel.close(&content_id);
        }

        // ── Phase 2.1: Backend Status ──

        EngineToShell::BackendStatus {
            mode,
            active,
            local_available,
            cloud_available,
        } => {
            status_bar.set_backend_status(&mode, &active, local_available, cloud_available);
        }

        // ── Phase 2.1: Session Management ──

        EngineToShell::SessionSwitched { session } => {
            chat_view.clear();
            state.borrow_mut().current_session_name = Some(session.name.clone());
            status_bar.set_session_name(Some(&session.name));
            // Request fresh session list to update sidebar.
            state.borrow_mut().sidebar_pending_refresh = true;
            let tx = state.borrow().shell_tx.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(ShellToEngine::SessionList).await;
            });
        }

        EngineToShell::SessionList { sessions } => {
            let is_refresh = state.borrow().sidebar_pending_refresh;
            if is_refresh {
                state.borrow_mut().sidebar_pending_refresh = false;
                sidebar.update_sessions(&sessions);
            } else {
                session_list.show(&sessions);
                sidebar.update_sessions(&sessions);
            }
            // Auto-show/hide sidebar based on session count.
            if !sidebar.user_toggled() {
                if sessions.len() >= 2 {
                    sidebar.show();
                } else {
                    sidebar.hide();
                }
            }
        }

        EngineToShell::SessionCreated { session } => {
            session_list.hide();
            chat_view.clear();
            state.borrow_mut().current_session_name = Some(session.name.clone());
            status_bar.set_session_name(Some(&session.name));
            input_bar.grab_focus();
            // Request session list to update sidebar.
            state.borrow_mut().sidebar_pending_refresh = true;
            let tx = state.borrow().shell_tx.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(ShellToEngine::SessionList).await;
            });
        }

        EngineToShell::SessionRenamed {
            session_id: _,
            name,
        } => {
            // If this is the active session, update status bar.
            status_bar.set_session_name(Some(&name));
            state.borrow_mut().current_session_name = Some(name);
            // Refresh sidebar to show updated name.
            state.borrow_mut().sidebar_pending_refresh = true;
            let tx = state.borrow().shell_tx.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(ShellToEngine::SessionList).await;
            });
        }

        EngineToShell::SessionArchived { session_id: _ } => {
            // Archived sessions are hidden from the list; no UI change needed.
        }

        EngineToShell::SessionDeleted { session_id: _ } => {
            // Engine will send SessionSwitched for the new active session.
            // Also request session list refresh for sidebar.
            state.borrow_mut().sidebar_pending_refresh = true;
            let tx = state.borrow().shell_tx.clone();
            glib::spawn_future_local(async move {
                let _ = tx.send(ShellToEngine::SessionList).await;
            });
        }
    }
}

/// Returns true if the tool name indicates self-improvement activity.
///
/// When a self-improve tool is detected, tool status messages are routed
/// to the split-view content panel instead of appearing inline in the chat.
fn is_self_improve_tool(name: &str) -> bool {
    name.starts_with("source_")
        || matches!(
            name,
            "build"
                | "build_status"
                | "test"
                | "deploy"
                | "rollback"
                | "git_log"
                | "git_commit"
                | "checkpoint_list"
                | "checkpoint_create"
                | "checkpoint_restore"
        )
}
