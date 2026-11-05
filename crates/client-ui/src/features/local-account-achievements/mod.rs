//! Account achievements earned by the admitted account.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::AchievementsPanel;
pub use view::panel_text;
#[cfg(test)]
mod tests;
