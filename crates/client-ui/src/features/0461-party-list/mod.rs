//! Negotiated Party List display shares the existing party panel.
mod protocol;
mod view;
pub use protocol::apply_result;
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
