//! Existing native chat presentation for explicitly negotiated item speech.
mod protocol;
pub use protocol::line_text;
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
