//! Live World highscores: server ranks and snapshot tokens remain authoritative.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request};
pub use state::{Action, HighscoresPanel};
pub use view::panel_text;
pub const CATEGORIES: [&str; 11] = [
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
];
pub const VOCATIONS: [&str; 6] = ["All", "Knight", "Paladin", "Sorcerer", "Druid", "Monk"];
