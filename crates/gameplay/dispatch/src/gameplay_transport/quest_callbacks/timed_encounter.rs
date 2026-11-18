//! Bounded source timers share the Channel source-owner loop and native carrier.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::map::{facts::QuestCallbackProjection, view::EntryFacts};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) const MAX_TIMED_ENCOUNTERS: usize = 64;

pub(super) struct TimedEncounter {
    callback: Callback,
    key: u64,
    revision: u64,
    content: [u8; 32],
    map: [u8; 32],
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    spawn_at: u64,
    reset_at: u64,
    warn_at: Option<u64>,
    attempted_spawn: bool,
    spawned: Vec<ExactActorRef>,
    switches: Vec<(u64, u64, EntryFacts)>,
    participants: Vec<(ExactActorRef, GameSessionId)>,
}
impl TimedEncounter {
    pub(super) fn source_key(&self) -> u64 {
        self.key
    }

    pub(super) fn occupies(&self, min: Position, max: Position) -> bool {
        self.switches.len() == 4
            && self
                .callback
                .effects
                .encounter
                .as_ref()
                .is_some_and(|encounter| encounter.room_min == min && encounter.room_max == max)
    }

    pub(super) fn next_due_us(&self) -> Option<u64> {
        if !self.switches.is_empty() && self.switches.len() < 4 {
            return None;
        }
        Some(if self.attempted_spawn {
            self.warn_at.unwrap_or(self.reset_at)
        } else {
            self.spawn_at
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the held native USE occurrence owns one source timer"
    )]
    pub(super) fn schedule_timed_quest_encounter(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        after: EntryFacts,
    ) -> UseDisposition {
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        let Some(timing) = encounter.timing else {
            return UseDisposition::Rejected;
        };
        let personal =
            oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(callback);
        if callback.lever.is_none() && !personal {
            return UseDisposition::Rejected;
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return UseDisposition::Rejected;
        };
        // Bind the authoritative artifact before looking up a native creature. A
        // source switch must not depend on an unrelated summon/AI initialization.
        if native.install_companion_policies(runtime).is_err() {
            return UseDisposition::Rejected;
        }
        if !encounter.players.is_empty()
            || states
                .get(runtime, actor, session)
                .is_none_or(|state| state.character_facts().level < encounter.minimum_level)
            || runtime.owner_fence().is_err()
            || callback.lever.is_some_and(|pair| {
                map.facts.quest_callback_replacement(key, pair[0]).is_none()
                    || map.facts.quest_callback_replacement(key, pair[1]).is_none()
            })
            || encounter
                .monsters
                .iter()
                .any(|monster| runtime.companion_policy(&monster.creature).is_err())
        {
            return UseDisposition::Rejected;
        }
        let now = self.owner_now().get();
        let (Some(spawn_at), Some(reset_at), Some(next_revision)) = (
            now.checked_add(u64::from(timing.spawn_after_ms) * 1000),
            now.checked_add(u64::from(timing.reset_after_ms) * 1000),
            revision.checked_add(1),
        ) else {
            return UseDisposition::Exhausted;
        };
        let Ok(mut pending) = self.states.quest_callbacks.timed.lock() else {
            return UseDisposition::Rejected;
        };
        let coordination = encounter.switches.as_ref();
        let mut walls = Vec::new();
        let mut participants = Vec::new();
        if let Some(coordinated) = coordination {
            let Some((position, _)) = crate::map::view::placement_of(key) else {
                return UseDisposition::Rejected;
            };
            let Some(station) = coordinated
                .stations
                .iter()
                .find(|station| tile(station.lever) == position)
            else {
                return UseDisposition::Rejected;
            };
            if runtime
                .read_actor_position(actor)
                .ok()
                .is_none_or(|pos| pos.position() != local(station.stand))
            {
                return UseDisposition::NotPossible;
            }
            let Some(facts) = map.facts.movement_item_facts(station.appearance) else {
                return UseDisposition::Rejected;
            };
            for position in station.walls.map(tile) {
                if map.facts.house_tile(position)
                    || map
                        .overlay
                        .base()
                        .tile(position.x, position.y, position.floor)
                        .is_none()
                {
                    return UseDisposition::Rejected;
                }
                walls.push((position, facts));
            }
            let Ok(census) = runtime.positioned_melee_census() else {
                return UseDisposition::Rejected;
            };
            participants.extend(census.into_iter().filter_map(|(actor, position, session)| {
                coordinated
                    .stations
                    .iter()
                    .any(|station| position.position() == local(station.stand))
                    .then_some((actor, session?))
            }));
        }
        let existing = coordination.and_then(|_| {
            pending
                .iter_mut()
                .find(|memo| memo.callback.effects.encounter.as_ref() == Some(encounter))
        });
        let projection = coordination.map(|_| QuestCallbackProjection::EncounterWalls {
            walls,
            clear: Vec::new(),
        });
        let restore = callback
            .lever
            .and_then(|pair| map.facts.quest_callback_replacement(key, pair[0]));
        if let Some(memo) = existing {
            let Some(restore) = restore else {
                return UseDisposition::Rejected;
            };
            if memo.switches.len() >= 4
                || memo.switches.iter().any(|(source, _, _)| {
                    crate::map::view::placement_of(*source).map(|(tile, _)| tile)
                        == crate::map::view::placement_of(key).map(|(tile, _)| tile)
                })
            {
                return UseDisposition::Committed;
            }
            if !map.facts.apply_quest_callback(
                key,
                revision,
                after,
                &[],
                &[],
                false,
                projection.as_ref(),
            ) {
                return UseDisposition::StaleState;
            }
            memo.switches.push((key, next_revision, restore));
            if memo.switches.len() == 4 {
                memo.spawn_at = spawn_at;
                memo.reset_at = reset_at;
                memo.warn_at = Some(now + (reset_at - now) / 2);
                memo.participants = participants;
                states.owner_wake.notify_one();
            }
            return UseDisposition::Committed;
        }
        if pending.len() >= MAX_TIMED_ENCOUNTERS
            || pending.iter().any(|pending| pending.key == key)
            || pending.try_reserve(1).is_err()
        {
            return UseDisposition::Exhausted;
        }
        let switches = if coordination.is_some() {
            let Some(restore) = restore else {
                return UseDisposition::Rejected;
            };
            vec![(key, next_revision, restore)]
        } else {
            Vec::new()
        };
        if !map.facts.apply_quest_callback(
            key,
            revision,
            after,
            &[],
            &[],
            false,
            projection.as_ref(),
        ) {
            return UseDisposition::Rejected;
        }
        pending.push(TimedEncounter {
            callback: callback.clone(),
            key,
            revision: next_revision,
            content: runtime.content_pin().server_artifact_digest(),
            map: map.overlay.base().digest(),
            scope: runtime.encounter_owner_scope(),
            spawn_at: if switches.is_empty() {
                spawn_at
            } else {
                u64::MAX
            },
            reset_at: if switches.is_empty() {
                reset_at
            } else {
                u64::MAX
            },
            warn_at: None,
            attempted_spawn: false,
            spawned: Vec::new(),
            switches,
            participants: Vec::new(),
        });
        states.owner_wake.notify_one();
        UseDisposition::Committed
    }

    pub(in crate::gameplay_transport) async fn drain_quest_encounter_deadlines(&self) {
        let Some(_permit) = self.spell_lane_permit().await else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let Some(map) = self.bundle_map() else { return };
        if runtime.owner_fence().is_err()
            || self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .is_none_or(|native| {
                    native.source_digest() != runtime.content_pin().server_artifact_digest()
                })
        {
            return;
        }
        let mut states = self.states.spell_states.lock().await;
        let Ok(mut pending) = self.states.quest_callbacks.timed.lock() else {
            return;
        };
        let now = self.owner_now().get();
        let mut index = 0;
        while index < pending.len() {
            let memo = &mut pending[index];
            if memo.content != runtime.content_pin().server_artifact_digest()
                || memo.map != map.overlay.base().digest()
                || memo.scope != runtime.encounter_owner_scope()
                || map.facts.object_revision(memo.key) != memo.revision
                || memo
                    .switches
                    .iter()
                    .any(|(key, revision, _)| map.facts.object_revision(*key) != *revision)
            {
                pending.remove(index);
                continue;
            }
            if memo.next_due_us().is_none_or(|due| now < due) {
                index += 1;
                continue;
            }
            let Some(encounter) = &memo.callback.effects.encounter else {
                pending.remove(index);
                continue;
            };
            if now < memo.reset_at && memo.warn_at.is_some_and(|warning| now >= warning) {
                memo.warn_at = None;
                for &(actor, session) in &memo.participants {
                    if runtime
                        .read_actor_position(actor)
                        .ok()
                        .and_then(|at| native(at.position()))
                        .is_some_and(|at| in_room(encounter, at))
                    {
                        self.states.chat.push_quest_feedback(
                            session,
                            "You have 5 minutes from now on until you get teleported out.",
                        );
                    }
                }
                index += 1;
                continue;
            }
            let Ok(census) = runtime.current_live_creature_census() else {
                index += 1;
                continue;
            };
            if now >= memo.reset_at {
                if let Some(coordinated) = &encounter.switches {
                    let Some((_, _, restore)) = memo.switches.first() else {
                        pending.remove(index);
                        continue;
                    };
                    let replacements: Vec<_> = memo
                        .switches
                        .iter()
                        .skip(1)
                        .map(|(source, _, facts)| (*source, *facts, true))
                        .collect();
                    let projection = QuestCallbackProjection::EncounterWalls {
                        walls: Vec::new(),
                        clear: memo.switches.iter().map(|(source, _, _)| *source).collect(),
                    };
                    let closed = crate::map::floor::BundleDoors(&map.facts);
                    let mut moves = Vec::new();
                    let mut proofs = Vec::new();
                    for &(actor, session) in &memo.participants {
                        let Ok(before) = runtime.read_actor_position(actor) else {
                            continue;
                        };
                        let Some(position) = native(before.position()) else {
                            continue;
                        };
                        if !in_room(encounter, position) {
                            continue;
                        }
                        let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
                            &runtime,
                            &self.movement_cells,
                            actor,
                            session,
                            before,
                            local(coordinated.exit),
                            &closed,
                        ) else {
                            continue;
                        };
                        moves.push((actor, session, before));
                        proofs.push(proof);
                    }
                    let reset = || {
                        map.facts.apply_quest_callback(
                            memo.key,
                            memo.revision,
                            *restore,
                            &[],
                            &replacements,
                            true,
                            Some(&projection),
                        )
                    };
                    let snapshots = if proofs.is_empty() {
                        if !reset() {
                            index += 1;
                            continue;
                        }
                        Vec::new()
                    } else {
                        let Ok(snapshots) = runtime.commit_source_steps_and_project(proofs, reset)
                        else {
                            index += 1;
                            continue;
                        };
                        snapshots
                    };
                    for ((actor, session, before), after) in moves.into_iter().zip(snapshots) {
                        states.stage_special_movement_step(
                            &runtime,
                            map,
                            actor,
                            session,
                            now,
                            Some(before.position()),
                            after.position(),
                            None,
                        );
                        states.stage_underwater_step(
                            &runtime,
                            actor,
                            session,
                            now,
                            underwater(map, before.position()),
                            underwater(map, after.position()),
                        );
                        map.facts.emit_movement_effect(tile(coordinated.exit), 11);
                    }
                    super::personal_trial::cleanup(&mut runtime, &mut memo.spawned);
                    pending.remove(index);
                    continue;
                }

                if oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(
                    &memo.callback,
                ) {
                    // Personal trials own only their exact spawns, never unrelated room actors.
                    super::personal_trial::cleanup(&mut runtime, &mut memo.spawned);
                    pending.remove(index);
                    continue;
                }
                let Some(pair) = memo.callback.lever else {
                    pending.remove(index);
                    continue;
                };
                let Some(restore) = map.facts.quest_callback_replacement(memo.key, pair[0]) else {
                    index += 1;
                    continue;
                };
                for actor in census {
                    let in_room = runtime
                        .read_actor_position(actor)
                        .ok()
                        .and_then(|snapshot| native(snapshot.position()))
                        .is_some_and(|position| {
                            position.floor == encounter.room_min.floor
                                && (encounter.room_min.x..=encounter.room_max.x)
                                    .contains(&position.x)
                                && (encounter.room_min.y..=encounter.room_max.y)
                                    .contains(&position.y)
                        });
                    if in_room {
                        let _ = runtime.remove_encounter_actor(actor);
                    }
                }
                if map.facts.apply_quest_callback(
                    memo.key,
                    memo.revision,
                    restore,
                    &[],
                    &[],
                    true,
                    None,
                ) {
                    pending.remove(index);
                } else {
                    index += 1;
                }
                continue;
            }
            // Source Game.createMonster attempts once at its due instant. An occupied/full
            // room does not invent a successful spawn or retry after the source deadline.
            let mut monsters = Vec::new();
            let mut admissible = true;
            for monster in &encounter.monsters {
                let at = local(monster.position);
                if self.movement_cells.index().lookup(
                    self.movement_cells.scope(),
                    LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ) != Ok(CollisionClass::Walkable)
                    || map.facts.quest_callback_blocked(
                        map.overlay.base(),
                        tile(monster.position),
                        false,
                    )
                    || runtime.positioned_melee_census().map_or(true, |census| {
                        census
                            .iter()
                            .any(|(_, position, _)| position.position() == at)
                    })
                {
                    admissible = false;
                    break;
                }
                let Ok(policy) = runtime.companion_policy(&monster.creature) else {
                    admissible = false;
                    break;
                };
                monsters.push((monster.creature.as_str(), policy.maximum_health, at));
            }
            if admissible && let Ok(spawned) = runtime.commit_quest_creature_spawn(&monsters) {
                memo.spawned = spawned;
            }
            memo.attempted_spawn = true;
            index += 1;
        }
        // Death/despawn and source cleanup release only their source-owned ladder.
        super::monsters::settle_authored_monster_puzzles(&runtime, map);
    }
}

fn in_room(encounter: &world_callbacks::Encounter, at: TilePos) -> bool {
    at.floor == encounter.room_min.floor
        && (encounter.room_min.x..=encounter.room_max.x).contains(&at.x)
        && (encounter.room_min.y..=encounter.room_max.y).contains(&at.y)
}
