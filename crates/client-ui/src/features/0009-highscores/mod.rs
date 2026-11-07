//! Live World highscores: server ranks and snapshot tokens remain authoritative.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request};
pub use state::{Action, HighscoresPanel};
pub use view::panel_text;
pub const CATEGORIES: [&str; 14] = [
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
    "Tibiadrome current rotation",
];
pub const VOCATIONS: [&str; 6] = ["All", "Knight", "Paladin", "Sorcerer", "Druid", "Monk"];
