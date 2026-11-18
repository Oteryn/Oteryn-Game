//! Source parties are the players standing on the exact staging tiles, not a social-party id.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::map::view::EntryFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

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
    ) -> UseDisposition {
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        let room_exit = callback.timed_room_exit();
        if preflight && callback.has_personal_state() {
            if !callback.requires_world_admission {
                return UseDisposition::Rejected;
            }
            if encounter.timing.is_some() && room_exit.is_none() {
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
        if encounter.timing.is_some() && room_exit.is_none() {
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
        let mut room_timer = if let Some(exit) = room_exit {
            match self.prepare_quest_room_timer(
                runtime,
                map,
                callback,
                key,
                expected_revision,
                actor,
                session,
                exit,
            ) {
                Ok(timer) => Some(timer),
                Err(disposition) => return disposition,
            }
        } else {
            None
        };
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        let timed_room_entry = room_exit
            .and_then(|_| crate::map::view::placement_of(key))
            .map(|(position, _)| {
                local(Position {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                })
            });
        let personal_entry = timed_room_entry.or_else(|| callback.personal_room_entry().map(local));
        if census.iter().any(|(who, pos, occupant_session)| {
            occupant_session.is_some()
                && native(pos.position()).is_some_and(|position| {
                    encounter.contains(position.x, position.y, position.floor)
                })
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
            let from = timed_room_entry.unwrap_or_else(|| local(member.from));
            let mut occupants = census.iter().filter(|(_, pos, _)| pos.position() == from);
            let Some(&(who, expected, Some(member_session))) = occupants.next() else {
                // Cults collects whoever stands on the five source staging tiles.
                if encounter.phase
                    == Some(oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase::Cults)
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
            if let Some(exit) = room_exit
                && crate::map::floor::prepare_quest_encounter_ejection(
                    runtime,
                    self.movement_cells(),
                    who,
                    member_session,
                    expected,
                    local(exit),
                    &closed,
                )
                .is_err()
            {
                return UseDisposition::NotPossible;
            }
            let blocked =
                super::super::pvp_live::pz_blocked(states, who, member_session, self.owner_now());
            let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
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
        if room_timer.is_some()
            && self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .is_none_or(|native| native.install_companion_policies(runtime).is_err())
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
        let Ok((snapshots, (phase, actors, roster))) = runtime.commit_quest_encounter_with_phase(
            proofs,
            &monsters,
            |runtime, actors| {
                let roster = if prepared.is_some() {
                    Some(oteryn_gameplay_entities::carrier::encounter_access::EncounterDefenders::from_admission(
                        party.iter().map(|(actor, _)| *actor).collect(), actors.to_vec(),
                    )?)
                } else { None };
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
                    .transpose().map(|phase| (phase, actors.to_vec(), roster))
            },
            || {
                map.facts
                    .apply_quest_callback(key, expected_revision, after, &[], &[], false, None)
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
        if let Some(timer) = room_timer.take() {
            timer.publish(actors.clone());
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
