//! Boot splash screen — shown while the engine initializes.

use gtk4::prelude::*;
use gtk4::{Align, Orientation};

/// Embedded logo for the splash screen.
const SPLASH_LOGO: &[u8] = include_bytes!("../../assets/flea logo.png");

pub struct BootSplash {
    container: gtk4::Box,
}

impl BootSplash {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 16);
        container.add_css_class("boot-splash");
        container.set_halign(Align::Center);
        container.set_valign(Align::Center);
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Logo.
        if let Some(logo) = create_splash_logo(140) {
            container.append(&logo);
        }

        // Title.
        let title = gtk4::Label::new(Some("Levsha OS"));
        title.add_css_class("boot-splash-title");
        container.append(&title);

        // Animated dots (reuses typing-pulse keyframe).
        let dots_box = gtk4::Box::new(Orientation::Horizontal, 6);
        dots_box.set_halign(Align::Center);
        dots_box.set_margin_top(8);
        for _ in 0..3 {
            let dot = gtk4::Label::new(Some("\u{00B7}"));
            dot.add_css_class("boot-splash-dot");
            dots_box.append(&dot);
        }
        container.append(&dots_box);

        Self { container }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

fn create_splash_logo(size: i32) -> Option<gtk4::Frame> {
    let bytes = glib::Bytes::from_static(SPLASH_LOGO);
    let texture = gdk4::Texture::from_bytes(&bytes).ok()?;

    let picture = gtk4::Picture::for_paintable(&texture);
    picture.set_size_request(size, size);
    picture.set_content_fit(gtk4::ContentFit::Cover);

    let frame = gtk4::Frame::new(None);
    frame.add_css_class("boot-splash-logo");
    frame.set_child(Some(&picture));
    frame.set_size_request(size, size);
    frame.set_overflow(gtk4::Overflow::Hidden);

    Some(frame)
}
