pub use oteryn_client_runtime::player_typing::{
    PlayerTypingCommand, PlayerTypingOrigin, PlayerTypingResponse,
};
mod input;
pub mod protocol;
mod state;
mod view;
pub use input::{composer, open_key, tick};
pub use protocol::{apply_result, request};
pub use state::{PlayerTypingPanel, TypingIndicator};
pub use view::panel_text;

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
