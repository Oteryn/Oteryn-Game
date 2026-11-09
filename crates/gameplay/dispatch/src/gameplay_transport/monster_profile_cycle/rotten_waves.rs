//! M26 current native think -> source wave -> native spawn/cleanup/AI registration.
use super::*;
use crate::ai_think::{CREATURE_THINK_INTERVAL_MILLIS, CreatureAiTable, ThinkReport};
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
};
use crate::movement::monster_map::{CurrentMonsterMap, MonsterMap};
use oteryn_gameplay_encounters::bosses::rotten_blood::{
    RoomPosition,
    elder_bloodjaw_think::{ELDER_BLOODJAW_KEY, is_rotten_blood_boss},
    mushroom_think::MUSHROOM_KEY,
    native_bloodjaw::remove_orphan as remove_orphan_bloodjaw,
    native_mushroom::remove_orphan,
    native_wave_spawn::{NativeWaveSpawnMap, NativeWaveSpawnOwner},
    summon_wave::{
        SummonNeighbour, SummonPlacement, SummonWaveEffect, SummonWaveFacts, SummonWaveOwner,
    },
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, deterministic_decision_u64};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Default)]
pub(in crate::gameplay_transport) struct RoomWaveStates {
    owners: BTreeMap<[u8; 16], RoomWave>,
    source_attempt: Option<SourceRoomBinding>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SourceRoomBinding {
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    content: [u8; 32],
}
impl RoomWaveStates {
    /// Observe source bosses and minions independently of the two wave planners.
    /// Keep minion callbacks alive after departure, only for this exact runtime binding.
    pub(super) fn observe_source_attempt(&mut self, runtime: &ChannelRuntimeV1) -> bool {
        if runtime.owner_fence().is_err() {
            return false;
        }
        let binding = runtime.binding();
        let current = SourceRoomBinding {
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            content: runtime.content_pin().server_artifact_digest(),
        };
        if self.source_attempt != Some(current) {
            self.source_attempt = None;
        }
        if !matches!(current.scope, RuntimeScopeRefV1::Instance { .. }) {
            return false;
        }
        if runtime.positioned_melee_census().is_ok_and(|rows| {
            rows.iter().any(|(actor, _, _)| {
                runtime.companion_snapshot(*actor).is_ok_and(|row| {
                    let key = row.state.policy.definition_key.as_str();
                    is_rotten_blood_boss(key) || matches!(key, ELDER_BLOODJAW_KEY | MUSHROOM_KEY)
                })
            })
        }) {
            self.source_attempt = Some(current);
        }
        self.source_attempt == Some(current)
    }

    #[cfg(test)]
    pub(in crate::gameplay_transport) fn has_attempts(&self) -> bool {
        !self.owners.is_empty()
    }
}
impl std::fmt::Debug for RoomWaveStates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoomWaveStates")
            .field("attempts", &self.owners.len())
            .field("source_attempt", &self.source_attempt.is_some())
            .finish()
    }
}
struct RoomWave {
    actor: ExactActorRef,
    rules: SummonWaveOwner,
    physical: NativeWaveSpawnOwner,
    handles: BTreeMap<u64, ExactActorRef>,
    next_handle: u64,
    sequence: u64,
}

pub(super) fn supports(key: &str) -> bool {
    matches!(key, "oteryn:creature.ichgahal" | "oteryn:creature.murcion")
}

pub(super) fn has_boss(runtime: &ChannelRuntimeV1) -> bool {
    runtime.positioned_melee_census().is_ok_and(|rows| {
        rows.iter().any(|(actor, _, _)| {
            runtime
                .companion_snapshot(*actor)
                .is_ok_and(|row| supports(&row.state.policy.definition_key))
        })
    })
}

/// Warm all source candidates before the synchronous native owner consumes any result.
pub(super) async fn warm(runtime: &ChannelRuntimeV1, map: &mut CurrentMonsterMap<'_, '_>) {
    let Ok(rows) = runtime.positioned_melee_census() else {
        return;
    };
    for (actor, position, _) in rows {
        if !runtime
            .companion_snapshot(actor)
            .is_ok_and(|row| supports(&row.state.policy.definition_key))
        {
            continue;
        }
        let centre = position.position();
        for dy in -2..=2 {
            for dx in -2..=2 {
                let (Some(x), Some(y)) = (centre.x.checked_add(dx), centre.y.checked_add(dy))
                else {
                    continue;
                };
                let _ = map
                    .read(
                        runtime,
                        MovementLocalPosition {
                            x,
                            y,
                            floor: centre.floor,
                        },
                    )
                    .await;
            }
        }
    }
}

