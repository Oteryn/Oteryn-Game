//! Local speech and named private messages over the session chat owner.
mod input;
mod protocol;
mod state;
mod view;
pub use crate::chat_public_channels::apply_result;
pub use input::open_key;
pub use protocol::request;
pub use state::{
    ChatPanel, CommunicationList, CommunicationLists, ComposerMode, Console, ConsoleTab, Mode,
};
pub use view::{panel_text, write_log};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
