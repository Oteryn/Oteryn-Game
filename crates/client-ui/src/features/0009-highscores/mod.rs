//! Live World highscores: server ranks and snapshot tokens remain authoritative.
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::{apply_result, request};
pub use state::{Action, HighscoresPanel};
pub use view::panel_text;
pub const CATEGORIES: [&str; 13] = [
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
    "Tibiadrome levels",
];
pub const VOCATIONS: [&str; 6] = ["All", "Knight", "Paladin", "Sorcerer", "Druid", "Monk"];

#[cfg(test)]
mod srx3_tests {
    use super::*;
    #[test]
    fn srx3_drome_lifetime_highscores_has_native_category_filter() {
        let mut panel = HighscoresPanel {
            selected: true,
            ..Default::default()
        };
        let request = request(
            &mut panel,
            Action::Filter {
                category: 13,
                vocation: 0,
            },
        );
        assert!(
            request.is_some(),
            "A6 cumulative Drome levels must have a native filter"
        );
        assert_eq!(CATEGORIES.get(12), Some(&"Tibiadrome levels"));
    }
}
