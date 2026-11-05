pub mod protocol;
mod view;
pub use view::{ambient_rgb, clock_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
