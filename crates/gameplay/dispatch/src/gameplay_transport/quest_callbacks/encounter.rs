//! Source parties are the players standing on the exact staging tiles, not a social-party id.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::EntryFacts;

pub(in crate::gameplay_transport) fn room_contains(
    encounter: &oteryn_gameplay_quests::quest::world_callbacks::Encounter,
    at: MovementLocalPosition,
) -> bool {
    at.floor == -i16::from(encounter.room_min.floor)
        && (i32::from(encounter.room_min.x)..=i32::from(encounter.room_max.x)).contains(&at.x)
        && (i32::from(encounter.room_min.y)..=i32::from(encounter.room_max.y)).contains(&at.y)
}

pub(super) struct EncounterVisit {
    room: oteryn_gameplay_quests::quest::world_callbacks::Encounter,
    participants: Vec<(ExactActorRef, GameSessionId)>,
    creatures: Vec<ExactActorRef>,
    timeout: Option<PersonalTimeout>,
}

struct PersonalTimeout {
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    exit: MovementLocalPosition,
    due_us: u64,
}

impl EncounterVisit {
    fn active(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.creatures
            .iter()
            .any(|actor| runtime.contains_live_creature(*actor))
    }

    pub(super) fn next_due_us(&self) -> Option<u64> {
        self.timeout.as_ref().map(|timeout| timeout.due_us)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Crystal 201189f8, the_first_dragon/movements_entrance_teleport.lua:82-87.
    /// Retire only this opening's exact actor and roster on the owning Channel.
    pub(super) fn drain_personal_encounter_deadlines(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        now: u64,
    ) {
        let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
            return;
        };
        visits.retain_mut(|visit| {
            let Some(timeout) = visit.timeout.as_mut() else {
                return visit.active(runtime);
            };
            if timeout.scope != runtime.encounter_owner_scope() {
                return false;
            }
            if now < timeout.due_us {
                return true;
            }
            for &(actor, session) in &visit.participants {
                let Ok(expected) = runtime.read_actor_position(actor) else {
                    continue;
                };
                if !room_contains(&visit.room, expected.position()) || states.is_dead(actor) {
                    continue;
                }
                let closed = crate::map::floor::BundleDoors(&map.facts);
                let blocked =
                    super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
                let proof = crate::map::floor::prepare_quest_callback_teleport(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    timeout.exit,
                    blocked,
                    &closed,
                );
                let Some(snapshot) = proof
                    .ok()
                    .and_then(|proof| runtime.commit_source_step(proof).ok())
                else {
                    timeout.due_us = now.saturating_add(250_000);
                    return true;
                };
                states.stage_special_movement_step(
                    runtime,
                    map,
                    actor,
                    session,
                    now,
                    Some(expected.position()),
                    snapshot.position(),
                    None,
                );
                states.stage_underwater_step(
                    runtime,
                    actor,
                    session,
                    now,
                    underwater(map, expected.position()),
                    underwater(map, snapshot.position()),
                );
                if let Some(pos) = native(snapshot.position()) {
                    map.facts.emit_movement_effect(pos, 11);
                }
            }
            visit.creatures.retain(|actor| {
                runtime.contains_live_creature(*actor)
                    && runtime.remove_encounter_actor(*actor).is_err()
            });
            if visit.creatures.is_empty() {
                return false;
            }
            timeout.due_us = now.saturating_add(250_000);
            true
        });
    }

