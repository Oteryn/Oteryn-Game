use super::*;
use crate::durability::seasonal_cake::CakeWorldSnapshot;
use crate::foundation::ExactActorRef;
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;
use sha2::{Digest, Sha256};
use std::time::Duration;

// Crystal a_piece_of_cake_config.lua:207-227. Twenty source attempts per city.
const RAID_SPOTS: [[(i32, i32); 2]; 5] = [
    [(32381, 31785), (32310, 31789)],
    [(32324, 32216), (32390, 32214)],
    [(33160, 31824), (33222, 31880)],
    [(33264, 32446), (33195, 32463)],
    [(32317, 32835), (32288, 32871)],
];

#[derive(Default)]
struct CakeRun {
    golems: Vec<ExactActorRef>,
    bridge_open: bool,
    machines_open: bool,
    projected: Option<(i64, u64)>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn install_cake_projection(
        &self,
        projection: &crate::durability::seasonal_cake::CakeMapProjection,
    ) -> bool {
        let Some(map) = self.bundle_map() else {
            return false;
        };
        for row in &projection.terrain {
            if row.state.placement.len() != 40
                || row.state.placement[..32] != map.overlay.base().digest()[..]
            {
                return false;
            }
            let Ok(bytes) = row.state.placement[32..40].try_into() else {
                return false;
            };
            let key = u64::from_be_bytes(bytes);
            let Some(before) = map.facts.quest_callback_replacement(key, row.source) else {
                return false;
            };
            let Ok(revision) = u64::try_from(row.state.revision) else {
                return false;
            };
            let after = if row.state.item == 0 {
                None
            } else {
                let Ok(item) = u16::try_from(row.state.item) else {
                    return false;
                };
                let Some(after) = map.facts.quest_callback_replacement(key, item) else {
                    return false;
                };
                Some(after)
            };
            if !map
                .facts
                .install_gathering_state(key, revision, before, after)
            {
                return false;
            }
        }
        map.facts.project_cake_tower(
            map.overlay.base(),
            projection.world.state.stage,
            projection.world.state.layer,
            &projection.removed.iter().copied().collect(),
        )
    }

    pub(in crate::gameplay_transport) async fn initialize_cake_world(&self) -> Result<(), ()> {
        self.cake_world_snapshot().await.map(|_| ())
    }

    async fn cake_world_snapshot(&self) -> Result<Option<CakeWorldSnapshot>, ()> {
        let binding = self.runtime.lock().await.binding();
        let Some(channel) = binding.channel_id() else {
            return Ok(None);
        };
        let world = self
            .root
            .refresh_cake_world(
                self.character,
                self.holder,
                self.world_id,
                channel,
                binding.scope_generation(),
                now_seconds(),
            )
            .await
            .map_err(|_| ())?;
        self.states.cake_stage.store(
            world.as_ref().map_or(0, |world| world.state.stage),
            std::sync::atomic::Ordering::Relaxed,
        );
        Ok(world)
    }

    pub(in crate::gameplay_transport) async fn run_cake_events(
        &self,
        stop: &CancellationToken,
    ) -> RespawnExit {
        let mut run = CakeRun::default();
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            if super::super::world_respawns::stopping_first(interval.tick(), stop)
                .await
                .is_none()
            {
                return RespawnExit::Shutdown;
            }
            self.resolve_cake_food_for_cadence().await;
            let Ok(Some(mut world)) = self.cake_world_snapshot().await else {
                continue;
            };
            let Some(_permit) = self.spell_lane_permit().await else {
                continue;
            };
            let now = now_seconds();
            let binding = self.runtime.lock().await.binding();
            let projection = if run.projected != Some((world.event_start, world.state.revision)) {
                let (Some(channel), Some(map), Some(native)) = (
                    binding.channel_id(),
                    self.bundle_map(),
                    self.active_generation
                        .and_then(|active| active.native_gameplay()),
                ) else {
                    continue;
                };
                let Ok(epoch) = i64::try_from(self.reset_epoch) else {
                    continue;
                };
                let scope = crate::durability::terrain_tool::TerrainScope {
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_generation: native.source_digest(),
                    reset_epoch: epoch,
                };
                let Ok(Some(projection)) = self
                    .root
                    .cake_map_projection(
                        self.character,
                        self.holder,
                        self.world_id,
                        channel,
                        binding.scope_generation(),
                        scope,
                        now,
                    )
                    .await
                else {
                    continue;
                };
                world = projection.world.clone();
                Some(projection)
            } else {
                None
            };

            let spawn_claim = if world.state.stage == 1 {
                if let Some(channel) = binding.channel_id() {
                    self.root
                        .claim_cake_raid_slot(
                            self.character,
                            self.holder,
                            self.world_id,
                            channel,
                            binding.scope_generation(),
                            now,
                        )
                        .await
                        .ok()
                        .flatten()
                } else {
                    None
                }
            } else {
                None
            };
            let mut runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return RespawnExit::StaleOwner;
            }
            run.golems
                .retain(|actor| runtime.contains_live_creature(*actor));
            if let Some(projection) = &projection {
                if !self.install_cake_projection(projection) {
                    continue;
                }
                run.projected = Some((world.event_start, world.state.revision));
            }

