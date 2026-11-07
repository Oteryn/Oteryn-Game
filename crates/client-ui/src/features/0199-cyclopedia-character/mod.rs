//! Native cyclopedia character panel and admitted command mapping.
mod input;
mod protocol;
mod state;
mod view;
pub use input::{open_key, set_target};
pub use protocol::{Intent, apply_result, request};
pub use state::CyclopediaCharacterPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/combat_modifiers.rs"]
mod combat_modifier_tests;

#[cfg(test)]
#[path = "tests/e2e_c20a.rs"]
mod e2e_c20a;

#[cfg(test)]
#[path = "tests/e2e_c15.rs"]
mod e2e_c15;

#[cfg(test)]
#[path = "tests/stamina.rs"]
mod stamina_tests;

#[cfg(test)]
#[path = "tests/gap_character_tabs.rs"]
mod gap_character_tabs;
