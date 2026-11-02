//! Capability-259 public-room state and controls over the existing chat owner.
mod input;
mod protocol;
mod state;
mod view;

pub use input::open_key;
pub use protocol::{Intent, apply_result, replace_state, request};
pub use state::ChatPanel;
pub use view::{line_text, panel_text, room_name};

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
