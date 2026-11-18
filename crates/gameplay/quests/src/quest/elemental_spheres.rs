//! Source-bound item arrivals at the Elemental Spheres Overlord altars.

use super::world_callbacks::{Callback, Position};
use std::collections::BTreeSet;

pub const QUEST: &str = "oteryn:quest.elemental_spheres_quest";

/// Crystal 201189f8, actions_machine1.lua:9-37. The counter is personal;
/// both machine halves are shared map objects.
pub const MACHINE_GEMS: &str = "oteryn:quest-progress/elemental_spheres/machine_gem_count";

#[derive(Clone, Copy)]
pub struct MachineUse {
    pub key: &'static str,
    pub position: Position,
    pub appearance: u16,
    pub paired_appearance: u16,
    pub entering: bool,
}

const LEFT: Position = Position {
    x: 33268,
    y: 31830,
    floor: -10,
};
const RIGHT: Position = Position { x: 33269, ..LEFT };

const USES: [MachineUse; 6] = [
    MachineUse {
        key: "oteryn:quest-callback/elemental-spheres/left-close",
        position: LEFT,
        appearance: 846,
        paired_appearance: 847,
        entering: false,
    },
    MachineUse {
        key: "oteryn:quest-callback/elemental-spheres/left-enter",
        position: LEFT,
        appearance: 842,
        paired_appearance: 843,
        entering: true,
    },
    MachineUse {
        key: "oteryn:quest-callback/elemental-spheres/left-open",
        position: LEFT,
        appearance: 842,
        paired_appearance: 843,
        entering: false,
    },
    MachineUse {
        key: "oteryn:quest-callback/elemental-spheres/right-close",
        position: RIGHT,
        appearance: 847,
        paired_appearance: 846,
        entering: false,
    },
    MachineUse {
        key: "oteryn:quest-callback/elemental-spheres/right-enter",
        position: RIGHT,
        appearance: 843,
        paired_appearance: 842,
        entering: true,
    },
    MachineUse {
        key: "oteryn:quest-callback/elemental-spheres/right-open",
        position: RIGHT,
        appearance: 843,
        paired_appearance: 842,
        entering: false,
    },
];

pub fn machine_use(key: &str) -> Option<MachineUse> {
    USES.iter().find(|rule| rule.key == key).copied()
}

pub(super) fn machine_phases_are_disjoint(left: &Callback, right: &Callback) -> bool {
    machine_use(&left.key)
        .zip(machine_use(&right.key))
        .is_some_and(|(left, right)| {
            left.position == right.position
                && left.appearance == right.appearance
                && left.entering != right.entering
        })
}

pub(super) fn retain_machine_predecessor(catalogue: &mut super::QuestStateCatalogue) {
    let mut tracks = catalogue.tracks.clone();
    if tracks.remove(MACHINE_GEMS).is_none() {
        return;
    }
    let mut transitions = catalogue.transitions.clone();
    transitions.retain(|key, _| machine_use(key).is_none());
    let hash = super::definition_hash(QUEST, &tracks, &transitions);
    catalogue
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(super::NpcDefinitionPredecessor {
            hash,
            defaults: Default::default(),
        });
}

pub fn sphere_destination(base_vocation: &str) -> Option<Position> {
    let (x, y) = match base_vocation {
        "druid" => (33331, 32076),
        "knight" => (33087, 32096),
        "paladin" => (33265, 32202),
        "sorcerer" => (33183, 32197),
        _ => return None,
    };
    Some(Position { x, y, floor: -13 })
}

pub(super) fn machine_callbacks() -> Result<Vec<Callback>, String> {
    USES.into_iter().map(|rule| {
        let opening = rule.appearance < 846;
        let replacement = |appearance| if opening { appearance + 4 } else { appearance - 4 };
        let paired_position = if rule.position == LEFT { RIGHT } else { LEFT };
        let storage = match (opening, rule.entering) {
            (_, true) => serde_json::json!([{
                "track": MACHINE_GEMS, "minimum": 20, "maximum": i64::MAX,
                "set": 0, "add": -20
            }]),
            (true, false) => serde_json::json!([{
                "track": MACHINE_GEMS, "minimum": -1, "maximum": 19,
                "set": 0, "preserve": true
            }]),
            (false, false) => serde_json::json!([]),
        };
        serde_json::from_value(serde_json::json!({
            "key": rule.key,
            "quest": QUEST,
            "trigger": "use",
            "selector": {"position": rule.position, "appearance": rule.appearance},
            "reversible": false,
            "passages": [{
                "position": paired_position, "appearance": rule.paired_appearance,
                "if_present": true, "replacement": replacement(rule.paired_appearance)
            }],
            "effects": {"transform": replacement(rule.appearance)},
            "storage": storage,
            "offerings": [],
            "return_on_failure": false,
            "sources": ["crystalserver:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/elemental_spheres/actions_machine1.lua:9-37"]
        })).map_err(|error| error.to_string())
    }).collect()
}

