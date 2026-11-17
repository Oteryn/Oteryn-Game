//! Lens help: a catalog of help texts keyed by the client's panel identifiers.
mod state;
pub use state::LensHelp;
mod view;
pub use view::{KEYS, help_text, mode_text};
#[cfg(test)]
mod tests;
