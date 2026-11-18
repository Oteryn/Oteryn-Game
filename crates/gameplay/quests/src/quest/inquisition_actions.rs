//! Native counterparts for the Crystal startup bindings of actions 2002 and 2003.
use super::world_callbacks::{
    Callback, ItemOnTarget, NativeSelector, Offering, Position, Selector, Storage, TargetTransform,
};

const QUEST: &str = "oteryn:quest.the_inquisition_quest";
const HAUNTED: &str = "oteryn:quest-callback/inquisition-haunted-ruin";
const NORTH: &str = "oteryn:quest-callback/inquisition-vampire-north";
const SOUTH: &str = "oteryn:quest-callback/inquisition-vampire-south";
const NORTH_WEAK: &str = "oteryn:quest-callback/inquisition-vampire-north-weakened";
const SOUTH_WEAK: &str = "oteryn:quest-callback/inquisition-vampire-south-weakened";
pub const SCRIPTURE: &str = "oteryn:item.tibia.i3083";
pub const ALTARS: [Position; 3] = [
    Position {
        x: 32777,
        y: 31982,
        floor: -9,
    },
    Position {
        x: 32779,
        y: 31977,
        floor: -9,
    },
    Position {
        x: 32781,
        y: 31982,
        floor: -9,
    },
];
const NEXUS_FIRST: &str = "oteryn:quest-callback/inquisition-nexus-first";
const NEXUS_REPEAT: &str = "oteryn:quest-callback/inquisition-nexus-repeat";
const MISSION07: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission07";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
const MISSION03: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission03";
const MISSION04: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission04";
pub const HOLY_WATER: &str = "oteryn:item.tibia.i133";

pub(crate) fn callbacks() -> [Callback; 7] {
    [
        haunted(),
        nexus(false),
        nexus(true),
        vampire(NORTH, 31979, false),
        vampire(NORTH_WEAK, 31979, true),
        vampire(SOUTH, 31980, false),
        vampire(SOUTH_WEAK, 31980, true),
    ]
}

/// The ordinary and weakened callbacks share the object, but the Channel
/// selects exactly one branch from the three current altar observations.
pub fn vampire_branch(callback: &Callback) -> Option<bool> {
    let (y, weakened) = match callback.key.as_str() {
        NORTH => (31979, false),
        NORTH_WEAK => (31979, true),
        SOUTH => (31980, false),
        SOUTH_WEAK => (31980, true),
        _ => return None,
    };
    (*callback == vampire(&callback.key, y, weakened)).then_some(weakened)
}

pub(crate) fn exclusive(left: &Callback, right: &Callback) -> bool {
    left.selector == right.selector
        && vampire_branch(left)
            .zip(vampire_branch(right))
            .is_some_and(|(a, b)| a != b)
}

pub fn summon(callback: &Callback) -> Option<&'static str> {
    if callback.key == HAUNTED && *callback == haunted() {
        return Some("oteryn:creature.pirate_ghost");
    }
    vampire_branch(callback).map(|weakened| {
        if weakened {
            "oteryn:creature.the_weakened_count"
        } else {
            "oteryn:creature.the_count"
        }
    })
}

pub fn holy_water(callback: &Callback) -> bool {
    (callback.key == HAUNTED && *callback == haunted()) || nexus_claim(callback)
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

fn vampire(key: &str, y: u16, weakened: bool) -> Callback {
    let mut callback = source(
        key,
        Position {
            x: 32779,
            y,
            floor: -9,
        },
    );
    if weakened {
        callback.offerings = ALTARS
            .into_iter()
            .map(|position| Offering {
                position,
                item: SCRIPTURE.into(),
                count: 1,
            })
            .collect();
    }
    callback.storage = vec![set(MISSION03, 0, 6, 4), set(QUESTLINE, 8, 8, 9)];
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
        ordinal: 0,
        appearance: 7867,
    });
    callback.item_on_target = Some(water());
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
    callback.storage = vec![set(MISSION04, 0, 3, 2), set(QUESTLINE, 12, 12, 13)];
    callback.sources = vec![
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_holy_water.lua:65-80".into(),
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:337-342".into(),
    ];
    callback
}

fn water() -> ItemOnTarget {
    ItemOnTarget {
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
    }
}

pub fn nexus_claim(callback: &Callback) -> bool {
    match callback.key.as_str() {
        NEXUS_FIRST => *callback == nexus(false),
        NEXUS_REPEAT => *callback == nexus(true),
        _ => false,
    }
}

fn nexus(repeat: bool) -> Callback {
    let position = Position {
        x: 33115,
        y: 31702,
        floor: -12,
    };
    let mut callback = source(if repeat { NEXUS_REPEAT } else { NEXUS_FIRST }, position);
    callback.selector.position = None;
    callback.selector.native = Some(NativeSelector {
        position,
        ordinal: 3,
        appearance: 7925,
    });
    callback.item_on_target = Some(water());
    callback.storage = if repeat {
        vec![set(QUESTLINE, 22, 25, 22)]
    } else {
        vec![set(MISSION07, 0, 5, 2), set(QUESTLINE, 0, 21, 22)]
    };
    callback.sources = vec![
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_holy_water.lua:117-135".into(),
    ];
    callback
}
