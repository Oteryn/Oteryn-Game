//! Immediate quest creations use the current Channel carrier and Crystal's local placement search.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::movement::source_placement::{SourcePlacementCell, SourcePlacementPlan};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::CreatureSpawn;
use std::hash::{BuildHasher, RandomState};

#[derive(Clone)]
pub(in crate::gameplay_transport) struct PreparedSpawn(Vec<(String, i64, MovementLocalPosition)>);

impl PreparedSpawn {
    fn members(&self) -> Vec<(&str, i64, MovementLocalPosition)> {
        self.0
            .iter()
            .map(|(key, health, at)| (key.as_str(), *health, *at))
            .collect()
    }

    pub(in crate::gameplay_transport) fn publish(&self, runtime: &mut ChannelRuntimeV1) -> bool {
        runtime.commit_quest_creature_spawn(&self.members()).is_ok()
    }
}

pub(in crate::gameplay_transport) fn prepare(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    map: &BundleMap,
    center: MovementLocalPosition,
    groups: &[CreatureSpawn],
) -> Result<PreparedSpawn, UseDisposition> {
    if !world_callbacks::valid_creature_spawn(groups) {
        return Err(UseDisposition::Rejected);
    }
    let native_gameplay = admission
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or(UseDisposition::Rejected)?;
    native_gameplay
        .install_companion_policies(runtime)
        .map_err(|_| UseDisposition::Rejected)?;
    let census = runtime
        .positioned_melee_census()
        .map_err(|_| UseDisposition::Rejected)?;
    let npcs = runtime.visible_entities().npcs;
    let mut occupied: Vec<_> = census.into_iter().map(|(_, at, _)| at.position()).collect();
    occupied.extend(npcs.iter().filter_map(|npc| {
        runtime
            .read_movement_actor_position(npc.actor)
            .ok()
            .map(|at| at.position())
    }));
    let mut creatures = Vec::new();
    for group in groups {
        let health = runtime
            .companion_policy(&group.creature)
            .map_err(|_| UseDisposition::Rejected)?
            .maximum_health;
        for _ in 0..group.count {
            // Crystal 201189f8 src/map/map.cpp:234-311 tries the center and shuffles adjacent tiles.
            let random = RandomState::new();
            let mut step = 0_u32;
            let placement = SourcePlacementPlan::prepare(center, false, false, |bound| {
                step += 1;
                (random.hash_one(step) % u64::from(bound)) as u32
            })
            .map_err(|_| UseDisposition::NotPossible)?;
            let at = placement
                .select(
                    |at| {
                        let pos = native(at)?;
                        let terrain = map.overlay.base().tile(pos.x, pos.y, pos.floor);
                        let protection_zone = terrain.is_some_and(|tile| tile.protection_zone());
                        let teleport = map
                            .facts
                            .quest_portal_destination(pos, admission.owner_now().get())
                            .is_some();
                        Some(SourcePlacementCell {
                            exists: terrain.is_some(),
                            protection_zone,
                            teleport,
                            query_add: !protection_zone
                                && !teleport
                                && !occupied.contains(&at)
                                && !map.facts.house_tile(pos)
                                && !map.facts.quest_callback_blocked(
                                    map.overlay.base(),
                                    pos,
                                    false,
                                )
                                && admission.movement_cells.index().lookup(
                                    admission.movement_cells.scope(),
                                    LogicalCell {
                                        x: at.x,
                                        y: at.y,
                                        z: i32::from(at.floor),
                                    },
                                ) == Ok(CollisionClass::Walkable),
                        })
                    },
                    |_, _| Some(true),
                )
                .map_err(|_| UseDisposition::NotPossible)?
                .ok_or(UseDisposition::Occupied)?;
            occupied.push(at);
            creatures.push((group.creature.clone(), health, at));
        }
    }
    let spawn = PreparedSpawn(creatures);
    runtime
        .preflight_quest_creature_spawn(&spawn.members())
        .map_err(|_| UseDisposition::Exhausted)?;
    Ok(spawn)
}
