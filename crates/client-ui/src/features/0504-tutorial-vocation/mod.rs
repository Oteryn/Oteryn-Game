//! The server-owned Newhaven vocation offer and its five submission buttons.
mod protocol;
mod state;
mod view;
pub use protocol::{apply_result, request};
pub use state::VocationPanel;
pub use view::choices;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
