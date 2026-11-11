//! Native daily boost panel and admitted command mapping.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::DailyBoostPanel;
pub use view::{SpotlightCard, panel_text, spotlight_cards, xp_boost_lines};
