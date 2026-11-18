//! Crystal 201189f8, movements_unlock_entrace.lua:52–112.

use super::world_callbacks::Position;

pub const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission05";
pub const ENTRANCE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/castle_entrance_stg";
pub const CHARGED_CRYSTAL: &str = "oteryn:item.tibia.i8225";

pub const SYMBOLS: [Position; 4] = [
    Position {
        x: 32928,
        y: 31460,
        floor: -7,
    },
    Position {
        x: 32941,
        y: 31510,
        floor: -7,
    },
    Position {
        x: 32966,
        y: 31498,
        floor: -6,
    },
    Position {
        x: 32946,
        y: 31499,
        floor: -6,
    },
];

const DESTINATIONS: [Position; 4] = [
    Position {
        x: 32951,
        y: 31479,
        floor: -6,
    },
    Position {
        x: 32954,
        y: 31479,
        floor: -6,
    },
    Position {
        x: 32951,
        y: 31482,
        floor: -6,
    },
    Position {
        x: 32954,
        y: 31482,
        floor: -6,
    },
];

#[derive(Clone, Copy)]
pub struct Participant {
    pub mission: i64,
    pub entrance: i64,
    pub has_charged_crystal: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    pub destination: Position,
    pub unlock: bool,
}

/// Each input is the top player on its corresponding symbol. A failed guard
/// invalidates the whole plan before the caller prepares any movement or write.
pub fn plan(entered: Position, players: [Option<Participant>; 4]) -> Option<[Admission; 4]> {
    if !SYMBOLS.contains(&entered) {
        return None;
    }
    let [Some(first), Some(second), Some(third), Some(fourth)] = players else {
        return None;
    };
    let players = [first, second, third, fourth];
    if players
        .iter()
        .any(|player| player.mission < 3 || (player.entrance != 1 && !player.has_charged_crystal))
    {
        return None;
    }
    Some(std::array::from_fn(|index| Admission {
        destination: DESTINATIONS[index],
        unlock: players[index].entrance != 1,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blood_brothers_castle_requires_all_four_players_and_checks_only_new_unlocks() {
        let ready = Participant {
            mission: 3,
            entrance: 0,
            has_charged_crystal: true,
        };
        let already = Participant {
            entrance: 1,
            has_charged_crystal: false,
            ..ready
        };
        let missing = Participant {
            has_charged_crystal: false,
            ..ready
        };
        let early = Participant {
            mission: 2,
            ..already
        };
        let group = [Some(ready), Some(already), Some(ready), Some(ready)];
        let admitted = plan(SYMBOLS[2], group);
        assert_eq!(
            admitted,
            Some(std::array::from_fn(|index| Admission {
                destination: DESTINATIONS[index],
                unlock: index != 1,
            }))
        );
        for invalid in [None, Some(missing), Some(early)] {
            let mut group = group;
            group[2] = invalid;
            assert_eq!(plan(SYMBOLS[2], group), None);
        }
        assert_eq!(plan(DESTINATIONS[2], group), None);
    }
}
