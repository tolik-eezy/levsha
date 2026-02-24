//! Levsha OS Chat Shell (L3)
//!
//! Full-screen Wayland-native chat GUI. The only graphical surface in Levsha OS.
//! Uses GTK4 + libadwaita for rendering.

mod app;
mod avatar;
mod boot_splash;
mod chat_view;
mod content_panel;
mod error_display;
mod input_bar;
mod key_entry;
mod keybindings;
mod message_widget;
mod search;
mod session_list;
mod sidebar;
mod status_bar;
mod streaming;
mod window;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Levsha OS Chat Shell starting");

    let app = app::LevshApp::new();
    app.run();
}
