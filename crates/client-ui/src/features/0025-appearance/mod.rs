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
mod tests;