#[derive(Clone, Copy, Debug)]
pub struct SummonRule {
    pub key: &'static str,
    pub altar: &'static str,
    pub corpse: &'static str,
    pub charged: &'static str,
    pub creature: &'static str,
    pub spawn: Position,
    pub positions: [Position; 8],
}

/// A proposal from the current Channel map owner, never a custody permission.
/// The arrival transaction must fence the readset and lock the shared boss latch.
#[derive(Clone, Debug)]
pub struct ArrivalPlan {
    pub rule: &'static SummonRule,
    pub entries: Vec<(u64, u64)>,
}

// Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
// data-global/scripts/quests/elemental_spheres/movements_summon_overlord.lua:1-122.
pub fn rules() -> &'static [SummonRule] {
    const RULES: &[SummonRule] = &[
        SummonRule {
            key: "oteryn:item-arrival/elemental_spheres/energy_overlord",
            altar: "oteryn:item.tibia.i7738",
            corpse: "oteryn:item.tibia.i8139",
            charged: "oteryn:item.tibia.i2126",
            creature: "oteryn:creature.energy_overlord",
            spawn: Position {
                x: 33095,
                y: 32194,
                floor: -13,
            },
            positions: [
                Position {
                    x: 33094,
                    y: 32189,
                    floor: -13,
                },
                Position {
                    x: 33097,
                    y: 32189,
                    floor: -13,
                },
                Position {
                    x: 33099,
                    y: 32191,
                    floor: -13,
                },
                Position {
                    x: 33099,
                    y: 32194,
                    floor: -13,
                },
                Position {
                    x: 33097,
                    y: 32196,
                    floor: -13,
                },
                Position {
                    x: 33094,
                    y: 32196,
                    floor: -13,
                },
                Position {
                    x: 33092,
                    y: 32194,
                    floor: -13,
                },
                Position {
                    x: 33092,
                    y: 32191,
                    floor: -13,
                },
            ],
        },
        SummonRule {
            key: "oteryn:item-arrival/elemental_spheres/fire_overlord",
            altar: "oteryn:item.tibia.i7739",
            corpse: "oteryn:item.tibia.i8141",
            charged: "oteryn:item.tibia.i7745",
            creature: "oteryn:creature.fire_overlord",
            spawn: Position {
                x: 33199,
                y: 32103,
                floor: -13,
            },
            positions: [
                Position {
                    x: 33198,
                    y: 32102,
                    floor: -13,
                },
                Position {
                    x: 33201,
                    y: 32102,
                    floor: -13,
                },
                Position {
                    x: 33203,
                    y: 32104,
                    floor: -13,
                },
                Position {
                    x: 33203,
                    y: 32107,
                    floor: -13,
                },
                Position {
                    x: 33201,
                    y: 32109,
                    floor: -13,
                },
                Position {
                    x: 33198,
                    y: 32109,
                    floor: -13,
                },
                Position {
                    x: 33196,
                    y: 32107,
                    floor: -13,
                },
                Position {
                    x: 33196,
                    y: 32104,
                    floor: -13,
                },
            ],
        },
        SummonRule {
            key: "oteryn:item-arrival/elemental_spheres/ice_overlord",
            altar: "oteryn:item.tibia.i7740",
            corpse: "oteryn:item.tibia.i8140",
            charged: "oteryn:item.tibia.i7744",
            creature: "oteryn:creature.ice_overlord",
            spawn: Position {
                x: 33286,
                y: 32102,
                floor: -13,
            },
            positions: [
                Position {
                    x: 33285,
                    y: 32097,
                    floor: -13,
                },
                Position {
                    x: 33288,
                    y: 32097,
                    floor: -13,
                },
                Position {
                    x: 33290,
                    y: 32099,
                    floor: -13,
                },
                Position {
                    x: 33290,
                    y: 32102,
                    floor: -13,
                },
                Position {
                    x: 33288,
                    y: 32104,
                    floor: -13,
                },
                Position {
                    x: 33285,
                    y: 32104,
                    floor: -13,
                },
                Position {
                    x: 33283,
                    y: 32102,
                    floor: -13,
                },
                Position {
                    x: 33283,
                    y: 32099,
                    floor: -13,
                },
            ],
        },
        SummonRule {
            key: "oteryn:item-arrival/elemental_spheres/earth_overlord",
            altar: "oteryn:item.tibia.i7748",
            corpse: "oteryn:item.tibia.i8106",
            charged: "oteryn:item.tibia.i7746",
            creature: "oteryn:creature.earth_overlord",
            spawn: Position {
                x: 33347,
                y: 32208,
                floor: -13,
            },
            positions: [
                Position {
                    x: 33346,
                    y: 32203,
                    floor: -13,
                },
                Position {
                    x: 33349,
                    y: 32203,
                    floor: -13,
                },
                Position {
                    x: 33351,
                    y: 32205,
                    floor: -13,
                },
                Position {
                    x: 33351,
                    y: 32208,
                    floor: -13,
                },
                Position {
                    x: 33349,
                    y: 32210,
                    floor: -13,
                },
                Position {
                    x: 33346,
                    y: 32210,
                    floor: -13,
                },
                Position {
                    x: 33344,
                    y: 32208,
                    floor: -13,
                },
                Position {
                    x: 33344,
                    y: 32205,
                    floor: -13,
                },
            ],
        },
    ];
    RULES
}

