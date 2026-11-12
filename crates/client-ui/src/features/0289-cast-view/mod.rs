//! Native observer presentation; never initializes player inventory, quests or movement.
mod input;
pub mod protocol;
mod state;
mod view;
pub use input::ChatDraft;
pub use state::Panel;
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
