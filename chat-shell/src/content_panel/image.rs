//! Image renderer — displays images with zoom controls.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;
use gtk4::{Align, Orientation, PolicyType, ScrolledWindow};

use base64::Engine as _;

/// Renders an image with zoom controls.
#[derive(Clone)]
pub struct ImageRenderer {
    container: gtk4::Box,
    picture: gtk4::Picture,
    _scroll: ScrolledWindow,
    zoom_label: gtk4::Label,
    zoom_level: Rc<Cell<f64>>,
}

impl ImageRenderer {
    pub fn new() -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 0);
        container.add_css_class("image-renderer");
        container.set_hexpand(true);
        container.set_vexpand(true);

        // Scroll area for the image.
        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(PolicyType::Automatic)
            .vscrollbar_policy(PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .build();

        let picture = gtk4::Picture::new();
        picture.set_halign(Align::Center);
        picture.set_valign(Align::Center);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk4::ContentFit::Contain);
        scroll.set_child(Some(&picture));

        container.append(&scroll);

        // Zoom bar.
        let zoom_bar = gtk4::Box::new(Orientation::Horizontal, 4);
        zoom_bar.add_css_class("image-zoom-bar");
        zoom_bar.set_halign(Align::Center);

        let fit_btn = gtk4::Button::with_label("Fit");
        fit_btn.add_css_class("image-zoom-button");

        let zoom_out_btn = gtk4::Button::with_label("\u{2212}");
        zoom_out_btn.add_css_class("image-zoom-button");

        let zoom_label = gtk4::Label::new(Some("100%"));
        zoom_label.add_css_class("image-zoom-button");
        zoom_label.set_width_chars(5);

        let zoom_in_btn = gtk4::Button::with_label("+");
        zoom_in_btn.add_css_class("image-zoom-button");

        zoom_bar.append(&fit_btn);
        zoom_bar.append(&zoom_out_btn);
        zoom_bar.append(&zoom_label);
        zoom_bar.append(&zoom_in_btn);

        container.append(&zoom_bar);

        let zoom_level = Rc::new(Cell::new(1.0));

        let renderer = Self {
            container,
            picture: picture.clone(),
            _scroll: scroll,
            zoom_label: zoom_label.clone(),
            zoom_level: zoom_level.clone(),
        };

        // Wire zoom controls.
        {
            let r = renderer.clone();
            fit_btn.connect_clicked(move |_| {
                r.zoom_level.set(1.0);
                r.picture.set_can_shrink(true);
                r.picture.set_size_request(-1, -1);
                r.zoom_label.set_text("Fit");
            });
        }
        {
            let r = renderer.clone();
            zoom_in_btn.connect_clicked(move |_| {
                r.zoom_by(1.25);
            });
        }
        {
            let r = renderer.clone();
            zoom_out_btn.connect_clicked(move |_| {
                r.zoom_by(0.8);
            });
        }

        renderer
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    /// Set the image from base64-encoded data.
    pub fn set_image(&self, base64_data: &str, _mime_type: &str) {
        let decoded = match base64::engine::general_purpose::STANDARD.decode(base64_data) {
            Ok(bytes) => bytes,
            Err(_) => return,
        };

        let gbytes = glib::Bytes::from(&decoded);
        match gdk4::Texture::from_bytes(&gbytes) {
            Ok(texture) => {
                self.picture.set_paintable(Some(&texture));
                self.zoom_level.set(1.0);
                self.zoom_label.set_text("Fit");
                self.picture.set_can_shrink(true);
                self.picture.set_size_request(-1, -1);
            }
            Err(e) => {
                tracing::warn!("Failed to load image texture: {}", e);
            }
        }
    }

    fn zoom_by(&self, factor: f64) {
        let current = self.zoom_level.get();
        let new_level = (current * factor).clamp(0.1, 10.0);
        self.zoom_level.set(new_level);

        // Get intrinsic size from the paintable.
        if let Some(paintable) = self.picture.paintable() {
            let w = paintable.intrinsic_width();
            let h = paintable.intrinsic_height();
            if w > 0 && h > 0 {
                self.picture.set_can_shrink(false);
                self.picture.set_size_request(
                    (w as f64 * new_level) as i32,
                    (h as f64 * new_level) as i32,
                );
            }
        }

        self.zoom_label
            .set_text(&format!("{}%", (new_level * 100.0) as i32));
    }
}
