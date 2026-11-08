//! Local speech and named private messages over the session chat owner.
#[path = "state/console.rs"]
mod console;
mod input;
mod protocol;
mod state;
mod view;
pub use crate::chat_public_channels::apply_result;
pub use console::{ComposerMode, Console, ConsoleFilter, ConsoleTab};
pub use input::open_key;
pub use protocol::request;
pub use state::{ChatPanel, Mode};
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
