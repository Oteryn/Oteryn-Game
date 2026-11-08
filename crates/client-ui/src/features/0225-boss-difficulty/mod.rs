//! Native boss difficulty panel and admitted command mapping.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::BossDifficultyPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests;
