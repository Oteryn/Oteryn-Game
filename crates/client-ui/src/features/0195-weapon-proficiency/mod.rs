//! Native weapon proficiency panel and admitted command mapping.
mod input;
#[path = "view/presentation.rs"]
mod presentation;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use presentation::{mastery_target, progress_level, progress_percent};
pub use protocol::{Intent, apply_result, request, tick};
pub use state::WeaponProficiencyPanel;
pub use view::{HudView, hud_view, panel_text};
#[cfg(test)]
#[path = "view/selection_tests.rs"]
mod selection_tests;
