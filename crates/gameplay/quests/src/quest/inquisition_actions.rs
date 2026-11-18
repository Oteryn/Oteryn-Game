//! Native counterparts for the Crystal startup bindings of actions 2002 and 2003.
use super::world_callbacks::{
    Callback, ItemOnTarget, NativeSelector, Position, Selector, Storage, TargetTransform,
};

const QUEST: &str = "oteryn:quest.the_inquisition_quest";
const HAUNTED: &str = "oteryn:quest-callback/inquisition-haunted-ruin";
const NORTH: &str = "oteryn:quest-callback/inquisition-vampire-north";
const SOUTH: &str = "oteryn:quest-callback/inquisition-vampire-south";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const MISSION03: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission03";
const MISSION04: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission04";
pub const HOLY_WATER: &str = "oteryn:item.tibia.i133";

pub(crate) fn callbacks() -> [Callback; 3] {
    [haunted(), vampire(NORTH, 31979), vampire(SOUTH, 31980)]
}

pub fn summon(callback: &Callback) -> Option<&'static str> {
    match callback.key.as_str() {
        HAUNTED if *callback == haunted() => Some("oteryn:creature.pirate_ghost"),
        NORTH if *callback == vampire(NORTH, 31979) => Some("oteryn:creature.the_count"),
        SOUTH if *callback == vampire(SOUTH, 31980) => Some("oteryn:creature.the_count"),
        _ => None,
    }
}

pub fn holy_water(callback: &Callback) -> bool {
    callback.key == HAUNTED && *callback == haunted()
}

fn set(track: &str, minimum: i64, maximum: i64, value: i64) -> Storage {
    Storage {
        track: track.into(),
        minimum,
        maximum,
        set: value,
        set_if: None,
        add: None,
        preserve: false,
        increment: false,
        at_least: false,
        read_only: false,
    }
}

fn source(key: &str, position: Position) -> Callback {
    let mut callback = super::source_world_objects::uneasy_crystal();
    callback.key = key.into();
    callback.quest = QUEST.into();
    callback.requires_world_admission = true;
    callback.selector = Selector {
        position: Some(position),
        position_aliases: vec![],
        ordinal: None,
        unique: None,
        action: None,
        appearance: None,
        native: None,
    };
    callback.storage.clear();
    callback.effects = Default::default();
    callback
}

fn vampire(key: &str, y: u16) -> Callback {
    let mut callback = source(
        key,
        Position {
            x: 32779,
            y,
            floor: -9,
        },
    );
    callback.storage = vec![set(MISSION03, -1, 4, 4), set(QUESTLINE, 8, 8, 9)];
    callback.sources = vec![
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_inquisition_quest/actions_vampire_hunt.lua:10-31".into(),
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:330-336".into(),
    ];
    callback
}

fn haunted() -> Callback {
    let position = Position {
        x: 32256,
        y: 32790,
        floor: -7,
    };
    let mut callback = source(HAUNTED, position);
    callback.selector.position = None;
    callback.selector.native = Some(NativeSelector {
        position,
        ordinal: 1,
        appearance: 7867,
    });
    callback.item_on_target = Some(ItemOnTarget {
        item: HOLY_WATER.into(),
        alternatives: vec![],
        ingredients: vec![],
        count: 1,
        reusable: false,
        selected_instance: true,
        on_step: false,
        replacement: None,
        target_replacement: None,
        target_remove: false,
    });
    callback.effects.target_transform = Some(TargetTransform {
        position: Position {
            x: 32260,
            y: 32791,
            floor: -7,
        },
        from: 7869,
        to: 7868,
        reset_after_ms: 10_000,
    });
    callback.storage = vec![set(MISSION04, -1, 2, 2), set(QUESTLINE, 12, 12, 13)];
    callback.sources = vec![
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_holy_water.lua:65-80".into(),
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:337-342".into(),
    ];
    callback
}
