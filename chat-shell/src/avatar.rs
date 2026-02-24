//! Reusable avatar helper — circular logo for assistant messages.

use gtk4::prelude::*;

/// Embedded Levsha logo (compiled into the binary).
const LOGO_BYTES: &[u8] = include_bytes!("../../assets/logo2.png");

/// Create a circular avatar with the Levsha logo at the given size.
pub fn create_avatar(size: i32) -> Option<gtk4::Frame> {
    let bytes = glib::Bytes::from_static(LOGO_BYTES);
    let texture = gdk4::Texture::from_bytes(&bytes).ok()?;

    let picture = gtk4::Picture::for_paintable(&texture);
    picture.set_size_request(size, size);
    picture.set_content_fit(gtk4::ContentFit::Cover);

    let frame = gtk4::Frame::new(None);
    frame.add_css_class("avatar-circle");
    frame.set_child(Some(&picture));
    frame.set_size_request(size, size);
    frame.set_overflow(gtk4::Overflow::Hidden);

    Some(frame)
}
