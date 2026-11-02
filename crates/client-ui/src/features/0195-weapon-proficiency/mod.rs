//! Native weapon proficiency panel and admitted command mapping.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{Intent, apply_result, request};
pub use state::WeaponProficiencyPanel;
pub use view::panel_text;