struct WaveMap<'a> {
    map: &'a MonsterMap,
    root: GameplayDecisionRoot,
}
impl NativeWaveSpawnMap for WaveMap<'_> {
    fn resolve(
        &mut self,
        runtime: &ChannelRuntimeV1,
        boss: ExactActorRef,
        effect: &SummonWaveEffect,
        sequence: u64,
        ordinal: u8,
        stamp: crate::foundation::RuntimeWorkStamp,
    ) -> Option<(MovementLocalPosition, bool)> {
        if !self.map.matches(runtime) || !runtime.owner_fence().ok()?.accepts_stamp(stamp) {
            return None;
        }
        let SummonWaveEffect::Spawn {
            around, placement, ..
        } = effect
        else {
            return None;
        };
        let (offset, tries, close) = match placement {
            SummonPlacement::Close { offset_tiles } => (*offset_tiles, 10u64, true),
            SummonPlacement::Random { offset_tiles } => (*offset_tiles, 1u64, false),
        };
        let width = u64::from(offset) * 2 + 1;
        let digest = Sha256::digest(
            [
                boss.placement_identity().as_slice(),
                &sequence.to_be_bytes(),
                &[ordinal],
            ]
            .concat(),
        );
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        let occurrence = DecisionOccurrenceId::from_bytes(id);
        for attempt in 0..tries {
            let draw = deterministic_decision_u64(
                &self.root,
                occurrence,
                "M26_ROTTEN_WAVE_POSITION",
                attempt,
            )
            .ok()?;
            let x = around
                .x
                .checked_add(i32::try_from(draw % width).ok()? - i32::from(offset))?;
            let y = around
                .y
                .checked_add(i32::try_from((draw / width) % width).ok()? - i32::from(offset))?;
            let at = MovementLocalPosition {
                x,
                y,
                floor: around.floor,
            };
            if !self.map.known(at) {
                return None;
            }
            let allowed = self.map.admits(at);
            // Source getClosePosition uses Tile:isWalkable(), whose projectile
            // refusal is independent of movement. Random placement and the
            // unconditional source-centre fallback retain creation admission.
            if close && !self.map.admits_close_spawn_candidate(runtime, at)? {
                continue;
            }
            return Some((at, allowed));
        }
        let at = MovementLocalPosition {
            x: around.x,
            y: around.y,
            floor: around.floor,
        };
        self.map.known(at).then(|| (at, self.map.admits(at)))
    }
}

