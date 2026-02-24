//! Application management — wraps adw::Application, loads CSS, wires engine.

use glib::clone;
use gtk4::prelude::*;
use gtk4::CssProvider;
use libadwaita as adw;

use crate::window::ChatWindow;

/// The top-level application struct.
pub struct LevshApp {
    app: adw::Application,
}

impl LevshApp {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id("os.levsha.chat")
            .build();

        app.connect_startup(|_| {
            load_css();
        });

        app.connect_activate(clone!(
            #[weak]
            app,
            move |_| {
                ChatWindow::new(&app);
            }
        ));

        Self { app }
    }

    pub fn run(&self) {
        self.app.run();
    }
}

/// Load the theme CSS from the embedded style.css.
fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("style.css"));

    gtk4::style_context_add_provider_for_display(
        &gdk4::Display::default().expect("Could not get default GDK display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
