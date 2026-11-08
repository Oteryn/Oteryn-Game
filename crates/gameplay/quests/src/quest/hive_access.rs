//! Hive Born's personal scent permit uses the existing QuestState timestamp owner.
//! Wiki56902@930222:20 (c8dd90a99df4f596c9db4b3231b19e9d3c1ad70b306d37f9e4b37d90fb207d0b):
//! fifty Favour Points buy seven days. Eastern access is stage2, western stage3.
//! Gate positions/appearances and outer-route stairs are native B3. Per-floor west/east
//! gate families and entry sides are explicit native choices, not donor Lua conditions.
//! In particular, floor5's western gate has x33534: a global x threshold is incorrect.
//! Outward crossings remain available after the personal scent or World stage expires.
//! Crystal hive_gates.lua:3-24 SHA4bb33cb5df62c5619cea16effdc571f1765e60065866813c6228ab273218834b
//! supplies the per-appearance axis and opposite-side destination.
use super::world_callbacks::{Callback, Position, Selector, Trigger};
pub const FAVOUR: &str = "oteryn:quest-progress/hive/favour";
pub const PURCHASED: &str = "oteryn:quest-progress/hive/scent-purchased-at";
pub const BUY: &str = "oteryn:quest-transition/hive-native-scent-purchase";
pub const DURATION_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const fn favour_reward(task: u8) -> i64 {
    if task < 4 { 1 } else { 2 }
}
pub fn permit_valid(purchased: i64, now: i64) -> bool {
    purchased > 0
        && now >= purchased
        && i128::from(now) - i128::from(purchased) < i128::from(DURATION_SECONDS)
}
#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub appearance: u16,
    pub position: Position,
    pub minimum_stage: u8,
    pub outer_route: Position,
}
pub const GATES: [Gate; 48] = [
    Gate {
        appearance: 13290,
        position: Position {
            x: 33495,
            y: 31197,
            floor: -1,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33501,
            y: 31197,
            floor: -1,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33495,
            y: 31198,
            floor: -1,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33501,
            y: 31197,
            floor: -1,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33495,
            y: 31199,
            floor: -1,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33501,
            y: 31197,
            floor: -1,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33598,
            y: 31227,
            floor: -1,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33577,
            y: 31235,
            floor: -1,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33598,
            y: 31228,
            floor: -1,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33577,
            y: 31235,
            floor: -1,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33598,
            y: 31229,
            floor: -1,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33577,
            y: 31235,
            floor: -1,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33491,
            y: 31192,
            floor: -2,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33501,
            y: 31197,
            floor: -2,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33491,
            y: 31193,
            floor: -2,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33501,
            y: 31197,
            floor: -2,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33491,
            y: 31194,
            floor: -2,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33501,
            y: 31197,
            floor: -2,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33577,
            y: 31220,
            floor: -2,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33606,
            y: 31229,
            floor: -2,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33577,
            y: 31221,
            floor: -2,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33606,
            y: 31229,
            floor: -2,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33577,
            y: 31222,
            floor: -2,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33606,
            y: 31229,
            floor: -2,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33492,
            y: 31198,
            floor: -3,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31206,
            floor: -3,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33492,
            y: 31199,
            floor: -3,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31206,
            floor: -3,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33492,
            y: 31200,
            floor: -3,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31206,
            floor: -3,
        },
    },
    Gate {
        appearance: 13278,
        position: Position {
            x: 33563,
            y: 31225,
            floor: -3,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33580,
            y: 31231,
            floor: -3,
        },
    },
    Gate {
        appearance: 13279,
        position: Position {
            x: 33564,
            y: 31225,
            floor: -3,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33580,
            y: 31231,
            floor: -3,
        },
    },
    Gate {
        appearance: 13280,
        position: Position {
            x: 33565,
            y: 31225,
            floor: -3,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33580,
            y: 31231,
            floor: -3,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33492,
            y: 31200,
            floor: -4,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31206,
            floor: -4,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33492,
            y: 31201,
            floor: -4,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31206,
            floor: -4,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33492,
            y: 31202,
            floor: -4,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31206,
            floor: -4,
        },
    },
    Gate {
        appearance: 13278,
        position: Position {
            x: 33582,
            y: 31220,
            floor: -4,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33580,
            y: 31231,
            floor: -4,
        },
    },
    Gate {
        appearance: 13279,
        position: Position {
            x: 33583,
            y: 31220,
            floor: -4,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33580,
            y: 31231,
            floor: -4,
        },
    },
    Gate {
        appearance: 13280,
        position: Position {
            x: 33584,
            y: 31220,
            floor: -4,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33580,
            y: 31231,
            floor: -4,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33534,
            y: 31204,
            floor: -5,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33518,
            y: 31193,
            floor: -5,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33534,
            y: 31205,
            floor: -5,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33518,
            y: 31193,
            floor: -5,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33534,
            y: 31206,
            floor: -5,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33518,
            y: 31193,
            floor: -5,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33599,
            y: 31226,
            floor: -5,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33594,
            y: 31230,
            floor: -5,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33599,
            y: 31227,
            floor: -5,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33594,
            y: 31230,
            floor: -5,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33599,
            y: 31228,
            floor: -5,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33594,
            y: 31230,
            floor: -5,
        },
    },
    Gate {
        appearance: 13278,
        position: Position {
            x: 33513,
            y: 31183,
            floor: -6,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31209,
            floor: -6,
        },
    },
    Gate {
        appearance: 13279,
        position: Position {
            x: 33514,
            y: 31183,
            floor: -6,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31209,
            floor: -6,
        },
    },
    Gate {
        appearance: 13280,
        position: Position {
            x: 33515,
            y: 31183,
            floor: -6,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33519,
            y: 31209,
            floor: -6,
        },
    },
    Gate {
        appearance: 13278,
        position: Position {
            x: 33562,
            y: 31228,
            floor: -6,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33594,
            y: 31230,
            floor: -6,
        },
    },
    Gate {
        appearance: 13279,
        position: Position {
            x: 33563,
            y: 31228,
            floor: -6,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33594,
            y: 31230,
            floor: -6,
        },
    },
    Gate {
        appearance: 13280,
        position: Position {
            x: 33564,
            y: 31228,
            floor: -6,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33594,
            y: 31230,
            floor: -6,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33484,
            y: 31204,
            floor: -7,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33480,
            y: 31203,
            floor: -7,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33484,
            y: 31205,
            floor: -7,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33480,
            y: 31203,
            floor: -7,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33484,
            y: 31206,
            floor: -7,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33480,
            y: 31203,
            floor: -7,
        },
    },
    Gate {
        appearance: 13278,
        position: Position {
            x: 33588,
            y: 31238,
            floor: -7,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33590,
            y: 31268,
            floor: -7,
        },
    },
    Gate {
        appearance: 13279,
        position: Position {
            x: 33589,
            y: 31238,
            floor: -7,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33590,
            y: 31268,
            floor: -7,
        },
    },
    Gate {
        appearance: 13280,
        position: Position {
            x: 33590,
            y: 31238,
            floor: -7,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33590,
            y: 31268,
            floor: -7,
        },
    },
    Gate {
        appearance: 13278,
        position: Position {
            x: 33505,
            y: 31209,
            floor: -8,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33484,
            y: 31314,
            floor: -8,
        },
    },
    Gate {
        appearance: 13279,
        position: Position {
            x: 33506,
            y: 31209,
            floor: -8,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33484,
            y: 31314,
            floor: -8,
        },
    },
    Gate {
        appearance: 13280,
        position: Position {
            x: 33507,
            y: 31209,
            floor: -8,
        },
        minimum_stage: 3,
        outer_route: Position {
            x: 33484,
            y: 31314,
            floor: -8,
        },
    },
    Gate {
        appearance: 13290,
        position: Position {
            x: 33539,
            y: 31198,
            floor: -8,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33587,
            y: 31261,
            floor: -8,
        },
    },
    Gate {
        appearance: 13291,
        position: Position {
            x: 33539,
            y: 31199,
            floor: -8,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33587,
            y: 31261,
            floor: -8,
        },
    },
    Gate {
        appearance: 13292,
        position: Position {
            x: 33539,
            y: 31200,
            floor: -8,
        },
        minimum_stage: 2,
        outer_route: Position {
            x: 33587,
            y: 31261,
            floor: -8,
        },
    },
];
pub fn gate_key(index: usize) -> String {
    format!("oteryn:quest-callback/hive-native-gate-{index}")
}
pub fn gate(key: &str) -> Option<&'static Gate> {
    let index = key
        .strip_prefix("oteryn:quest-callback/hive-native-gate-")?
        .parse::<usize>()
        .ok()?;
    GATES.get(index).filter(|_| key == gate_key(index))
}
/// The frozen actor is on the inner side; crossing moves toward the bound outer route.
pub fn is_egress(key: &str, actor: Position) -> bool {
    let Some(gate) = gate(key) else {
        return false;
    };
    if actor.floor != gate.position.floor {
        return false;
    }
    let (source, barrier, outer) = if gate.appearance < 13290 {
        (actor.y, gate.position.y, gate.outer_route.y)
    } else {
        (actor.x, gate.position.x, gate.outer_route.x)
    };
    (outer > barrier && source < barrier) || (outer < barrier && source > barrier)
}
/// This key is derived only by the server from its frozen actor snapshot. It is not
/// registered as a callback selector and never replaces the original source/CAS key.
pub fn observation_key(key: &str, actor: Position) -> String {
    if is_egress(key, actor) {
        format!("{key}/exit")
    } else {
        key.into()
    }
}
pub fn exit_gate(key: &str) -> Option<&'static Gate> {
    gate(key.strip_suffix("/exit")?)
}