impl RoomWaveStates {
    #[cfg(test)]
    pub(in crate::gameplay_transport) fn wave_children(
        &self,
        actor: ExactActorRef,
    ) -> Vec<ExactActorRef> {
        self.owners
            .get(&actor.placement_identity())
            .filter(|row| row.actor == actor)
            .map(|row| row.handles.values().copied().collect())
            .unwrap_or_default()
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "current native think, map and durable room facts stay explicit"
    )]
    pub(super) fn consume(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        table: &mut CreatureAiTable,
        native: &NativeGameplayState,
        actor: ExactActorRef,
        report: &ThinkReport,
        map: &MonsterMap,
        room: &QualifiedNativeEntryRoom,
        taints: &[(GameSessionId, u8)],
        now: u64,
    ) -> Result<(), ()> {
        let Ok(snapshot) = runtime.companion_snapshot(actor) else {
            return Ok(());
        };
        let key = snapshot.state.policy.definition_key.as_str();
        #[cfg(test)]
        if supports(key) {
            eprintln!(
                "M26_WAVE_THINK sequence={} idle={} over_budget={} refusal={:?}",
                report.sequence, report.idle, report.over_budget, report.refusal
            );
        }
        // An attack-schedule refusal does not cancel a committed source onThink.
        // The map and spawn owners independently prove this callback's current work.
        if matches!(key, MUSHROOM_KEY | ELDER_BLOODJAW_KEY) && !report.over_budget {
            let removed = if key == MUSHROOM_KEY {
                remove_orphan(runtime, actor, native.source_digest())
            } else {
                remove_orphan_bloodjaw(runtime, actor, native.source_digest())
            }
            .map_err(|_| ())?;
            if removed {
                table.remove(actor);
            }
            return Ok(());
        }
        if !supports(key) || report.over_budget {
            return Ok(());
        }
        self.owners
            .retain(|_, row| runtime.contains_live_creature(row.actor));
        let id = actor.placement_identity();
        if !self.owners.get(&id).is_some_and(|row| row.actor == actor) {
            self.owners.insert(
                id,
                RoomWave {
                    actor,
                    rules: SummonWaveOwner::for_boss(key).ok_or(())?,
                    physical: NativeWaveSpawnOwner::for_boss(runtime, actor).ok_or(())?,
                    handles: BTreeMap::new(),
                    next_handle: 1,
                    sequence: 0,
                },
            );
        }
        let owner = self.owners.get_mut(&id).ok_or(())?;
        if report.sequence <= owner.sequence {
            return Ok(());
        }
        let position = runtime
            .read_actor_position(actor)
            .map_err(|_| ())?
            .position();
        let boss = RoomPosition {
            x: position.x,
            y: position.y,
            floor: position.floor,
        };
        let rows = runtime.positioned_actor_census().map_err(|_| ())?;
        let mut monsters = Vec::new();
        let mut maximum = 0;
        // Source fixed centres use the captured source frame, not the moving boss.
        let source_centre = if key == "oteryn:creature.ichgahal" {
            [33008, 32334, 15]
        } else {
            [33008, 32367, 15]
        };
        if room.source_world().is_none() {
            return Err(());
        }
        let centre = MovementLocalPosition {
            x: source_centre[0] - 5854,
            y: source_centre[1] - 5298,
            floor: -15,
        };
        for (current, at, session) in rows {
            let at = at.position();
            if let Some(session) = session {
                if at.floor == centre.floor
                    && at.x.abs_diff(centre.x) <= 15
                    && at.y.abs_diff(centre.y) <= 15
                {
                    let taint = taints
                        .iter()
                        .find(|(member, _)| *member == session)
                        .ok_or(())?
                        .1;
                    maximum = maximum.max(taint);
                }
            } else if runtime.contains_live_creature(current) {
                let row = runtime.companion_snapshot(current).map_err(|_| ())?;
                monsters.push((
                    row.state.policy.definition_key.clone(),
                    RoomPosition {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                    },
                ));
            }
        }
        let neighbours = monsters
            .iter()
            .map(|(key, position)| SummonNeighbour {
                creature_key: key,
                position: *position,
            })
            .collect::<Vec<_>>();
        let root = GameplayDecisionRoot::from_bytes(native.source_digest());
        let digest = Sha256::digest(
            [
                actor.placement_identity().as_slice(),
                &report.sequence.to_be_bytes(),
            ]
            .concat(),
        );
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&digest[..16]);
        let interval = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "M26_ROTTEN_WAVE_INTERVAL",
            0,
        )
        .map_err(|_| ())?;
        let mut next = owner.rules.clone();
        let effects = next
            .think(
                &SummonWaveFacts {
                    boss,
                    interval_ms: CREATURE_THINK_INTERVAL_MILLIS,
                    neighbours: &neighbours,
                    max_room_taint: maximum,
                    mechanics_disabled: false,
                    bloodjaw_spawn_paused: false,
                },
                || 8_000 + interval % 1_001,
            )
            .map_err(|_| ())?;
        owner.next_handle.checked_add(3).ok_or(())?;
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        // All source old-wave removals precede replacement spawns. A refusal
        // leaves the rules unadvanced, so dead exact children remain safe retries.
        for effect in &effects {
            if let SummonWaveEffect::RemoveSummon { actor: handle } = effect
                && let Some(child) = owner.handles.get(handle)
            {
                owner
                    .physical
                    .retire_wave_child(runtime, stamp, *child)
                    .map_err(|_| ())?;
                table.remove(*child);
            }
        }
        owner
            .handles
            .retain(|_, child| runtime.contains_live_creature(*child));
        let mut placements = WaveMap { map, root };
        for (ordinal, effect) in effects
            .iter()
            .filter(|effect| matches!(effect, SummonWaveEffect::Spawn { .. }))
            .enumerate()
        {
            let receipt = owner.physical.spawn_current(
                runtime,
                stamp,
                report.sequence,
                u8::try_from(ordinal).map_err(|_| ())?,
                effect,
                &mut placements,
            );
            #[cfg(test)]
            if let Err(error) = &receipt {
                eprintln!("M26_WAVE_SPAWN ordinal={ordinal} error={error:?} effect={effect:?}");
            }
            let Ok(receipt) = receipt else {
                continue;
            }; // source createMonster may fail placement
            let SummonWaveEffect::Spawn {
                creature_key,
                placement,
                ..
            } = effect
            else {
                continue;
            };
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|row| row.profile.target.key == *creature_key)
                .ok_or(())?;
            if table.state(receipt.child).is_none()
                && table
                    .admit_native_creature(
                        runtime,
                        native,
                        receipt.child,
                        &record.profile.target,
                        now,
                    )
                    .is_err()
            {
                #[cfg(test)]
                eprintln!("M26_WAVE_AI_REGISTRATION failed creature={creature_key}");
                runtime
                    .remove_current_native_encounter_subject(
                        stamp,
                        receipt.child,
                        creature_key,
                        native.source_digest(),
                    )
                    .map_err(|_| ())?;
                continue;
            }
            if matches!(placement, SummonPlacement::Random { .. }) {
                let handle = owner.next_handle;
                owner.next_handle += 1;
                owner.handles.insert(handle, receipt.child);
                next.record_wave_summon(handle);
            }
        }
        owner.rules = next;
        owner.sequence = report.sequence;
        Ok(())
    }
}
