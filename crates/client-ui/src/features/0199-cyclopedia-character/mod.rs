//! Native cyclopedia character panel and admitted command mapping.
#[path = "protocol/achievements.rs"]
mod achievements;
#[path = "view/augments.rs"]
mod augment_view;
#[path = "protocol/augments.rs"]
mod augments;
mod input;
pub use augment_view::{AugmentRow, augment_heading, augment_rows};
pub use augments::{AugmentIntent, AugmentPanel, request as augment_request};
#[path = "view/profile.rs"]
mod profile;
mod protocol;
mod state;
mod view;
pub use achievements::{next_request as achievement_next_request, request as achievement_request};
pub use input::{blessings_key, open_key, set_target};
pub use profile::{ProfileRow, profile_heading, profile_rows};
pub use protocol::{BlessingsPanel, Intent, apply_result, request, request_blessings};
pub use state::{BadgeFilter, BadgeRow, CyclopediaCharacterPanel, HistoryRow};
pub use view::{
    CombatRow, badge_rows, badge_text, blessings_text, combat_detail, combat_rows, history_rows,
    misc_bars, misc_cards, overview_bars, overview_cards, panel_text, store_blessings, store_cards,
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
#[cfg(test)]
#[path = "tests/profile.rs"]
mod profile_tests;
