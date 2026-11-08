//! Native cyclopedia character panel and admitted command mapping.
#[path = "protocol/achievements.rs"]
mod achievements;
#[path = "state/gallery.rs"]
mod gallery;
pub use gallery::Gallery;
#[path = "state/items.rs"]
mod items;
pub use items::ItemSummary;
mod input;
mod protocol;
mod state;
mod view;
pub use achievements::{next_request as achievement_next_request, request as achievement_request};
pub use input::{open_key, set_target};
pub use protocol::{Intent, apply_result, request};
pub use state::{BadgeFilter, BadgeRow, CyclopediaCharacterPanel, HistoryRow};
pub use view::{
    CombatRow, badge_rows, badge_text, combat_detail, combat_rows, history_rows, misc_bars,
    misc_cards, overview_bars, overview_cards, panel_text, store_blessings, store_cards,
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
