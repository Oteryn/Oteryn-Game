//! Crystal 201189f8, court_warlock_fight.lua:22-37,128-145,361-414.
use super::world_callbacks::{
    Callback, Encounter, EncounterPhase, EncounterPlayer, NativeSelector, Position, Selector,
    Storage,
};

pub const KEY: &str = "oteryn:quest-callback/order-of-the-stag-court-warlock";
pub const QUEST: &str = "oteryn:quest.the_order_of_the_stag_quest";
pub const TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
pub const LEVER: Position = Position {
    x: 33843,
    y: 32147,
    floor: -3,
};
pub const LEVER_APPEARANCE: u16 = 8913;
pub const CENTER: Position = Position {
    x: 33838,
    y: 32141,
    floor: -2,
};
pub const ROOM_MIN: Position = Position {
    x: 33830,
    y: 32133,
    floor: -2,
};
pub const ROOM_MAX: Position = Position {
    x: 33847,
    y: 32150,
    floor: -2,
};
pub const EXIT: Position = Position {
    x: 33842,
    y: 32141,
    floor: -3,
};
pub const INITIAL_WAVE_US: u64 = 3_000_000;
pub const TIMEOUT_US: u64 = 30 * 60 * 1_000_000;
pub const RAUBRITTERS_PER_WAVE: usize = 7;
pub const RAUBRITTERS: [&str; 4] = [
    "oteryn:creature.poisoned_raubritter",
    "oteryn:creature.frozen_raubritter",
    "oteryn:creature.raging_raubritter",
    "oteryn:creature.energised_raubritter",
];

pub(crate) fn callback() -> Callback {
    let mut callback = super::source_world_objects::uneasy_crystal();
    callback.key = KEY.into();
    callback.quest = QUEST.into();
    callback.selector = Selector {
        native: Some(NativeSelector {
            position: LEVER,
            ordinal: 1,
            appearance: LEVER_APPEARANCE,
        }),
        position: None,
        position_aliases: Vec::new(),
        ordinal: None,
        unique: None,
        action: None,
        appearance: None,
    };
    callback.storage = vec![Storage {
        track: TRACK.into(),
        minimum: 11,
        maximum: i64::MAX,
        set: 0,
        read_only: true,
        set_if: None,
        add: None,
        preserve: false,
        increment: false,
        at_least: false,
    }];
    callback.failure_message = Some("You are not prepared to face the Court Warlock yet.".into());
    callback.effects = Default::default();
    callback.effects.encounter = Some(Encounter {
        minimum_level: 0,
        players: (32142..=32146)
            .map(|y| EncounterPlayer {
                from: Position {
                    x: 33842,
                    y,
                    floor: -3,
                },
                to: CENTER,
            })
            .collect(),
        room_min: ROOM_MIN,
        room_max: ROOM_MAX,
        monsters: Vec::new(),
        timing: None,
        phase: Some(EncounterPhase::CourtWarlock),
    });
    callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_order_of_the_stag/court_warlock_fight.lua:361-414".into()];
    callback
}

pub fn admitted(candidate: &Callback) -> bool {
    candidate.key == KEY && *candidate == callback()
}
