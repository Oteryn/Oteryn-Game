//! Capability 8: achievement advancement pop-up and bounded session log.
mod protocol;
mod state;
mod view;
pub use protocol::apply;
pub use state::{
    AchievementNotices, AchievementPopup, MAX_ADVANCEMENT_LOG_LINES, advancement_text,
};
pub use view::{panel_text, popup_text};

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
