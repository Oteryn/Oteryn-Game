//! Private channels over the existing chat session and native panel.
mod input;
mod protocol;
mod state;
mod view;
pub use input::{Intent, request};
pub use protocol::{apply_result, replace_state};
pub use state::ChatPanel;
pub use view::{leave_label, panel_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
