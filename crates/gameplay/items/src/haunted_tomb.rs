//! Crystal 201189f8, actions_sacrophagus_use.lua:1–76 and items.xml:55553–55558.

pub const SARCOPHAGUS: u32 = 29349;
pub const ACTION_ID: u16 = 23104;
pub const KEY: u32 = 29351;
pub const KEY_LIFETIME_MS: u32 = 1_800_000;
pub const COOLDOWN_MS: u32 = 30_000;
pub const TOMB_TRACK: &str =
    "oteryn:quest-progress/quest/u12_00/the_dream_courts/haunted_house/tomb";
pub const MUMMY: &str = "oteryn:creature.mummy";

const PLACEMENTS: [(u16, u16); 13] = [
    (33051, 32309),
    (33054, 32335),
    (33055, 32297),
    (33058, 32323),
    (33081, 32349),
    (33081, 32355),
    (33083, 32338),
    (33091, 32301),
    (33091, 32319),
    (33092, 32347),
    (33093, 32338),
    (33093, 32356),
    (33103, 32344),
];

pub fn is_source(item: u32, action: Option<u16>, x: u16, y: u16, floor: i8) -> bool {
    item == SARCOPHAGUS
        && action == Some(ACTION_ID)
        && floor == -8
        && PLACEMENTS.binary_search(&(x, y)).is_ok()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Curse,
    Key,
}

pub fn select(questline: i64, tomb: i64, roll: u32) -> Option<Outcome> {
    if questline != 1 || tomb >= 1 {
        return None;
    }
    match roll {
        1..=6 => Some(Outcome::Curse),
        7..=10 => Some(Outcome::Key),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haunted_tomb_source_gate_and_outcomes() {
        for (x, y) in PLACEMENTS {
            assert!(is_source(SARCOPHAGUS, Some(ACTION_ID), x, y, -8));
            assert!(!is_source(SARCOPHAGUS, Some(ACTION_ID), x, y, -7));
            assert!(!is_source(SARCOPHAGUS, None, x, y, -8));
        }
        assert!(!is_source(SARCOPHAGUS, Some(ACTION_ID), 33080, 32355, -8));
        assert_eq!(select(1, -1, 1), Some(Outcome::Curse));
        assert_eq!(select(1, -1, 6), Some(Outcome::Curse));
        assert_eq!(select(1, -1, 7), Some(Outcome::Key));
        assert_eq!(select(1, 0, 10), Some(Outcome::Key));
        for input in [(0, 0, 9), (1, 1, 9), (2, 0, 9), (1, 0, 0), (1, 0, 11)] {
            assert_eq!(select(input.0, input.1, input.2), None);
        }
    }
}
