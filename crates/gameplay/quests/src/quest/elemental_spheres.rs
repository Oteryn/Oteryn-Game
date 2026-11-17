//! Source-bound item arrivals at the Elemental Spheres Overlord altars.

use super::world_callbacks::Position;
use std::collections::BTreeSet;

pub const QUEST: &str = "oteryn:quest.elemental_spheres_quest";

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
