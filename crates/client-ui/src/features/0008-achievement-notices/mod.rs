//! Live achievement notices for the current play session.
pub use oteryn_client_runtime::achievement_notices::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::reduce;
pub use state::AchievementNoticesPanel;
pub use view::notices_panel_text;
#[cfg(test)]
mod tests;

pub use protocol::apply;
pub use state::{
    AchievementNotices, AchievementPopup, MAX_ADVANCEMENT_LOG_LINES, advancement_text,
};
pub use view::{panel_text, popup_text};
#[cfg(test)]
#[path = "tests/state.rs"]
mod popup_tests;
