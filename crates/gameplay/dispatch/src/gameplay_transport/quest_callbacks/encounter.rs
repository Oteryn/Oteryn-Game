//! Source parties are the players standing on the exact staging tiles, not a social-party id.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::map::{facts::QuestCallbackProjection, view::EntryFacts};
use oteryn_gameplay_entities::carrier::encounter_access::{
    DetachedQuestEncounter, EncounterDefenders,
};

struct PreparedPhase {
    phase: Option<super::super::encounter_phases::ActivePhase>,
    actors: Vec<ExactActorRef>,
    roster: Option<EncounterDefenders>,
}

struct EncounterCreditPublication {
    cache: std::sync::Arc<std::sync::Mutex<super::super::grizzly_kills::GrizzlyCaptureCache>>,
    prepared: super::super::grizzly_kills::PreparedEncounter,
}

pub(super) struct StagedSourceBoss {
    native: DetachedQuestEncounter<PreparedPhase>,
    players: Vec<(ExactActorRef, GameSessionId, MovementPositionSnapshot)>,
    capture: Option<EncounterCreditPublication>,
    visit: Option<EncounterVisit>,
}

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
}

impl EncounterVisit {
    fn active(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.creatures
            .iter()
            .any(|actor| runtime.contains_live_creature(*actor))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
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
            projection,
            None,
        )
    }

    pub(super) fn stage_source_boss(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        source: &super::court_warlock::SourceBossUse<'_>,
    ) -> Result<StagedSourceBoss, UseDisposition> {
        let mut staged = None;
        let disposition = self.quest_encounter(
            runtime,
            states,
            actor,
            session,
            source.map,
            source.callback,
            source.key,
            source.revision,
            source.after,
            false,
            source.cause,
            None,
            None,
            Some(&mut staged),
        );
        staged.ok_or(disposition)
    }

    pub(super) fn publish_source_boss(
        &self,
        staged: StagedSourceBoss,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        lever: crate::map::facts::SourceBossLeverProjection,
    ) -> UseDisposition {
        self.publish_staged_encounter(staged, runtime, states, map, lever, None)
    }

    fn publish_staged_encounter(
        &self,
        staged: StagedSourceBoss,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        lever: crate::map::facts::SourceBossLeverProjection,
        reservation: Option<&mut personal_encounter::Reservation<'_>>,
    ) -> UseDisposition {
        let Ok((snapshots, prepared)) = staged.native.publish(runtime) else {
            return UseDisposition::StaleState;
        };
        if let (Some(capture), Some(roster)) = (staged.capture, prepared.roster) {
            capture
                .cache
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .publish_encounter(capture.prepared, roster);
        }
        if let Some(phase) = prepared.phase {
            states
                .encounter_phases
                .active
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(phase);
        }
        if let Some(mut visit) = staged.visit {
            visit.creatures.clone_from(&prepared.actors);
            self.states
                .quest_callbacks
                .encounters
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(visit);
        }
        if let Some(reservation) = reservation {
            reservation.publish_staged(runtime, prepared.actors);
        }
        lever.publish();
        self.project_encounter_movements(runtime, states, map, staged.players, snapshots);
        UseDisposition::Committed
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the spawn retains the exact world and Character proofs"
    )]
    pub(super) async fn deliver_coffin(
        &self,
        mut runtime_guard: tokio::sync::MutexGuard<'_, ChannelRuntimeV1>,
        mut states_guard: tokio::sync::MutexGuard<'_, ChannelSpellStates>,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        expected: u64,
        after: EntryFacts,
        cause: QuestCause,
        permit: &SpellLanePermit,
        fence: Option<CurrentCharacterGameplayFence>,
        reservation: Option<&mut personal_encounter::Reservation<'_>>,
        kind: VampireCoffinKind,
    ) -> UseDisposition {
        let (runtime, states) = (&mut *runtime_guard, &mut *states_guard);
        let (Some(fence), QuestCause::Use(command), Some(reservation)) =
            (fence, cause, reservation)
        else {
            return UseDisposition::Rejected;
        };
        let content = runtime.content_pin().server_artifact_digest();
        let Some(coffin) = map.facts.reserve_source_boss_lever(key, expected, after) else {
            return UseDisposition::Rejected;
        };
        let mut staged = None;
        let disposition = self.quest_encounter(
            runtime,
            states,
            actor,
            session,
            map,
            callback,
            key,
            expected,
            after,
            false,
            cause,
            Some(reservation),
            None,
            Some(&mut staged),
        );
        let Some(staged) = staged else {
            return disposition;
        };
        let [count] = staged.native.prepared().actors.as_slice() else {
            return UseDisposition::Rejected;
        };
        let count = *count;
        drop(states_guard);
        drop(runtime_guard);
        // A contended observation rolls back the read-only attempt. Wait outside
        // the Character transaction, then retry for the same exact generation.
        let mut retired = None;
        let delivery = loop {
            let mut contended = None;
            let delivery = self
                .root
                .deliver_vampire_coffin(
                    self.character,
                    self.holder,
                    permit,
                    fence.item_fence(),
                    command,
                    content,
                    kind,
                    count,
                    |previous| {
                        if retired == Some(previous) {
                            return false;
                        }
                        match self.runtime.try_lock() {
                            Ok(runtime) => {
                                runtime.contains_live_creature(previous)
                                    || runtime.corpse_projection_pinned(previous)
                            }
                            Err(_) => {
                                contended = Some(previous);
                                true
                            }
                        }
                    },
                )
                .await;
            let Some(previous) = contended.filter(|_| matches!(delivery, Ok(false))) else {
                break delivery;
            };
            let runtime = self.runtime.lock().await;
            if runtime.contains_live_creature(previous)
                || runtime.corpse_projection_pinned(previous)
            {
                break Ok(false);
            }
            retired = Some(previous);
        };
        #[cfg(test)]
        let delivery = if matches!(delivery, Ok(true))
            && self
                .states
                .quest_callbacks
                .lose_coffin_delivery_ack
                .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            Err(crate::durability::DurabilityError::CommitOutcomeUnknown)
        } else {
            delivery
        };
        if matches!(delivery, Ok(true)) {
            let (mut runtime, mut states) = self.callback_guards().await;
            if !states.current_cooldown_transport(actor, session, fence.connection_generation.get())
            {
                return UseDisposition::StaleState;
            }
            self.publish_staged_encounter(
                staged,
                &mut runtime,
                &mut states,
                map,
                coffin,
                Some(reservation),
            )
        } else {
            // The payment is durable. A fresh use may recover it after an
            // abandoned delivery, including when its identity write committed.
            if matches!(
                delivery,
                Ok(false) | Err(crate::durability::DurabilityError::CommitOutcomeUnknown)
            ) && let Ok(mut pending) = self.states.quest_callbacks.records.lock()
            {
                pending.remove(&session);
            }
            UseDisposition::Rejected
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
            true,
            cause,
            None,
            projection,
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
        projection: Option<&QuestCallbackProjection>,
        staged_source_boss: Option<&mut Option<StagedSourceBoss>>,
    ) -> UseDisposition {
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        if staged_source_boss.is_some()
            && (encounter.timing.is_some()
                || (encounter.phase.is_none()
                    && VampireCoffinKind::for_callback(&callback.key).is_none()))
        {
            return UseDisposition::Rejected;
        }
        if preflight && callback.has_personal_state() {
            if !callback.requires_world_admission {
                return UseDisposition::Rejected;
            }
            if encounter.timing.is_some() {
                // A timed personal trial schedules only after its durable receipt.
                // Preflight checks the same guards without reserving a timer.
                let ready = oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(
                    callback,
                )
                    || oteryn_gameplay_quests::quest::world_callbacks::spike_mound::chances(
                        callback,
                    )
                    .is_some()
                    || (callback.starts_merchant_wave() && projection.is_some());
                let ready = ready
                    && encounter.players.is_empty()
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
                projection,
            );
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        let personal_entry = callback.personal_room_entry().map(local);
        if census.iter().any(|(who, pos, occupant_session)| {
            occupant_session.is_some()
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
                // Crystal party levers collect whoever stands on their staging tiles.
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
            // Court Warlock requires a clear centre before the party relocates.
            if encounter.phase
                == Some(
                    oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase::CourtWarlock,
                )
                && crate::map::floor::prepare_quest_callback_teleport(
                    runtime,
                    &self.movement_cells,
                    who,
                    member_session,
                    expected,
                    local(member.to),
                    blocked,
                    &closed,
                )
                .is_err()
            {
                return UseDisposition::NotPossible;
            }
            // Source phase parties can share an arrival tile while still
            // qualifying voluntary movement and protection-zone access.
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_member(
                runtime,
                &self.movement_cells,
                who,
                member_session,
                expected,
                local(member.to),
                blocked,
                &closed,
                encounter.phase.is_some(),
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
            let result = if encounter.phase.is_some() {
                runtime.preflight_quest_encounter_with_phase(proofs, &monsters)
            } else {
                runtime.preflight_quest_encounter_entry(proofs, &monsters)
            };
            return if result.is_ok() {
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
            use oteryn_gameplay_quests::quest::authored_bindings::Trigger;
            let bindings = catalogue.authored_bindings()?;
            bindings.quests.iter().find(|quest| quest.stages.iter().any(|stage|
                matches!(&stage.trigger, Trigger::MapUse { callback: source } | Trigger::StepIn { callback: source }
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
            let Ok(active) =
                states
                    .encounter_phases
                    .reserve(runtime, states, key, self.owner_now().get())
            else {
                return UseDisposition::Occupied;
            };
            Some(active)
        } else {
            None
        };
        let mut visits = if encounter.phase.is_none() && !monsters.is_empty() {
            let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
                return UseDisposition::Rejected;
            };
            visits.retain(|visit| visit.active(runtime));
            if visits.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS
                || visits.try_reserve(1).is_err()
            {
                return UseDisposition::Exhausted;
            }
            Some(visits)
        } else {
            None
        };
        let capture_required = prepared.is_some();
        let bind = |runtime: &mut ChannelRuntimeV1, actors: &[ExactActorRef]| {
            let roster = if capture_required {
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
                        encounter,
                        key,
                        actors,
                        &party,
                        credits,
                        self.movement_cells().clone(),
                        self.owner_now().get(),
                    )
                })
                .transpose()
                .map(|phase| PreparedPhase {
                    phase,
                    actors: actors.to_vec(),
                    roster,
                })
        };
        if let Some(staged) = staged_source_boss {
            let Ok(mut native) = runtime.stage_quest_encounter_with_phase(proofs, &monsters, bind)
            else {
                return UseDisposition::Rejected;
            };
            let capture = prepared.zip(attack.as_ref()).map(|(prepared, attack)| {
                EncounterCreditPublication {
                    cache: attack.kills().1.clone(),
                    prepared,
                }
            });
            if VampireCoffinKind::for_callback(&callback.key).is_some() {
                let [count] = native.prepared().actors.as_slice() else {
                    return UseDisposition::Rejected;
                };
                if native.retain_spawn_generation(*count).is_err() {
                    return UseDisposition::Rejected;
                }
            }
            *staged = Some(StagedSourceBoss {
                native: native.detach(),
                players,
                capture,
                visit: visits.as_ref().map(|_| EncounterVisit {
                    room: encounter.clone(),
                    participants: party,
                    creatures: Vec::new(),
                }),
            });
            return UseDisposition::Committed;
        }
        let Ok((
            snapshots,
            PreparedPhase {
                phase,
                actors,
                roster,
            },
        )) = runtime.commit_quest_encounter_with_phase(proofs, &monsters, bind, || {
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
        if let Some(visits) = &mut visits {
            visits.push(EncounterVisit {
                room: encounter.clone(),
                participants: party,
                creatures: actors.clone(),
            });
        }
        drop(visits);
        if let Some(reservation) = reservation
            && let Err(disposition) = reservation.publish(runtime, actors)
        {
            return disposition;
        }
        self.project_encounter_movements(runtime, states, map, players, snapshots);
        UseDisposition::Committed
    }

    fn project_encounter_movements(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        players: Vec<(ExactActorRef, GameSessionId, MovementPositionSnapshot)>,
        snapshots: Vec<MovementPositionSnapshot>,
    ) {
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
