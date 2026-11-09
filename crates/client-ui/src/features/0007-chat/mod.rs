//! Local speech and named private messages over the session chat owner.
#[path = "../../components/console_display.rs"]
mod display;
pub use display::{ConsoleDisplay, Metadata};
mod input;
mod protocol;
mod state;
mod view;
pub use crate::chat_public_channels::apply_result;
pub use input::open_key;
pub use protocol::{console_request, request};
pub use state::{
    ChatPanel, CommunicationList, CommunicationLists, ComposerMode, Console, ConsoleTab, Mode,
};
pub use view::{panel_text, write_log};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
