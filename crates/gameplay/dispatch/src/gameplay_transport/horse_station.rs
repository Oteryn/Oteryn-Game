//! Closed source horse station uses existing ordinary native actors and respawn task lifetime.
use super::ComposedFreshAdmission;
use crate::durability::horse_rental::HorseStationChannel;
use crate::durability::horse_rental::rules;
use crate::foundation::MovementLocalPosition;
use crate::monster_owner_cycle::respawn_job::RespawnExit;
use oteryn_foundation::CancellationToken;
use sha2::{Digest, Sha256};
mod catch;
mod expiry;
pub(in crate::gameplay_transport) use expiry::ExpiryClocks;
fn draw(seed: [u8; 32], domain: u16, bound: u32) -> Option<u32> {
    if ![rules::RNG_SPAWN_X, rules::RNG_SPAWN_Y].contains(&domain) || bound == 0 {
        return None;
    }
    let limit = ((1u128 << 64) / u128::from(bound)) * u128::from(bound);
    for attempt in 0i32..16 {
        let bytes = Sha256::new()
            .chain_update(b"oteryn:horse-station-rng:v1")
            .chain_update(i32::from(domain).to_be_bytes())
            .chain_update(seed)
            .chain_update(attempt.to_be_bytes())
            .finalize();
        let value = u64::from_be_bytes(bytes[..8].try_into().ok()?);
        if u128::from(value) < limit {
            return u32::try_from(value % u64::from(bound)).ok();
        }
    }
    None
}
#[derive(Default)]
struct Run {
    generation: Option<i64>,
    ended: bool,
    installed: Option<(i64, bool, i16)>,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn run_horse_station(
        &self,
        stop: &CancellationToken,
    ) -> RespawnExit {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        let mut run = Run::default();
        loop {
            if super::world_respawns::stopping_first(interval.tick(), stop)
                .await
                .is_none()
            {
                return RespawnExit::Shutdown;
            }
            let Some(_permit) = self.spell_lane_permit().await else {
                continue;
            };
            let Some(map) = self.bundle_map() else {
                continue;
            };
            let binding = self.runtime.lock().await.binding();
            let Ok(world) = self
                .root
                .refresh_horse_station(
                    self.character,
                    self.holder,
                    HorseStationChannel {
                        world: self.world_id,
                        channel: self.channel_id,
                        generation: binding.scope_generation().get(),
                    },
                )
                .await
            else {
                continue;
            };
            let mut runtime = self.runtime.lock().await;
            if runtime.binding() != binding || runtime.owner_fence().is_err() {
                return RespawnExit::StaleOwner;
            }
            // A source escape removes stable residents, not escaped wild horses. The
            // surviving raid actors remain ordinary/tameable after catch or daily expiry.
            if !world.ended && (run.generation != Some(world.generation) || run.ended) {
                let Ok(census) = runtime.positioned_actor_census() else {
                    return RespawnExit::StaleOwner;
                };
                for (actor, position, session) in census {
                    let at = position.position();
                    if session.is_none()
                        && at.floor == 7
                        && rules::CLEAR_STABLE_AREAS.iter().any(|area| {
                            at.x >= area.min_x
                                && at.x <= area.max_x
                                && at.y >= area.min_y
                                && at.y <= area.max_y
                        })
                        && [
                            "oteryn:creature.horse",
                            "oteryn:creature.grey_horse",
                            "oteryn:creature.brown_horse",
                        ]
                        .iter()
                        .any(|key| runtime.matches_live_creature_identity(actor, key.as_bytes()))
                    {
                        let _ = runtime.remove_arena_opponent(actor);
                    }
                }
            }
            run.generation = Some(world.generation);
            run.ended = world.ended;
            let source_slot = (world.generation, world.ended, world.wave);
            if !world.claimed || run.installed == Some(source_slot) {
                continue;
            }
            let Some(doors) = self.door.try_lock().ok() else {
                continue;
            };
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            let areas = if world.ended {
                &rules::STABLE_AREAS
            } else {
                &rules::RAID_AREAS
            };
            for (area_index, area) in areas.iter().enumerate() {
                let keys: Vec<&str> = if world.ended {
                    rules::STABLE_CREATURES[area_index].to_vec()
                } else {
                    (0..3).flat_map(|_| rules::WAVE_CREATURES).collect()
                };
                for (ordinal, key) in keys.into_iter().enumerate() {
                    let (Ok(ordinal), Ok(area_index)) =
                        (u32::try_from(ordinal), u32::try_from(area_index))
                    else {
                        continue;
                    };
                    let seed = Sha256::new()
                        .chain_update(self.world_id.as_bytes())
                        .chain_update(self.channel_id.as_bytes())
                        .chain_update(world.generation.to_be_bytes())
                        .chain_update(world.wave.to_be_bytes())
                        .chain_update(area_index.to_be_bytes())
                        .chain_update(ordinal.to_be_bytes())
                        .finalize()
                        .into();
                    let (Ok(width), Ok(height)) = (
                        u32::try_from(area.max_x - area.min_x + 1),
                        u32::try_from(area.max_y - area.min_y + 1),
                    ) else {
                        continue;
                    };
                    let (Some(x), Some(y)) = (
                        draw(seed, rules::RNG_SPAWN_X, width),
                        draw(seed, rules::RNG_SPAWN_Y, height),
                    ) else {
                        continue;
                    };
                    let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
                        continue;
                    };
                    let at = MovementLocalPosition {
                        x: area.min_x + x,
                        y: area.min_y + y,
                        floor: 7,
                    };
                    let admits = |owner: &crate::foundation::ChannelRuntimeV1, position| {
                        Some(
                            crate::movement::relocate::creature_cell_admissible(
                                self.movement_cells,
                                &closed,
                                position,
                            ) && crate::movement::relocate::creature_cell_admissible(
                                self.movement_cells,
                                doors.blocking_cells(),
                                position,
                            ) && owner.native_summon_cell_free(position),
                        )
                    };
                    let _ = runtime.spawn_arena_opponent(key, at, admits);
                }
            }
            run.installed = Some(source_slot);
        }
    }
}
