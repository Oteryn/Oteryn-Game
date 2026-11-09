//! Source-qualified Elemental Spheres item-arrival rules.
use super::world_callbacks::Position;

#[derive(Debug, PartialEq, Eq)]
pub struct Recipe {
    pub altar: u16,
    pub corpse: u16,
    pub charged: u16,
    pub creature: &'static str,
    pub summon: Position,
    pub positions: [Position; 8],
}

pub const QUEST: &str = "oteryn:quest.elemental_spheres_quest";
pub const SOURCE: &str = "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/elemental_spheres/movements_summon_overlord.lua:1-114";
// OTS_HYPOTHESIS_ONLY. The retained donor requires all eight positions.
pub const RECIPES: &[Recipe] = &[
    Recipe {
        altar: 7738,
        corpse: 8139,
        charged: 2126,
        creature: "oteryn:creature.energy_overlord",
        summon: Position {
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
    Recipe {
        altar: 7739,
        corpse: 8141,
        charged: 7745,
        creature: "oteryn:creature.fire_overlord",
        summon: Position {
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
    Recipe {
        altar: 7740,
        corpse: 8140,
        charged: 7744,
        creature: "oteryn:creature.ice_overlord",
        summon: Position {
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
    Recipe {
        altar: 7748,
        corpse: 8106,
        charged: 7746,
        creature: "oteryn:creature.earth_overlord",
        summon: Position {
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

pub fn recipe(altar: u16) -> Option<&'static Recipe> {
    RECIPES.iter().find(|recipe| recipe.altar == altar)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn elemental_arrival_binds_eight_source_altars_for_each_overlord() -> Result<(), String> {
        for (altar, corpse, charged, creature) in [
            (7738, 8139, 2126, "oteryn:creature.energy_overlord"),
            (7739, 8141, 7745, "oteryn:creature.fire_overlord"),
            (7740, 8140, 7744, "oteryn:creature.ice_overlord"),
            (7748, 8106, 7746, "oteryn:creature.earth_overlord"),
        ] {
            let recipe = recipe(altar).ok_or("missing source item-arrival recipe")?;
            assert_eq!(
                (recipe.corpse, recipe.charged, recipe.creature),
                (corpse, charged, creature)
            );
            assert_eq!(recipe.positions.len(), 8);
        }
        assert!(recipe(0).is_none());
        Ok(())
    }
}

/// Immutable expected scope; admission must supply independently observed current facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrivalFence {
    pub world: [u8; 16],
    pub channel: [u8; 16],
    pub scope_generation: u64,
    pub map: [u8; 32],
    pub content: [u8; 32],
    pub reset_epoch: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectSnapshot {
    pub key: u64,
    pub revision: u64,
    pub position: Position,
    pub appearance: u16,
}

/// Physical Item identity and its exact post-arrival state, supplied by the Item owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorpseArrival {
    pub instance: [u8; 16],
    pub state_revision: u64,
    pub item: u16,
    pub quantity: u32,
    pub position: Position,
}

/// One indivisible instruction: retire this whole corpse and summon this native boss.
/// No caller can construct a successful plan from a partial altar readset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummonPlan {
    fence: ArrivalFence,
    recipe: &'static Recipe,
    target: ObjectSnapshot,
    corpse: CorpseArrival,
    readset: [ObjectSnapshot; 8],
}

pub fn prepare_arrival(
    current: ArrivalFence,
    target: ObjectSnapshot,
    corpse: CorpseArrival,
    altars: &[ObjectSnapshot],
    boss_active: bool,
) -> Option<SummonPlan> {
    let recipe = recipe(target.appearance)?;
    if boss_active
        || altars.len() != 8
        || current.scope_generation == 0
        || corpse.instance == [0; 16]
        || corpse.state_revision == u64::MAX
        || corpse.item != recipe.corpse
        || corpse.quantity != 1
        || corpse.position != target.position
        || !(-15..=0).contains(&target.position.floor)
    {
        return None;
    }
    let keys: std::collections::BTreeSet<_> = altars.iter().map(|altar| altar.key).collect();
    if keys.len() != 8 || keys.contains(&target.key) {
        return None;
    }
    let mut readset = Vec::with_capacity(8);
    for position in recipe.positions {
        let mut at = altars.iter().filter(|altar| altar.position == position);
        let altar = *at.next()?;
        if at.next().is_some() || altar.appearance != recipe.charged {
            return None;
        }
        readset.push(altar);
    }
    Some(SummonPlan {
        fence: current,
        recipe,
        target,
        corpse,
        readset: readset.try_into().ok()?,
    })
}

impl SummonPlan {
    pub fn fence(&self) -> ArrivalFence {
        self.fence
    }
    pub fn recipe(&self) -> &'static Recipe {
        self.recipe
    }
    pub fn corpse(&self) -> CorpseArrival {
        self.corpse
    }
    pub fn target(&self) -> ObjectSnapshot {
        self.target
    }
    pub fn readset(&self) -> &[ObjectSnapshot; 8] {
        &self.readset
    }

    /// The commit owner must re-prove the entire observation, even the unchanged altars.
    pub fn revalidate(
        &self,
        current: ArrivalFence,
        target: ObjectSnapshot,
        corpse: CorpseArrival,
        altars: &[ObjectSnapshot],
        boss_active: bool,
    ) -> bool {
        prepare_arrival(current, target, corpse, altars, boss_active).as_ref() == Some(self)
    }
}

#[cfg(test)]
mod readset_tests {
    use super::*;

    fn fence() -> ArrivalFence {
        ArrivalFence {
            world: [1; 16],
            channel: [2; 16],
            scope_generation: 1,
            map: [3; 32],
            content: [4; 32],
            reset_epoch: 0,
        }
    }

    #[test]
    fn elemental_arrival_requires_every_altar_and_revalidates_every_fence() -> Result<(), String> {
        for recipe in RECIPES {
            let target = ObjectSnapshot {
                key: 100,
                revision: 7,
                position: recipe.summon,
                appearance: recipe.altar,
            };
            let corpse = CorpseArrival {
                instance: [5; 16],
                state_revision: 1,
                item: recipe.corpse,
                quantity: 1,
                position: target.position,
            };
            let altars: Vec<_> = recipe
                .positions
                .iter()
                .zip(1..=8)
                .map(|(position, key)| ObjectSnapshot {
                    key,
                    revision: 10,
                    position: *position,
                    appearance: recipe.charged,
                })
                .collect();
            let plan = prepare_arrival(fence(), target, corpse, &altars, false)
                .ok_or("complete source arrival")?;
            assert_eq!(plan.recipe().creature, recipe.creature);
            assert_eq!(plan.corpse(), corpse);
            assert!(plan.revalidate(fence(), target, corpse, &altars, false));
            let mut reordered = altars.clone();
            reordered.reverse();
            assert!(plan.revalidate(fence(), target, corpse, &reordered, false));
            assert!(prepare_arrival(fence(), target, corpse, &altars[..7], false).is_none());
            assert!(prepare_arrival(fence(), target, corpse, &altars, true).is_none());
            for cell in 0..8 {
                let mut changed = altars.clone();
                changed[cell].appearance = 1;
                assert!(prepare_arrival(fence(), target, corpse, &changed, false).is_none());
                changed[cell] = altars[cell];
                changed[cell].revision += 1;
                assert!(!plan.revalidate(fence(), target, corpse, &changed, false));
            }
            let mut duplicate = altars.clone();
            duplicate[7].key = duplicate[0].key;
            assert!(prepare_arrival(fence(), target, corpse, &duplicate, false).is_none());
            duplicate[7] = duplicate[0];
            assert!(prepare_arrival(fence(), target, corpse, &duplicate, false).is_none());
            for mutation in 0..6 {
                let mut changed = fence();
                match mutation {
                    0 => changed.world[0] += 1,
                    1 => changed.channel[0] += 1,
                    2 => changed.scope_generation += 1,
                    3 => changed.map[0] += 1,
                    4 => changed.content[0] += 1,
                    _ => changed.reset_epoch += 1,
                }
                assert!(!plan.revalidate(changed, target, corpse, &altars, false));
            }
            let retired = CorpseArrival {
                quantity: 0,
                state_revision: 2,
                ..corpse
            };
            assert!(!plan.revalidate(fence(), target, retired, &altars, false));
            let wrong_corpse = CorpseArrival {
                item: 3031,
                ..corpse
            };
            assert!(prepare_arrival(fence(), target, wrong_corpse, &altars, false).is_none());
            let wrong_place = CorpseArrival {
                position: Position {
                    x: target.position.x + 1,
                    ..target.position
                },
                ..corpse
            };
            assert!(prepare_arrival(fence(), target, wrong_place, &altars, false).is_none());
            let stale_target = ObjectSnapshot {
                revision: target.revision + 1,
                ..target
            };
            assert!(!plan.revalidate(fence(), stale_target, corpse, &altars, false));
        }
        Ok(())
    }
}
