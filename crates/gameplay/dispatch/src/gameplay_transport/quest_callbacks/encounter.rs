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
        let in_room = |at: MovementLocalPosition| {
            at.floor == -i16::from(encounter.room_min.floor)
                && (i32::from(encounter.room_min.x)..=i32::from(encounter.room_max.x))
                    .contains(&at.x)
                && (i32::from(encounter.room_min.y)..=i32::from(encounter.room_max.y))
                    .contains(&at.y)
        };
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
                // These source levers collect whoever stands on the staging tiles.
                if encounter.weekday_monsters.is_some() || encounter.phase.is_some() {
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
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_entry(
                runtime,
                &self.movement_cells,
                who,
                member_session,
                expected,
                local(member.to),
                blocked,
                &closed,
                encounter.phase.is_some() || encounter.weekday_monsters.is_some(),
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
        let roster = if encounter.weekday_monsters.is_some() {
            let startup = &self.states.quest_callbacks.encounter_startup;
            let Some(started_at) = startup.get().copied().or_else(|| {
                let now = super::super::unix_seconds()?;
                let started_at =
                    now.checked_sub(i64::try_from(self.clock_origin.elapsed().as_secs()).ok()?)?;
                Some(*startup.get_or_init(|| started_at))
            }) else {
                return UseDisposition::Rejected;
            };
            encounter.monsters_at_startup(started_at)
        } else {
            &encounter.monsters
        };
        for monster in roster {
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
        use oteryn_gameplay_quests::quest::authored_bindings::Trigger as BindingTrigger;
        let scoped = self.quest_catalogue.as_deref().and_then(|catalogue| {
            let bindings = catalogue.authored_bindings()?;
            let quest =
                bindings.quests.iter().find(|quest| {
                    quest.quest == callback.quest && quest.stages.iter().any(|stage| matches!(
                    &stage.trigger,
                    BindingTrigger::MapUse { callback: source } if source.as_ref() == callback
                ))
                })?;
            let counters: Vec<_> = quest
                .stages
                .iter()
                .filter_map(|stage| match &stage.trigger {
                    BindingTrigger::Kill { counter }
                        if counter.encounter_source.as_deref() == Some(callback.key.as_str()) =>
                    {
                        Some(counter.as_ref())
                    }
                    _ => None,
                })
                .collect();
            if counters.is_empty() {
                return None;
            }
            Some((catalogue.definition_hash(&quest.quest)?, counters))
        });
        let party: Vec<_> = players
            .iter()
            .map(|(actor, session, _)| (*actor, *session))
            .collect();
        if encounter.weekday_monsters.is_some()
            && let Some((_, counters)) = &scoped
            && let Err(disposition) =
                self.check_encounter_death_cooldowns(&party, &monsters, counters)
        {
            return disposition;
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
        let attack = if scoped.is_some() {
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
        if encounter.weekday_monsters.is_some()
            && let (Some(attack), Some(capture)) = (&attack, &capture)
            && capture.encounter_credit_pending(&callback.key, attack.kills())
        {
            return UseDisposition::Rejected;
        }
        let prepared = if let (Some(capture), Some(hash)) =
            (capture.as_mut(), scoped.as_ref().map(|(hash, _)| *hash))
        {
            let damagers_only = encounter.weekday_monsters.is_some()
                || encounter.phase == Some(world_callbacks::EncounterPhase::FacelessBane);
            match capture.prepare_encounter(runtime, &callback.key, hash, &party, damagers_only) {
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
                let roster = if let Some((_, counters)) = &scoped {
                    let defenders = actors.iter().copied().zip(&monsters)
                        .filter_map(|(actor, (creature, _, _))| {
                            (encounter.weekday_monsters.is_none()
                                || counters.iter().any(|counter| counter.creature == *creature))
                                .then_some(actor)
                        }).collect();
                    Some(oteryn_gameplay_entities::carrier::encounter_access::EncounterDefenders::from_admission(
                        party.iter().map(|(actor, _)| *actor).collect(), defenders,
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
    fn check_encounter_death_cooldowns(
        &self,
        party: &[(ExactActorRef, GameSessionId)],
        monsters: &[(&str, i64, MovementLocalPosition)],
        counters: &[&oteryn_gameplay_quests::quest::authored_bindings::kills::KillCounter],
    ) -> Result<(), UseDisposition> {
        let tracks: std::collections::BTreeSet<_> = counters
            .iter()
            .filter(|counter| {
                monsters
                    .iter()
                    .any(|(creature, _, _)| *creature == counter.creature)
            })
            .filter_map(|counter| counter.cooldown_track.as_deref())
            .collect();
        if tracks.is_empty() {
            return Ok(());
        }
        let now = super::super::unix_seconds().ok_or(UseDisposition::Rejected)?;
        let sessions = self
            .states
            .quest_sessions
            .lock()
            .map_err(|_| UseDisposition::Rejected)?;
        for (_, session) in party {
            let copy = sessions
                .get(session)
                .and_then(|state| state.copy.as_ref())
                .ok_or(UseDisposition::Rejected)?;
            if tracks.iter().any(|track| {
                copy.tracks()
                    .get(*track)
                    .is_some_and(|last| *last > 0 && now.saturating_sub(*last) < 20 * 60 * 60)
            }) {
                return Err(UseDisposition::NotPossible);
            }
        }
        Ok(())
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
