//! Exact source carried-item STEP branches, with no invented Quest progress.
use super::world_callbacks::{Callback, Position, Selector, Trigger};

#[derive(Clone, Copy, Debug)]
pub struct Recipe {
    pub key: &'static str,
    pub unique: u16,
    pub item: &'static str,
    pub count: u16,
    pub success: Position,
    pub missing: Position,
    pub source_line: u16,
}

// Crystal201189f movements_tomb_teleport.lua:3-44; repository source evidence
// docs/agents/evidence/OTV2-20261102-quests-5-20261102/the_ancient_tombs.json:2090-2365.
pub const RECIPES: &[Recipe] = &[
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-key-3950",
        unique: 3950,
        item: "oteryn:item.tibia.i3237",
        count: 1,
        success: Position {
            x: 33182,
            y: 32714,
            floor: -14,
        },
        missing: Position {
            x: 33231,
            y: 32705,
            floor: -8,
        },
        source_line: 3,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-key-3951",
        unique: 3951,
        item: "oteryn:item.tibia.i3238",
        count: 1,
        success: Position {
            x: 33174,
            y: 32934,
            floor: -15,
        },
        missing: Position {
            x: 33282,
            y: 32744,
            floor: -8,
        },
        source_line: 9,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-key-3952",
        unique: 3952,
        item: "oteryn:item.tibia.i3240",
        count: 1,
        success: Position {
            x: 33126,
            y: 32591,
            floor: -15,
        },
        missing: Position {
            x: 33250,
            y: 32832,
            floor: -8,
        },
        source_line: 15,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-key-3953",
        unique: 3953,
        item: "oteryn:item.tibia.i3239",
        count: 1,
        success: Position {
            x: 33145,
            y: 32665,
            floor: -15,
        },
        missing: Position {
            x: 33025,
            y: 32868,
            floor: -8,
        },
        source_line: 21,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-key-3954",
        unique: 3954,
        item: "oteryn:item.tibia.i3235",
        count: 1,
        success: Position {
            x: 33041,
            y: 32774,
            floor: -14,
        },
        missing: Position {
            x: 33133,
            y: 32642,
            floor: -8,
        },
        source_line: 27,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-key-3955",
        unique: 3955,
        item: "oteryn:item.tibia.i3241",
        count: 1,
        success: Position {
            x: 33349,
            y: 32827,
            floor: -14,
        },
        missing: Position {
            x: 33131,
            y: 32566,
            floor: -8,
        },
        source_line: 33,
    },
    Recipe {
        key: "oteryn:quest-callback/ancient-tombs-key-3956",
        unique: 3956,
        item: "oteryn:item.tibia.i3236",
        count: 1,
        success: Position {
            x: 33186,
            y: 33012,
            floor: -14,
        },
        missing: Position {
            x: 33206,
            y: 32592,
            floor: -8,
        },
        source_line: 39,
    },
];

pub fn recipe(key: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|recipe| recipe.key == key)
}

pub(super) fn callbacks() -> impl Iterator<Item = Callback> {
    RECIPES.iter().map(|recipe| Callback { admission: None, requires_premium: false,
    RECIPES.iter().map(|recipe| Callback { required_completed_quests: Vec::new(), admission: None,
world_change: None,
        actor_kind: Default::default(), opening_only: false, requires_world_admission: false,
        state_ops: Vec::new(), world_requirements: Vec::new(),
        key: recipe.key.into(), quest: "oteryn:quest.the_ancient_tombs_quest".into(),
        trigger: Trigger::StepIn,
        selector: Selector { native: None, position: None, unique: Some(recipe.unique), action: None, appearance: None },
        lever: None, reversible: false, passages: Vec::new(),
        teleport: Some(recipe.missing), facing: None, effects: Default::default(),
        storage: Vec::new(), minimum_level: None, character_requirements: None, experience: None, monk_reward: None,
        offerings: Vec::new(), item_on_target: None, return_on_failure: true,
        failure_message: None, success_message: None, native_rule: None,
        sources: vec![format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ancient_tombs/movements_tomb_teleport.lua:{}", recipe.source_line)],
    })
}

/// An exact native counterpart of a source UID omitted by the B3 import.
/// Matching includes the original native teleport; destination alone is never identity.
#[derive(Clone, Copy, Debug)]
pub struct NativeBinding {
    pub position: Position,
    pub ordinal: u8,
    pub appearance: u16,
    pub raw_destination: Position,
}

pub fn native_binding(key: &str) -> Option<NativeBinding> {
    let (position, raw_destination) = match recipe(key)?.unique {
        3950 => (
            Position {
                x: 33183,
                y: 32717,
                floor: -14,
            },
            Position {
                x: 33231,
                y: 32705,
                floor: -8,
            },
        ),
        3951 => (
            Position {
                x: 33175,
                y: 32937,
                floor: -15,
            },
            Position {
                x: 33250,
                y: 32832,
                floor: -8,
            },
        ),
        3952 => (
            Position {
                x: 33127,
                y: 32594,
                floor: -15,
            },
            Position {
                x: 33131,
                y: 32566,
                floor: -8,
            },
        ),
        3953 => (
            Position {
                x: 33146,
                y: 32668,
                floor: -15,
            },
            Position {
                x: 33206,
                y: 32592,
                floor: -8,
            },
        ),
        3954 => (
            Position {
                x: 33042,
                y: 32777,
                floor: -14,
            },
            Position {
                x: 33133,
                y: 32642,
                floor: -8,
            },
        ),
        3955 => (
            Position {
                x: 33350,
                y: 32830,
                floor: -14,
            },
            Position {
                x: 33282,
                y: 32744,
                floor: -8,
            },
        ),
        3956 => (
            Position {
                x: 33187,
                y: 33015,
                floor: -14,
            },
            Position {
                x: 33250,
                y: 32832,
                floor: -8,
            },
        ),
        _ => return None,
    };
    Some(NativeBinding {
        position,
        ordinal: 1,
        appearance: 1949,
        raw_destination,
    })
}
