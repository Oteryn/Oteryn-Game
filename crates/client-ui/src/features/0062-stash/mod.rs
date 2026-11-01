//! Native stash panel state and commands.
pub use oteryn_client_runtime::stash::{StashCommand, StashResponse, stash};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::StashPanel;
pub use view::{panel_text, row_label};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
