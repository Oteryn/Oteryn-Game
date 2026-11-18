//! The source Ferumbras staging pads and opening roster.
use super::world_callbacks::{
    Callback, Encounter, EncounterMonster, EncounterPlayer, NativeSelector, Position, Selector,
};

pub const KEY: &str = "oteryn:quest-callback/ferumbras-final-lever";

const fn room(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

pub(crate) fn callback() -> Callback {
    let mut callback = super::source_world_objects::uneasy_crystal();
    callback.key = KEY.into();
    callback.quest = "oteryn:quest.ferumbras_ascension_quest".into();
    callback.selector = Selector {
        position: None,
        position_aliases: vec![],
        ordinal: None,
        unique: None,
        action: None,
        appearance: None,
        native: Some(NativeSelector {
            position: room(33270, 31476),
            ordinal: 2,
            appearance: 8911,
        }),
    };
    callback.lever = Some([8911, 8912]);
    callback.reversible = true;
    callback.storage.clear();
    callback.effects = Default::default();
    callback.effects.encounter = Some(Encounter {
        minimum_level: 0,
        players: (33269..=33271)
            .flat_map(|x| {
                (31477..=31481).map(move |y| EncounterPlayer {
                    from: room(x, y),
                    to: room(33392, 31479),
                })
            })
            .collect(),
        monsters: [
            (33392, 31473, "oteryn:creature.ascending_ferumbras"),
            (33385, 31466, "oteryn:creature.enthralled_demon"),
            (33396, 31466, "oteryn:creature.enthralled_demon"),
            (33392, 31480, "oteryn:creature.enthralled_demon"),
            (33392, 31468, "oteryn:creature.enthralled_demon"),
        ]
        .into_iter()
        .map(|(x, y, creature)| EncounterMonster {
            position: room(x, y),
            creature: creature.into(),
        })
        .collect(),
        room_min: room(33379, 31460),
        room_max: room(33405, 31485),
        timing: None,
        phase: None,
    });
    callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/ferumbras_ascension/actions_ferumbras_lever.lua:1-63".into()];
    callback
}

pub fn admitted(callback: &Callback) -> bool {
    callback.key == KEY && *callback == self::callback()
}
