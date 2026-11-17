//! Spell action-bar input and vitals rendering.
#[path = "view/assign_dialog.rs"]
mod assign_dialog;
mod input;
mod state;
pub use assign_dialog::{SHOW_ONLY_KNOWN_LABEL, TARGET_OPTIONS_LABEL, TargetOption, dialog_spells};
pub use state::ActionBarPanel;
#[path = "protocol.rs"]
pub mod movement;
mod view;
pub use input::*;
pub use view::*;
