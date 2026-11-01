//! Server-confirmed Prey slots and their complete wire action set.
pub use oteryn_client_runtime::prey::{CommandStatus, PreyCommand, PreyResponse, prey};
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::PreyPanel;
pub use view::{panel_text, slot_text};

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
