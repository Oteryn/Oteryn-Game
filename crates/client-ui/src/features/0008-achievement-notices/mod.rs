//! Live achievement notices for the current play session.
pub use oteryn_client_runtime::achievement_notices::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::reduce;
pub use state::AchievementNoticesPanel;
pub use view::panel_text;
#[cfg(test)]
mod tests;
