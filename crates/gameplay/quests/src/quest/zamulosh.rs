//! Crystal 201189f8, movements_lovely_monsters.lua:1-39.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestTrack, QuestTransition};

pub const QUEST: &str = "oteryn:quest.ferumbras_ascension_quest";
pub const TRACK: &str = "oteryn:quest-progress/ferumbras-ascension/zamulosh-teleports";
pub const TRANSITION: &str = "oteryn:quest-transition/ferumbras-ascension/lovely-creature";
pub const PAIRS: [(u16, &str, [i32; 5]); 8] = [
    (
        34315,
        "oteryn:creature.lovely_frazzlemaw",
        [33609, 32657, 33627, 32675, -12],
    ),
    (
        34316,
        "oteryn:creature.lovely_deer",
        [33632, 32657, 33650, 32675, -12],
    ),
    (
        34317,
        "oteryn:creature.lovely_rotworm",
        [33609, 32675, 33627, 32693, -12],
    ),
    (
        34318,
        "oteryn:creature.lovely_scorpion",
        [33632, 32675, 33650, 32693, -12],
    ),
    (
        34319,
        "oteryn:creature.lovely_snake",
        [33609, 32693, 33627, 32711, -12],
    ),
    (
        34320,
        "oteryn:creature.lovely_polar_bear",
        [33632, 32693, 33650, 32711, -12],
    ),
    (
        34321,
        "oteryn:creature.lovely_souleater",
        [33609, 32711, 33627, 32729, -12],
    ),
    (
        34322,
        "oteryn:creature.lovely_yielothax",
        [33632, 32711, 33650, 32729, -12],
    ),
];

pub(super) fn track() -> QuestTrack {
    QuestTrack {
        key: TRACK.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: i64::MAX,
    }
}

pub(super) fn transition() -> QuestTransition {
    QuestTransition {
        key: TRANSITION.into(),
        quest: QUEST.into(),
        effects: vec![QuestEffect {
            track: TRACK.into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Add(1),
        }],
        completes: false,
        experience: None,
        world_change: None,
    }
}

pub struct Habitat {
    pub key: &'static str,
    pub origin: (u16, u16),
    pub counterpart_x: u16,
    pub creature: &'static str,
    pub spawn: (u16, u16),
}

pub const HABITATS: [Habitat; 8] = [
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-corrupted",
        origin: (33611, 32712),
        counterpart_x: 33634,
        creature: "oteryn:creature.lovely_yielothax",
        spawn: (33619, 32722),
    },
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-desert",
        origin: (33634, 32676),
        counterpart_x: 33611,
        creature: "oteryn:creature.lovely_rotworm",
        spawn: (33641, 32684),
    },
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-dimension",
        origin: (33634, 32712),
        counterpart_x: 33611,
        creature: "oteryn:creature.lovely_souleater",
        spawn: (33642, 32722),
    },
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-grass",
        origin: (33634, 32658),
        counterpart_x: 33611,
        creature: "oteryn:creature.lovely_frazzlemaw",
        spawn: (33642, 32666),
    },
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-ice",
        origin: (33634, 32694),
        counterpart_x: 33611,
        creature: "oteryn:creature.lovely_snake",
        spawn: (33642, 32702),
    },
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-mushroom",
        origin: (33611, 32676),
        counterpart_x: 33634,
        creature: "oteryn:creature.lovely_scorpion",
        spawn: (33617, 32684),
    },
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-roshamuul",
        origin: (33611, 32658),
        counterpart_x: 33634,
        creature: "oteryn:creature.lovely_deer",
        spawn: (33619, 32666),
    },
    Habitat {
        key: "oteryn:quest-callback/zamulosh-habitat-venom",
        origin: (33611, 32694),
        counterpart_x: 33634,
        creature: "oteryn:creature.lovely_polar_bear",
        spawn: (33617, 32702),
    },
];
