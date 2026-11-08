//! Native cyclopedia character panel and admitted command mapping.
mod input;
mod protocol;
mod state;
mod view;
pub use input::{open_key, set_target};
pub use protocol::{Intent, apply_result, request};
pub use state::{CyclopediaCharacterPanel, HistoryRow};
pub use view::{
    CombatRow, combat_detail, combat_rows, history_rows, overview_bars, overview_cards, panel_text,
    store_blessings, store_cards,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/combat_modifiers.rs"]
mod combat_modifier_tests;

#[cfg(test)]
#[path = "tests/stamina.rs"]
mod stamina_tests;

#[cfg(test)]
#[path = "tests/gap_character_tabs.rs"]
mod gap_character_tabs;