pub fn destination(key: &str, actor: Position) -> Option<Position> {
    let gate = gate(key)?;
    if actor.floor != gate.position.floor {
        return None;
    }
    let mut target = gate.position;
    if (13278..=13283).contains(&gate.appearance) {
        if actor.y == target.y {
            return None;
        }
        target.y = if actor.y > target.y {
            target.y.checked_sub(1)?
        } else {
            target.y.checked_add(1)?
        };
    } else {
        if actor.x == target.x {
            return None;
        }
        target.x = if actor.x > target.x {
            target.x.checked_sub(1)?
        } else {
            target.x.checked_add(1)?
        };
    }
    Some(target)
}
pub fn callbacks() -> Vec<Callback> {
    GATES.iter().enumerate().map(|(index,gate)|Callback { failure_teleport: None, state_operations: Vec::new(),actor_kind: Default::default(), opening_only: false, requires_world_admission: false, facing: None, world_requirements: Vec::new(),
        key:gate_key(index),quest:super::shared_world_progress::HIVE_QUEST.into(),trigger:Trigger::Use,
use_range:1,disguise:None,completes:false,premium_required:false,present:vec![],required_travel:vec![],
        selector:Selector { position_aliases: Vec::new(),ordinal:None,native: None,position:Some(gate.position),unique:None,action:None,appearance:Some(gate.appearance)},
        lever:None,reversible:false,passages:vec![],teleport:Some(gate.position),storage:vec![],minimum_level:None,
        experience: None, world_change: None,monk_reward:None,offerings:vec![],item_on_target:None,return_on_failure:false,
        failure_message:Some("The Hive rejects your scent.".into()),success_message:None,native_rule:None,effects:Default::default(),remote_teleport:None,item_reward:None,reward_bundle:false,achievements:vec![],activation:None,requirements:None,
        sources:vec!["crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data/scripts/actions/objects/hive_gates.lua:3-24; wiki56902@930222:20; native B3 exact gates, authored east/west membership".into()],
    }).collect()
}
