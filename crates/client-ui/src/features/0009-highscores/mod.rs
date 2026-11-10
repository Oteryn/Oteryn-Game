//! Live World highscores: server ranks and snapshot tokens remain authoritative.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request};
pub use state::{Action, HighscoresPanel};
pub use view::{panel_text, parse_world, world_text};
// Categories 13 (Loyalty) and 15 (Taint) are reserved; Game filters preserve explicit wire IDs.
pub const CATEGORY_IDS: [u32; 15] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 16, 17];
pub const CATEGORIES: [&str; 15] = [
    "Experience",
    "Magic level",
    "Fist",
    "Club",
    "Sword",
    "Axe",
    "Distance",
    "Shielding",
    "Fishing",
    "Achievement points",
    "Charm points",
    "Boss points",
    // docs/reference/tibia-manual/combat.md:262; both lists use the existing native Highscores controls.
    "Tibiadrome lifetime",
    "Phosphorus Record",
    "Tibiadrome current rotation",
];
pub const VOCATIONS: [&str; 6] = ["All", "Knight", "Paladin", "Sorcerer", "Druid", "Monk"];

/// Convert a native combo selection to the registered category ID, including gaps.
pub fn category_for_selection(index: usize) -> Option<u32> {
    CATEGORY_IDS.get(index).copied()
}
/// Restore the native combo selection from a registered category ID.
pub fn category_selection(category: u32) -> Option<usize> {
    CATEGORY_IDS.iter().position(|id| *id == category)
}

#[cfg(test)]
#[path = "tests/e2e_s11.rs"]
mod e2e_s11;
