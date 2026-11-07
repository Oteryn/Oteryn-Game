//! Compile-time selection references become one bounded runtime activation plan.
use super::*;
use oteryn_gameplay_entities::carrier::{ActivationSpawnAlternative, ActivationWeightedSpawnPoint};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BundleSpawnPlan {
    pub sources: Vec<BundleSpawnFacts>,
    pub weighted: Vec<ActivationWeightedSpawnPoint>,
}

pub fn table_activation_spawn_plan(
    table: &oteryn_world_bundle::spawn::Table,
    profile: impl Fn(&str) -> Option<(u64, bool)>,
) -> Option<BundleSpawnPlan> {
    use oteryn_world_bundle::spawn::{self, Period};
    if table.sources.len() > spawn::MAX_SOURCES
        || table.creatures.len() > spawn::MAX_POINTS
        || table.point_count() > spawn::MAX_POINTS
    {
        return None;
    }
    spawn::validate_weighted(table).ok()?;
    let mut profiles = vec![None; table.creatures.len()];
    let mut resolve = |index: u32| -> Option<(String, u64, bool)> {
        let index = usize::try_from(index).ok()?;
        let creature = table.creatures.get(index)?;
        let cached = profiles.get_mut(index)?;
        let (health, blockable) = match *cached {
            Some(known) => known,
            None => *cached.insert(profile(&creature.key)?),
        };
        Some((creature.key.clone(), health, blockable))
    };
    let mut plan = BundleSpawnPlan::default();
    for (source_index, source) in table.sources.iter().enumerate() {
        let floor = -i16::from(source.floor);
        let mut points = Vec::with_capacity(source.points.len());
        for (point_index, point) in source.points.iter().enumerate() {
            let creature = table.creatures.get(point.creature as usize)?;
            if creature.period != Period::All || point.boss_policy.is_some() {
                continue;
            }
            let (key, health, blockable) = resolve(point.creature)?;
            let pair = (
                u32::try_from(source_index).ok()?,
                u16::try_from(point_index).ok()?,
            );
            if let Ok(index) = table
                .weighted
                .binary_search_by_key(&pair, |entry| (entry.source, entry.point))
            {
                let selection = &table.weighted[index];
                let choices = selection
                    .choices
                    .iter()
                    .map(|choice| {
                        let (creature, maximum_health, blockable) = resolve(choice.creature)?;
                        Some(ActivationSpawnAlternative {
                            creature,
                            weight: choice.weight,
                            maximum_health,
                            blockable,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                plan.weighted.push(ActivationWeightedSpawnPoint {
                    source: u16::try_from(plan.sources.len()).ok()?,
                    point: u16::try_from(points.len()).ok()?,
                    choices,
                });
            }
            points.push((
                (i32::from(point.x), i32::from(point.y), floor),
                u64::from(point.respawn_ms),
                key,
                health,
                blockable,
            ));
        }
        if !points.is_empty() {
            plan.sources.push((points, BUNDLE_OCCUPANCY_RETRY_MS));
        }
    }
    Some(plan)
}

#[cfg(test)]
mod tests {
    use crate::foundation::{ChannelId, ChannelRuntimeV1, NodeId, WorldId};
    use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
    use oteryn_world_bundle::{
        bundle::Extent,
        spawn::{
            self, Creature, Direction, Period, Point, Source, Table, WeightedChoice, WeightedPoint,
        },
    };
    use std::cell::RefCell;

    #[test]
    fn srv7_weighted_bundle_decoder_boot_plan_and_runtime_realize_two_physical_slots()
    -> Result<(), Box<dyn std::error::Error>> {
        let table = Table {
            creatures: vec![
                Creature {
                    key: "oteryn:creature.silverfrost_sentinel".into(),
                    period: Period::All,
                },
                Creature {
                    key: "oteryn:creature.true_frost_flower_asura".into(),
                    period: Period::All,
                },
            ],
            sources: vec![Source {
                key: "oteryn:spawn.x33866_y32698_z9".into(),
                floor: -7,
                x: 2,
                y: 0,
                points: vec![
                    Point {
                        creature: 1,
                        x: 2,
                        y: 0,
                        direction: Direction::North,
                        respawn_ms: 47_104_000,
                        boss_policy: None,
                    },
                    Point {
                        creature: 1,
                        x: 3,
                        y: 0,
                        direction: Direction::North,
                        respawn_ms: 60_000,
                        boss_policy: None,
                    },
                ],
            }],
            weighted: vec![WeightedPoint {
                source: 0,
                point: 0,
                choices: vec![
                    WeightedChoice {
                        creature: 0,
                        weight: 25,
                    },
                    WeightedChoice {
                        creature: 1,
                        weight: 25,
                    },
                ],
            }],
        };
        let extent = Extent {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
            floors: vec![-7],
        };
        let table = spawn::decode(&spawn::encode(&table), &extent)?;
        assert!(
            super::super::table_activation_spawns(&table, |_| Some((100, false))).is_none(),
            "scalar boot must never guarantee one weighted identity"
        );
        let world = super::super::tests::door_world()?.with_spawns_for_test(table);
        let reads = RefCell::new(Vec::new());
        let plan = world
            .activation_spawn_plan(|key| {
                reads.borrow_mut().push(key.to_owned());
                Some((if key.ends_with("sentinel") { 100 } else { 200 }, false))
            })
            .ok_or("complete activation plan")?;
        assert_eq!(reads.borrow().len(), 2, "resolve each native profile once");
        assert_eq!(plan.sources.len(), 1);
        assert_eq!(plan.sources[0].0.len(), 2);
        assert_eq!(plan.weighted.len(), 1);
        assert_eq!(plan.weighted[0].choices.len(), 2);
        let id = |n: u8| [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n];
        let world_id = WorldId::decode(&id(1))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            ChannelId::decode(&id(2))?,
            NodeId::decode(&id(3))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(world_id),
        )?;
        assert_eq!(
            runtime.begin_activation_spawn_plan(plan.sources, plan.weighted)?,
            2
        );
        assert_eq!(runtime.realize_activation_spawn_window(0)?.realized, 2);
        assert_eq!(runtime.spawn_point_creatures().len(), 2);
        Ok(())
    }
}
