//! Projection of the admitted served Channel; no directory or switch command.
pub use oteryn_session::channel_list::{ServedChannel, decode};
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
