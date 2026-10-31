//! First-vocation chooser opened by the server's Newhaven portal hint.
mod input;
mod protocol;
mod state;
mod view;
pub use input::choice_key;
pub use protocol::request;
pub use state::VocationPanel;
pub use view::panel_text;

#[cfg(test)]
#[path = "tests/choice.rs"]
mod tests;