            self.reconcile_cake_brutus(&mut runtime, &world, now);
            let machines_open = world.state.stage > 0 && now < world.event_end;
            if machines_open != run.machines_open
                && let Some(map) = self.bundle_map()
                && map
                    .facts
                    .project_cake_machines(map.overlay.base(), machines_open)
            {
                run.machines_open = machines_open;
            }
            let bridge_open = matches!(world.state.stage, 2 | 3) && now < world.event_end;
            if bridge_open != run.bridge_open
                && let Some(map) = self.bundle_map()
                && map
                    .facts
                    .project_cake_bridge(map.overlay.base(), bridge_open)
            {
                run.bridge_open = bridge_open;
            }

            if world.state.stage == 0 || now >= world.event_end {
                for actor in run.golems.drain(..) {
                    if runtime.matches_live_creature_identity(actor, GOLEM.as_bytes()) {
                        let _ = runtime.remove_arena_opponent(actor);
                    }
                }
                continue;
            }
            if world.state.stage != 1 {
                continue;
            }
            let Some((_, slot)) = spawn_claim else {
                continue;
            };
            // At-most-once source spawn attempts: a crash after the durable claim but
            // before native admission can lose a wave, and cannot multiply it on restart.
            let Some(map) = self.bundle_map() else {
                continue;
            };
            for (city, spots) in RAID_SPOTS.iter().enumerate() {
                for ordinal in 0_u64..20 {
                    let digest = Sha256::new()
                        .chain_update(b"oteryn:cake-raid:v1")
                        .chain_update(self.world_id.as_bytes())
                        .chain_update(self.channel_id.as_bytes())
                        .chain_update(world.event_start.to_be_bytes())
                        .chain_update(slot.to_be_bytes())
                        .chain_update(u64::try_from(city).unwrap_or(u64::MAX).to_be_bytes())
                        .chain_update(ordinal.to_be_bytes())
                        .finalize();
                    let draw = u64::from_be_bytes(digest[..8].try_into().unwrap_or([0; 8]));
                    let anchor = if draw & 1 == 0 { spots[0] } else { spots[1] };
                    let first = usize::try_from((u128::from(draw) * 49) >> 64).unwrap_or(0);
                    for attempt in 0..49 {
                        let offset = (first + attempt) % 49;
                        let at = MovementLocalPosition {
                            x: anchor.0 + i32::try_from(offset % 7).unwrap_or(0) - 3,
                            y: anchor.1 + i32::try_from(offset / 7).unwrap_or(0) - 3,
                            floor: 7,
                        };
                        let (Ok(x), Ok(y)) = (u16::try_from(at.x), u16::try_from(at.y)) else {
                            continue;
                        };
                        use crate::map::view::MapFacts;
                        if map.facts.pinned().house_tile(TilePos { x, y, floor: -7 }) {
                            continue;
                        }
                        let Some(doors) = self.door.try_lock().ok() else {
                            continue;
                        };
                        let closed = crate::map::floor::BundleDoors(map.facts.doors());
                        let admit = |owner: &crate::foundation::ChannelRuntimeV1, at| {
                            Some(
                                crate::movement::relocate::creature_cell_admissible(
                                    self.movement_cells,
                                    &closed,
                                    at,
                                ) && crate::movement::relocate::creature_cell_admissible(
                                    self.movement_cells,
                                    doors.blocking_cells(),
                                    at,
                                ) && owner.native_summon_cell_free(at),
                            )
                        };
                        if let Ok(actor) = runtime.spawn_arena_opponent(GOLEM, at, admit) {
                            run.golems.push(actor);
                            break;
                        }
                    }
                }
            }
        }
    }

    fn reconcile_cake_brutus(
        &self,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        world: &CakeWorldSnapshot,
        now: i64,
    ) {
        const BRUTUS: &str = "oteryn:npc.brutus_the_baker";
        let wanted = world.state.stage == 3
            && world.state.bonus_from.is_some_and(|start| now >= start)
            && world.state.bonus_until.is_some_and(|end| now < end)
            && now < world.event_end;
        if !wanted {
            let _ = runtime.remove_dynamic_npc(BRUTUS);
            return;
        }
        if runtime
            .visible_entities()
            .npcs
            .iter()
            .any(|npc| runtime.npc_key(npc.actor) == Some(BRUTUS))
        {
            return;
        }
        if self
            .states
            .chat
            .npc_talk()
            .is_none_or(|book| book.get(BRUTUS).is_none())
        {
            return;
        }
        let _ = runtime.place_dynamic_npc(
            BRUTUS,
            crate::foundation::NpcPlacement {
                x: 33303,
                y: 32064,
                floor: -7,
                direction: crate::foundation::MovementFacing::South,
            },
        );
    }
}
