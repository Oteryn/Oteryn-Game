#[path = "state/memorial.rs"]
mod memorial;
pub use memorial::{MemorialGallery, MemorialOffer, MemorialTab};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::{Intent, Panel};
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/memorial_gallery.rs"]
mod memorial_gallery_tests;
#[cfg(test)]
mod tests;

#[path = "state/memorial_panel.rs"]
mod memorial_panel;
pub use memorial_panel::{MemorialAction, MemorialEntry, MemorialPanel};