    pub(in crate::gameplay_transport) fn encounter_blocks_channel_switch(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        position: MovementLocalPosition,
    ) -> bool {
        if self.states.grizzly_boss.has_pending(session) {
            return true;
        }
        let callbacks = &self.states.quest_callbacks;
        if callbacks.encounters.lock().map_or(true, |visits| {
            visits.iter().any(|visit| {
                visit.participants.contains(&(actor, session))
                    && room_contains(&visit.room, position)
                    && visit.active(runtime)
            })
        }) {
            return true;
        }
        self.bundle_map().is_some_and(|map| {
            let now = self.owner_now().get();
            callbacks.timed.lock().map_or(true, |visits| {
                visits.iter().any(|visit| {
                    visit.contains_participant(runtime, map, actor, session, position, now)
                })
            })
        })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one owner turn binds party, native carrier, map and exact lever revision"
    )]
    pub(super) fn execute_quest_encounter(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        expected_revision: u64,
        after: EntryFacts,
        preflight: bool,
        cause: QuestCause,
        reservation: Option<&mut personal_encounter::Reservation<'_>>,
        step_origin: Option<MovementLocalPosition>,
        projection: Option<&QuestCallbackProjection>,
    ) -> UseDisposition {
        self.quest_encounter(
            runtime,
            states,
            actor,
            session,
            map,
            callback,
            key,
            expected_revision,
            after,
            preflight,
            cause,
            reservation,
            step_origin,
            projection,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn preflight_personal_quest_encounter(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        expected_revision: u64,
        after: EntryFacts,
        cause: QuestCause,
        step_origin: Option<MovementLocalPosition>,
    ) -> UseDisposition {
        self.quest_encounter(
            runtime,
            states,
            actor,
            session,
            map,
            callback,
            key,
            expected_revision,
            after,
            true,
            cause,
            None,
            step_origin,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn quest_encounter(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        expected_revision: u64,
        after: EntryFacts,
        preflight: bool,
        cause: QuestCause,
        reservation: Option<&mut personal_encounter::Reservation<'_>>,
        step_origin: Option<MovementLocalPosition>,
        projection: Option<&QuestCallbackProjection>,
    ) -> UseDisposition {
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        let timeout = if let Some(ms) = encounter.personal_timeout_ms {
            let Some(exit) = step_origin else {
                return UseDisposition::Rejected;
            };
            let Some(due_us) = self.owner_now().get().checked_add(u64::from(ms) * 1_000) else {
                return UseDisposition::Exhausted;
            };
            Some(PersonalTimeout {
                scope: runtime.encounter_owner_scope(),
                exit,
                due_us,
            })
        } else {
            None
        };
        if preflight && callback.has_personal_state() {
            if !callback.requires_world_admission {
                return UseDisposition::Rejected;
            }
            if encounter.timing.is_some() {
                // A timed personal trial schedules only after its durable receipt.
                // Preflight checks the same guards without reserving a timer.
                let ready = oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(
                    callback,
                ) && encounter.players.is_empty()
                    && states.get(runtime, actor, session).is_some_and(|state| {
                        state.character_facts().level >= encounter.minimum_level
                    })
                    && self.personal_trial_ready(runtime, callback, key);
                return if ready {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                };
            }
        }
        if callback.has_personal_state() && !preflight && reservation.is_none() {
            return UseDisposition::Rejected;
        }
        if encounter.timing.is_some() {
            return self.schedule_timed_quest_encounter(
                runtime,
                states,
                actor,
                session,
                map,
                callback,
                key,
                expected_revision,
                after,
            );
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        let personal_entry = callback.personal_room_entry().map(local);
        if census.iter().any(|(who, pos, occupant_session)| {
            (occupant_session.is_some() || encounter.personal_cooldown_seconds.is_some())
                && room_contains(encounter, pos.position())
                && !(Some(pos.position()) == personal_entry
                    && *who == actor
                    && *occupant_session == Some(session))
        }) {
            return UseDisposition::Occupied;
        }
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let mut players = Vec::new();
        let mut proofs = Vec::new();
        for member in &encounter.players {
            let mut occupants = census
                .iter()
                .filter(|(_, pos, _)| pos.position() == local(member.from));
            let Some(&(who, expected, Some(member_session))) = occupants.next() else {
                // Cults collects whoever stands on the five source staging tiles.
                if encounter.shared_staging
                    || encounter.phase
                        == Some(
                            oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase::Cults,
                        )
                {
                    continue;
                }
                return UseDisposition::Rejected;
            };
            if occupants.next().is_some()
                || states.is_dead(who)
                || !states.spell_idle(runtime, who, member_session)
                || states
                    .get(runtime, who, member_session)
                    .is_none_or(|state| state.character_facts().level < encounter.minimum_level)
                || runtime.bound_attacker_lease(who, member_session).is_err()
                || !self.states.player_trades.physical(member_session, None)
                || (self.has_pending_quest_callback(member_session)
                    && !(who == actor
                        && member_session == session
                        && !self.states.grizzly_boss.has_pending(session)
                        && self
                            .states
                            .quest_callbacks
                            .soul_rewards
                            .lock()
                            .is_ok_and(|pending| !pending.contains_key(&session))
                        && self
                            .states
                            .quest_callbacks
                            .records
                            .lock()
                            .is_ok_and(|pending| {
                                pending.get(&session).is_some_and(|record| {
                                    pending_encounter_matches(
                                        record,
                                        actor,
                                        member_session,
                                        expected,
                                        &callback.key,
                                        key,
                                        expected_revision,
                                        cause,
                                    )
                                })
                            })))
            {
                return UseDisposition::Rejected;
            }
            let blocked =
                super::super::pvp_live::pz_blocked(states, who, member_session, self.owner_now());
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_landing(
                runtime,
                &self.movement_cells,
                who,
                member_session,
                expected,
                local(member.to),
                blocked,
                &closed,
                encounter.shared_staging,
            ) else {
                return UseDisposition::NotPossible;
            };
            players.push((who, member_session, expected));
            proofs.push(proof);
        }
        if !players
            .iter()
            .any(|(who, s, _)| *who == actor && *s == session)
        {
            return UseDisposition::Rejected;
        }
        let mut monsters = Vec::new();
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
                || census.iter().any(|(_, pos, _)| pos.position() == at)
            {
                return UseDisposition::Occupied;
            }
            let Ok(policy) = runtime.companion_policy(&monster.creature) else {
                return UseDisposition::Rejected;
            };
            monsters.push((monster.creature.as_str(), policy.maximum_health, at));
        }
        if preflight {
            return if runtime
                .preflight_quest_encounter_entry(proofs, &monsters)
                .is_ok()
            {
                UseDisposition::Committed
            } else {
                UseDisposition::Occupied
            };
        }
        let party: Vec<_> = players
            .iter()
            .map(|(actor, session, _)| (*actor, *session))
            .collect();
        let scoped_hash = self.quest_catalogue.as_deref().and_then(|catalogue| {
            let bindings = catalogue.authored_bindings()?;
            bindings.quests.iter().find(|quest| quest.stages.iter().any(|stage|
                matches!(&stage.trigger, oteryn_gameplay_quests::quest::authored_bindings::Trigger::MapUse { callback: source }
                    if source.as_ref() == callback))
                && quest.stages.iter().any(|stage|
                    matches!(&stage.trigger, oteryn_gameplay_quests::quest::authored_bindings::Trigger::Kill { counter }
                        if counter.encounter_source.as_deref() == Some(callback.key.as_str()))))
                .and_then(|quest| catalogue.definition_hash(&quest.quest))
        });
        let attack = if scoped_hash.is_some() {
            match self.states.attack.try_lock() {
                Ok(attack) => Some(attack),
                Err(_) => return UseDisposition::Rejected,
            }
        } else {
            None
        };
        let mut capture = if let Some(attack) = attack.as_ref() {
            match attack.kills().1.lock() {
                Ok(capture) => Some(capture),
                Err(_) => return UseDisposition::Rejected,
            }
        } else {
            None
        };
        let prepared = if let (Some(capture), Some(hash)) = (capture.as_mut(), scoped_hash) {
            match capture.prepare_encounter(runtime, &callback.key, hash, &party) {
                Ok(prepared) => Some(prepared),
                Err(_) => return UseDisposition::Rejected,
            }
        } else {
            None
        };
        let credits = if encounter.phase == Some(world_callbacks::EncounterPhase::Cults) {
            let Ok(credits) =
                super::super::grizzly_kills::capture_encounter_admission(self, runtime, &party)
            else {
                return UseDisposition::Rejected;
            };
            Some(credits)
        } else {
            None
        };
        let mut phases = if encounter.has_phases() {
            let Ok(active) = states.encounter_phases.reserve(runtime, key) else {
                return UseDisposition::Occupied;
            };
            Some(active)
        } else {
            None
        };
        let mut visits = if !encounter.has_phases() && !monsters.is_empty() {
            let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
                return UseDisposition::Rejected;
            };
            visits.retain(|visit| {
                visit.active(runtime)
                    || visit
                        .timeout
                        .as_ref()
                        .is_some_and(|timeout| timeout.scope == runtime.encounter_owner_scope())
            });
            if visits.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS
                || visits.try_reserve(1).is_err()
            {
                return UseDisposition::Exhausted;
            }
            Some(visits)
        } else {
            None
        };
        let Ok((snapshots, (phase, actors, roster))) = runtime.commit_quest_encounter_with_phase(
            proofs,
            &monsters,
            |runtime, actors| {
                let roster = if prepared.is_some() {
                    Some(oteryn_gameplay_entities::carrier::encounter_access::EncounterDefenders::from_admission(
                        party.iter().map(|(actor, _)| *actor).collect(), actors.to_vec(),
                    )?)
                } else { None };
                encounter.has_phases().then(|| {
                        super::super::encounter_phases::bind(
                            runtime,
                            callback,
                            key,
                            actors,
                            &party,
                            credits.unwrap_or_default(),
                            self.movement_cells().clone(),
                        )
                    })
                    .transpose().map(|phase| (phase, actors.to_vec(), roster))
            },
            || {
                map.facts
                    .apply_quest_callback(key, expected_revision, after, &[], &[], false, projection)
            },
        ) else {
            return UseDisposition::Rejected;
        };
        if let (Some(capture), Some(prepared), Some(roster)) = (capture.as_mut(), prepared, roster)
        {
            capture.publish_encounter(prepared, roster);
        }
        if let (Some(phases), Some(phase)) = (&mut phases, phase) {
            phases.push(phase);
        }
        drop(phases);
        if let Some(visits) = &mut visits {
            visits.push(EncounterVisit {
                room: encounter.clone(),
                participants: party,
                creatures: actors.clone(),
                timeout,
            });
        }
        drop(visits);
        if let Some(reservation) = reservation
            && let Err(disposition) = reservation.publish(runtime, actors)
        {
            return disposition;
        }
        for ((who, member_session, expected), snapshot) in players.into_iter().zip(snapshots) {
            states.stage_special_movement_step(
                runtime,
                map,
                who,
                member_session,
                self.owner_now().get(),
                Some(expected.position()),
                snapshot.position(),
                None,
            );
            states.stage_underwater_step(
                runtime,
                who,
                member_session,
                self.owner_now().get(),
                underwater(map, expected.position()),
                underwater(map, snapshot.position()),
            );
            if let Some(pos) = native(snapshot.position()) {
                map.facts.emit_movement_effect(pos, 11);
            }
        }
        if encounter.personal_timeout_ms.is_some() {
            states.owner_wake.notify_one();
        }
        UseDisposition::Committed
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "compare the original actor, movement, object and command fences"
)]
fn pending_encounter_matches(
    pending: &Pending,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    callback: &str,
    key: u64,
    revision: u64,
    cause: QuestCause,
) -> bool {
    pending.actor == actor
        && pending.session == session
        && pending.expected == expected
        && pending.callback == callback
        && pending.key == key
        && pending.revision == revision
        && pending.cause == cause
}

#[cfg(test)]
#[path = "encounter_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "elemental_tests.rs"]
mod elemental_tests;
