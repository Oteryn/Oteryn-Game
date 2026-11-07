//! Character-owned consent is independent of account cast settings.
pub mod protocol;
mod view;
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
