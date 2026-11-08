//! oteryn-choice: native local view of confirmed server facts, with no gameplay authority.
#[path = "state/bosses.rs"]
pub mod bosses;
mod input;
#[path = "state/kills.rs"]
pub mod kills;
#[path = "state/party.rs"]
pub mod party;
#[path = "state/party_import.rs"]
pub mod party_import;
pub mod protocol;
mod state;
#[path = "state/tracking.rs"]
pub mod tracking;
mod view;
pub use input::{open_key, select_tool, set_item_price, set_xp_goal};
pub use state::{Panel, Tool};
pub use view::{panel_text, session_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/loot_tracker.rs"]
mod loot_tracker_tests;

#[cfg(test)]
#[path = "tests/bosses.rs"]
mod boss_tracker_tests;
#[cfg(test)]
#[path = "tests/kills.rs"]
mod kill_tracker_tests;
#[cfg(test)]
#[path = "tests/party_import.rs"]
mod party_import_tests;
