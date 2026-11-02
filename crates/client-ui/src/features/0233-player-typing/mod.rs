pub use oteryn_client_runtime::player_typing::{PlayerTypingCommand, PlayerTypingResponse};
mod input;
pub mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request};
pub use state::{PlayerTypingPanel, TypingIndicator};
pub use view::panel_text;

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
