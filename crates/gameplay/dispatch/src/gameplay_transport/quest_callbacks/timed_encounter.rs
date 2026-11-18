//! Bounded source timers share the Channel source-owner loop and native carrier.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::map::view::EntryFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) const MAX_TIMED_ENCOUNTERS: usize = 64;

pub(super) struct TimedEncounter {
    callback: Callback,
    key: u64,
    revision: u64,
    content: [u8; 32],
    map: [u8; 32],
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    participant: Option<(ExactActorRef, GameSessionId)>,
    roster: Vec<(ExactActorRef, GameSessionId)>,
    switches: Vec<(u64, u64, EntryFacts)>,
    spawn_at: u64,
    reset_at: u64,
    attempted_spawn: bool,
    spawned: Vec<ExactActorRef>,
}
impl TimedEncounter {
    pub(super) fn group(
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        callback: &Callback,
        source: (u64, u64),
        now: u64,
        roster: Vec<(ExactActorRef, GameSessionId)>,
    ) -> Option<Self> {
        let transfer = callback.effects.timed_group_transfer.as_ref()?;
        Some(Self {
            callback: callback.clone(),
            key: source.0,
            revision: source.1.checked_add(1)?,
            content: runtime.content_pin().server_artifact_digest(),
            map: map.overlay.base().digest(),
            scope: runtime.encounter_owner_scope(),
            participant: None,
            roster,
            switches: Vec::new(),
            spawn_at: now,
            reset_at: now.checked_add(u64::from(transfer.after_ms) * 1_000)?,
            attempted_spawn: true,
            spawned: Vec::new(),
        })
    }

    pub(super) fn bind_switches(&mut self, switches: Vec<(u64, u64, EntryFacts)>) {
        self.switches = switches;
    }

    pub(super) fn bind_spawns(&mut self, spawned: Vec<ExactActorRef>) {
        self.spawned = spawned;
    }

    pub(super) fn source_key(&self) -> u64 {
        self.key
    }

    pub(super) fn contains_participant(
        &self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        position: MovementLocalPosition,
        now: u64,
    ) -> bool {
        (now < self.reset_at || self.callback.effects.timed_group_transfer.is_some())
            && self.scope == runtime.encounter_owner_scope()
            && self.content == runtime.content_pin().server_artifact_digest()
            && self.map == map.overlay.base().digest()
            && self.revision == map.facts.object_revision(self.key)
            && (self.roster.is_empty() || self.roster.contains(&(actor, session)))
            && self
                .participant
                .is_none_or(|participant| participant == (actor, session))
            && self
                .callback
                .effects
                .encounter
                .as_ref()
                .is_some_and(|room| super::encounter::room_contains(room, position))
    }

    pub(super) fn next_due_us(&self) -> u64 {
        if self.attempted_spawn {
            self.reset_at
        } else {
            self.spawn_at
        }
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
        if pending.len() >= MAX_TIMED_ENCOUNTERS
            || pending.iter().any(|pending| pending.key == key)
            || pending.try_reserve(1).is_err()
        {
            return UseDisposition::Exhausted;
        }
        if !map
            .facts
            .apply_quest_callback(key, revision, after, &[], &[], false, None)
        {
            return UseDisposition::Rejected;
        }
        pending.push(TimedEncounter {
            callback: callback.clone(),
            key,
            revision: next_revision,
            content: runtime.content_pin().server_artifact_digest(),
            map: map.overlay.base().digest(),
            scope: runtime.encounter_owner_scope(),
            participant: personal.then_some((actor, session)),
            roster: Vec::new(),
            switches: Vec::new(),
            spawn_at,
            reset_at,
            attempted_spawn: false,
            spawned: Vec::new(),
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
            {
                pending.remove(index);
                continue;
            }
            if now < memo.next_due_us() {
                index += 1;
                continue;
            }
            let Some(encounter) = &memo.callback.effects.encounter else {
                pending.remove(index);
                continue;
            };
            if let Some(transfer) = &memo.callback.effects.timed_group_transfer {
                if self.expire_group(&mut runtime, &mut states, map, memo, transfer.destination) {
                    pending.remove(index);
                } else {
                    index += 1;
                }
                continue;
            }
            let Ok(census) = runtime.current_live_creature_census() else {
                index += 1;
                continue;
            };
            if now >= memo.reset_at {
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

    // Crystal 201189f8 actions_lord_of_elements_lever.lua:63-99,168-180.
    // The deadline acts on the captured roster still inside the source room.
    fn expire_group(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        memo: &TimedEncounter,
        destination: Position,
    ) -> bool {
        let (Some(room), Some([closed_appearance, _])) =
            (&memo.callback.effects.encounter, memo.callback.lever)
        else {
            return false;
        };
        let Some(restore) = map
            .facts
            .quest_callback_replacement(memo.key, closed_appearance)
        else {
            return false;
        };
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let mut participants = Vec::new();
        let mut proofs = Vec::new();
        for &(actor, session) in &memo.roster {
            if states.is_dead(actor) || runtime.bound_attacker_lease(actor, session).is_err() {
                continue;
            }
            let Ok(expected) = runtime.read_actor_position(actor) else {
                continue;
            };
            if !super::encounter::room_contains(room, expected.position()) {
                continue;
            }
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
                runtime,
                self.movement_cells(),
                actor,
                session,
                expected,
                local(destination),
                &doors,
            ) else {
                return false;
            };
            participants.push((actor, session, expected.position()));
            proofs.push(proof);
        }
        let Ok(landed) = runtime.commit_quest_relocation(proofs, || {
            if memo.switches.is_empty() {
                map.facts.apply_quest_callback(
                    memo.key,
                    memo.revision,
                    restore,
                    &[],
                    &[],
                    true,
                    None,
                )
            } else {
                map.facts.reset_quest_switch_walls(&memo.switches)
            }
        }) else {
            return false;
        };
        for &creature in &memo.spawned {
            let _ = runtime.remove_encounter_actor(creature);
        }
        let now = self.owner_now().get();
        for ((actor, session, before), after) in participants.into_iter().zip(landed) {
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                now,
                Some(before),
                after.position(),
                None,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                now,
                underwater(map, before),
                underwater(map, after.position()),
            );
            map.facts.emit_movement_effect(tile(destination), 11);
        }
        true
    }
}
