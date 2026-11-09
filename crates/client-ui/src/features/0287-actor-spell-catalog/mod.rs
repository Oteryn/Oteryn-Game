//! oteryn-choice: native local view of confirmed server facts, with no gameplay authority.
mod input;
#[path = "state/metadata.rs"]
mod metadata;
pub use metadata::{VOCATIONS, professions};
pub mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use state::Panel;
pub use view::{detail_text, panel_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
