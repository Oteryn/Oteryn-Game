//! Server-owned world-event query panel (Ctrl+N).
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request};
pub use state::WorldEventPanel;
pub use view::panel_text;
#[cfg(test)]
mod tests;
