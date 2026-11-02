//! Server-owned Animus completion pages; the client never grants mastery.
pub use oteryn_client_runtime::animus_mastery::{
    AnimusMasteries, AnimusPage, MAX_ANIMUS_PAGE_KEYS, decode_page,
};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, request};
pub use state::AnimusPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests;
