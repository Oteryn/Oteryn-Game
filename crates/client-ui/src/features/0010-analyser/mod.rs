//! oteryn-choice: native local view of confirmed server facts, with no gameplay authority.
mod input;
#[path = "state/party.rs"]
pub mod party;
pub mod protocol;
mod state;
mod view;
pub use input::{open_key, select_tool, set_item_price, set_xp_goal};
pub use state::{Panel, Tool};
pub use view::{panel_text, session_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
