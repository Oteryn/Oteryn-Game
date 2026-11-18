//! Source parties are the players standing on the exact staging tiles, not a social-party id.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::map::view::EntryFacts;

impl ComposedFreshAdmission<'_, '_, '_> {
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
        preview_party: Option<&mut Vec<(ExactActorRef, GameSessionId)>>,
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
            preview_party,
        )
    }

    pub(super) async fn admit_quest_boss_cooldown(
        &self,
        runtime: &ChannelRuntimeV1,
        party: Vec<(ExactActorRef, GameSessionId)>,
        boss: &'static str,
        cooldown_s: u64,
    ) -> Result<(), UseDisposition> {
        use crate::durability::activity_handoff::{ActivityHandoffOwner, ActivityRefusal};

        let mut roster = Vec::with_capacity(party.len());
        for (actor, session) in party {
            let Ok(Some(fence)) = self.current_quest_fence(session).await else {
                return Err(UseDisposition::Rejected);
            };
            let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                return Err(UseDisposition::Rejected);
            };
            if lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
                || (fence.runtime_scope, fence.scope_ownership_generation)
                    != runtime.encounter_owner_scope()
            {
                return Err(UseDisposition::Rejected);
            }
            roster.push(fence.item_fence());
        }
        match ActivityHandoffOwner::new(self.root.clone())
            .admit_channel_boss_cooldown(self.character, self.holder, roster, boss, cooldown_s)
            .await
        {
            Ok(Ok(())) => Ok(()),
            Ok(Err(ActivityRefusal::Cooldown)) => Err(UseDisposition::RequirementNotMet),
            _ => Err(UseDisposition::Rejected),
        }
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
        preview_party: Option<&mut Vec<(ExactActorRef, GameSessionId)>>,
    ) -> UseDisposition {
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
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
        let Ok(mut census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        let in_room = |at| super::timed_encounter::in_room(encounter, at);
        if encounter.phase
            == Some(oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase::Ghulosh)
            && !census
                .iter()
                .any(|(_, at, session)| session.is_some() && in_room(at.position()))
        {
            match states.encounter_phases.retire_library_room(runtime, key) {
                Ok(true) => {
                    let Ok(current) = runtime.positioned_melee_census() else {
                        return UseDisposition::Rejected;
                    };
                    census = current;
                }
                Ok(false) => {}
                Err(_) => return UseDisposition::Rejected,
            }
        }
        let mut timeout = None;
        if let Some(rule) = encounter.room_timeout {
            let Ok(mut timers) = self.states.quest_callbacks.timed.lock() else {
                return UseDisposition::Rejected;
            };
            // Crystal clears a prior scorpion when nobody is left in the room.
            // Retire only actors owned by the previous admission.
            if !census
                .iter()
                .any(|(_, at, s)| s.is_some() && in_room(at.position()))
                && let Some(index) = timers.iter().position(|memo| memo.source_key() == key)
            {
                timers.swap_remove(index).retire(runtime);
                let Ok(current) = runtime.positioned_melee_census() else {
                    return UseDisposition::Rejected;
                };
                census = current;
            }
            let (Some(due), Some(revision)) = (
                self.owner_now()
                    .get()
                    .checked_add(u64::from(rule.after_ms) * 1000),
                expected_revision.checked_add(1),
            ) else {
                return UseDisposition::Exhausted;
            };
            if timers.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS
                || timers.iter().any(|memo| memo.source_key() == key)
                || timers.try_reserve(1).is_err()
            {
                return UseDisposition::Occupied;
            }
            timeout = Some((timers, due, revision));
        }
        let personal_entry = callback.personal_room_entry().map(local);
        if census.iter().any(|(who, pos, occupant_session)| {
            occupant_session.is_some()
                && in_room(pos.position())
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
                if encounter.phase.is_some() {
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
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_admission(
                runtime,
                &self.movement_cells,
                who,
                member_session,
                expected,
                local(member.to),
                blocked,
                &closed,
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
        if preflight && preview_party.is_none() {
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
        let credits = if encounter.phase.is_some() {
            let Ok(credits) =
                super::super::grizzly_kills::capture_encounter_admission(self, runtime, &party)
            else {
                return UseDisposition::Rejected;
            };
            Some(credits)
        } else {
            None
        };
        let mut phases = if encounter.phase.is_some() {
            let Ok(active) = states.encounter_phases.reserve(runtime, key) else {
                return UseDisposition::Occupied;
            };
            Some(active)
        } else {
            None
        };
        let bind = |runtime: &mut ChannelRuntimeV1, actors: &[ExactActorRef]| {
            let roster = if prepared.is_some() {
                Some(oteryn_gameplay_entities::carrier::encounter_access::EncounterDefenders::from_admission(
                        party.iter().map(|(actor, _)| *actor).collect(), actors.to_vec(),
                    )?)
            } else {
                None
            };
            credits
                .map(|credits| {
                    super::super::encounter_phases::bind(
                        runtime,
                        callback,
                        key,
                        actors,
                        &party,
                        credits,
                        self.movement_cells().clone(),
                    )
                })
                .transpose()
                .map(|phase| (phase, actors.to_vec(), roster))
        };
        if let Some(preview) = preview_party {
            if !map.facts.quest_callback_capacity(key, expected_revision, 0)
                || runtime
                    .preflight_quest_encounter_with_phase(proofs, &monsters, bind)
                    .is_err()
            {
                return UseDisposition::Rejected;
            }
            preview.extend(party);
            return UseDisposition::Committed;
        }
        let Ok((snapshots, (phase, actors, roster))) =
            runtime.commit_quest_encounter_with_phase(proofs, &monsters, bind, || {
                map.facts
                    .apply_quest_callback(key, expected_revision, after, &[], &[], false, None)
            })
        else {
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
        if let Some((mut timers, due, revision)) = timeout {
            timers.push(super::timed_encounter::TimedEncounter::room(
                runtime,
                callback.clone(),
                map,
                key,
                revision,
                due,
                party,
                actors.clone(),
            ));
            states.owner_wake.notify_one();
        }
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
