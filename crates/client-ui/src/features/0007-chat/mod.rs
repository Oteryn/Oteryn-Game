//! Local speech and named private messages over the session chat owner.
mod input;
mod protocol;
mod state;
mod view;
pub use crate::chat_public_channels::apply_result;
pub use input::open_key;
pub use protocol::request;
pub use state::{ChatPanel, ComposerMode, Console, ConsoleTab, Mode};
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