/// Read every source charged cell only for the matching arrival. Additional
/// scenery is allowed, as in Tile:getItemById; no charged object is consumed.
pub fn plan_arrival(
    altar: &str,
    arrived: &str,
    boss_active: bool,
    mut read: impl FnMut(Position, &str) -> Option<(u64, u64)>,
) -> Option<ArrivalPlan> {
    if boss_active {
        return None;
    }
    let rule = rules()
        .iter()
        .find(|rule| rule.altar == altar && rule.corpse == arrived)?;
    let mut identities = BTreeSet::new();
    let mut entries = Vec::with_capacity(rule.positions.len());
    for position in rule.positions {
        let (key, revision) = read(position, rule.charged)?;
        if key == 0 || revision == u64::MAX || !identities.insert(key) {
            return None;
        }
        entries.push((key, revision));
    }
    Some(ArrivalPlan { rule, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elemental_spheres_machine_routes_use_the_same_durable_counter() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let track = catalogue.track(MACHINE_GEMS).ok_or("gem counter")?;
        assert_eq!((track.initial, track.min, track.max), (-1, -1, i64::MAX));
        for rule in USES {
            let transition = catalogue.transition(rule.key);
            if rule.appearance >= 846 {
                assert!(transition.is_none());
                continue;
            }
            let [effect] = transition
                .ok_or("machine USE transition")?
                .effects
                .as_slice()
            else {
                return Err("machine USE must have exactly one counter effect".into());
            };
            use super::super::{QuestComparison, QuestEffectKind};
            assert_eq!(effect.track, MACHINE_GEMS);
            if rule.entering {
                assert_eq!(effect.from, QuestComparison::Between(20, i64::MAX));
                assert_eq!(effect.effect, QuestEffectKind::Add(-20));
            } else {
                assert_eq!(effect.from, QuestComparison::Between(-1, 19));
                assert_eq!(effect.effect, QuestEffectKind::Add(0));
            }
        }
        Ok(())
    }

    #[test]
    fn m49_elemental_arrival_requires_the_corpse_and_all_eight_charged_cells()
    -> Result<(), &'static str> {
        let rule = rules()
            .iter()
            .find(|r| r.altar == "oteryn:item.tibia.i7738")
            .ok_or("energy altar")?;
        let mut seen = Vec::new();
        let plan = plan_arrival(rule.altar, rule.corpse, false, |position, item| {
            seen.push((position, item.to_owned()));
            Some((u64::try_from(seen.len()).ok()?, 0))
        })
        .ok_or("complete source readset")?;
        assert_eq!(seen.len(), 8);
        assert_eq!(plan.entries.len(), 8);
        assert_eq!(plan.rule.creature, "oteryn:creature.energy_overlord");
        assert_eq!(
            plan.rule.spawn,
            Position {
                x: 33095,
                y: 32194,
                floor: -13
            }
        );
        assert!(
            plan_arrival(rule.altar, "oteryn:item.tibia.i8141", false, |_, _| Some((
                1, 0
            )))
            .is_none()
        );
        assert!(plan_arrival(rule.altar, rule.corpse, true, |_, _| Some((1, 0))).is_none());
        for missing in rule.positions {
            let mut ordinal = 0;
            assert!(
                plan_arrival(rule.altar, rule.corpse, false, |position, _| {
                    ordinal += 1;
                    (position != missing).then_some((ordinal, 0))
                })
                .is_none()
            );
        }
        assert!(plan_arrival(rule.altar, rule.corpse, false, |_, _| Some((1, 0))).is_none());
        Ok(())
    }
}
