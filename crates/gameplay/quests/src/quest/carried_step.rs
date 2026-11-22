//! Exact source carried-item STEP branches, with no invented Quest progress.
use super::world_callbacks::{Callback, NativeSelector, Position, Selector, Trigger};

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
    RECIPES.iter().map(|recipe| Callback {
        use_range: 1,
        disguise: None,
        state_operations: Vec::new(),
        completes: false,
        premium_required: false,
        present: Vec::new(),
        required_travel: Vec::new(),
        failure_teleport: None, track_initials: Default::default(), required_completed_quests: Vec::new(), admission: None, requires_premium: false,
world_change: None,
        actor_kind: Default::default(), opening_only: false, requires_world_admission: false,
        state_ops: Vec::new(), world_requirements: Vec::new(),
        key: recipe.key.into(), quest: "oteryn:quest.the_ancient_tombs_quest".into(),
        trigger: Trigger::StepIn,
        selector: Selector { position_aliases: Vec::new(),
 ordinal: None,
 native: boss_forcefield(recipe.unique), position: None, unique: Some(recipe.unique), action: None, appearance: None },
        lever: None, reversible: false, passages: Vec::new(),
        teleport: Some(recipe.missing), facing: None, effects: Default::default(),
        storage: Vec::new(), minimum_level: None, character_requirements: None, experience: None, monk_reward: None,
        offerings: Vec::new(), item_on_target: None, return_on_failure: true,
        failure_message: None, success_message: None, native_rule: None,
        sources: vec![format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_ancient_tombs/movements_tomb_teleport.lua:{}", recipe.source_line)],
achievements: Vec::new(),
activation: None,
item_reward: None,
remote_teleport: None,
requirements: None,
reward_bundle: false,
})
}

// The Global spoiler calls for the forcefield in the boss room. Crystal startup
// item.lua:4437-4463 identifies the corresponding bare B3 objects.
fn boss_forcefield(unique: u16) -> Option<NativeSelector> {
    let (x, y, floor) = match unique {
        3950 => (33174, 32694, -14),
        3951 => (33396, 32852, -14),
        3952 => (33191, 32959, -15),
        3953 => (33195, 33002, -14),
        3954 => (33073, 32781, -14),
        3955 => (33103, 32590, -15),
        3956 => (33116, 32656, -15),
        _ => return None,
    };
    Some(NativeSelector {
        position: Position { x, y, floor },
        ordinal: 1,
        appearance: 1949,
    })
}

/// Retained compatibility counterparts at two original native teleport exits.
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

/// A coal basin's retained source tile, occupied blue flame and inner landing.
/// Crystal 201189f8, startup/tables/item.lua:4500-4531 and
/// the_ancient_tombs/movements_all_teleports_tombs_coal_basin.lua:1-25.
#[derive(Clone, Copy)]
pub struct CoalBasin {
    pub unique: u16,
    pub basin: (u16, u16, i8),
    pub flame: (i32, i32, i16),
    pub landing: (i32, i32, i16),
}

pub const COAL_BASINS: &[CoalBasin] = &[
    CoalBasin {
        unique: 9033,
        basin: (33098, 32816, -13),
        flame: (33097, 32816, 13),
        landing: (33093, 32824, 13),
    },
    CoalBasin {
        unique: 9034,
        basin: (33293, 32741, -13),
        flame: (33293, 32742, 13),
        landing: (33299, 32742, 13),
    },
    CoalBasin {
        unique: 9035,
        basin: (33073, 32589, -13),
        flame: (33073, 32590, 13),
        landing: (33080, 32588, 13),
    },
    CoalBasin {
        unique: 9036,
        basin: (33240, 32855, -13),
        flame: (33240, 32856, 13),
        landing: (33246, 32850, 13),
    },
    CoalBasin {
        unique: 9037,
        basin: (33276, 32552, -14),
        flame: (33276, 32553, 14),
        landing: (33271, 32553, 14),
    },
    CoalBasin {
        unique: 9038,
        basin: (33233, 32692, -13),
        flame: (33234, 32692, 13),
        landing: (33234, 32687, 13),
    },
    CoalBasin {
        unique: 9039,
        basin: (33135, 32682, -12),
        flame: (33135, 32683, 12),
        landing: (33130, 32683, 12),
    },
    CoalBasin {
        unique: 9040,
        basin: (33161, 32831, -10),
        flame: (33162, 32831, 10),
        landing: (33158, 32832, 10),
    },
];

pub const SCARAB_COIN: &str = "oteryn:item.tibia.i3042";
