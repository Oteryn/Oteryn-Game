//! Source parties are the players standing on the exact staging tiles, not a social-party id.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::map::view::EntryFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use sha2::{Digest, Sha256};

mod shared_room;

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
    expires: Option<VisitExpiry>,
    arrival_due: Option<u64>,
    join_until: Option<u64>,
    threshold: Option<super::super::encounter_phases::threshold::Threshold>,
    forms: Option<super::super::encounter_phases::position_forms::Forms>,
    owner_scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
}

struct VisitExpiry {
    due: u64,
    exit: world_callbacks::Position,
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    bundle_digest: [u8; 32],
}

impl EncounterVisit {
    pub(super) fn next_due_us(&self) -> Option<u64> {
        self.arrival_due
            .into_iter()
            .chain(self.expires.as_ref().map(|expiry| expiry.due))
            .chain(self.forms.as_ref().and_then(|forms| forms.next_due_us()))
            .min()
    }

    fn active(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.owner_scope == runtime.encounter_owner_scope()
            && (self.arrival_due.is_some()
                || self
                    .expires
                    .as_ref()
                    .is_some_and(|expiry| expiry.scope == runtime.encounter_owner_scope())
                || self
                    .creatures
                    .iter()
                    .any(|actor| runtime.contains_live_creature(*actor)))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn observe_quest_encounter_forms(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        now: u64,
    ) {
        let Some(map) = self.bundle_map() else {
            return;
        };
        let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
            return;
        };
        let scope = runtime.encounter_owner_scope();
        for visit in visits.iter_mut().filter(|visit| {
            visit.owner_scope == scope
                && visit.expires.as_ref().is_some_and(|expiry| {
                    expiry.due > now && expiry.bundle_digest == map.overlay.base().digest()
                })
        }) {
            if let (Some(forms), Some(boss)) = (&mut visit.forms, visit.creatures.first_mut())
                && forms
                    .observe(runtime, boss, now)
                    .is_ok_and(|scheduled| scheduled)
            {
                states.owner_wake.notify_one();
            }
        }
    }

    pub(in crate::gameplay_transport) fn tick_quest_encounter_thresholds(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) {
        let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
            return;
        };
        let now = self.owner_now().get();
        let scope = runtime.encounter_owner_scope();
        for visit in visits.iter_mut().filter(|visit| {
            visit.owner_scope == scope
                && visit
                    .expires
                    .as_ref()
                    .is_some_and(|expiry| expiry.due > now)
        }) {
            if let Some(phase) = &mut visit.threshold
                && let Ok(actors) = phase.tick(runtime)
            {
                visit.creatures.extend(actors);
            }
        }
    }

    pub(super) fn release_quest_encounter(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        source: &str,
        at: MovementLocalPosition,
    ) -> UseDisposition {
        let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
            return UseDisposition::Rejected;
        };
        let scope = runtime.encounter_owner_scope();
        let now = self.owner_now().get();
        if visits.iter_mut().any(|visit| {
            visit.owner_scope == scope
                && visit
                    .expires
                    .as_ref()
                    .is_some_and(|expiry| expiry.due > now)
                && visit.participants.contains(&(actor, session))
                && room_contains(&visit.room, at)
                && visit
                    .threshold
                    .as_mut()
                    .is_some_and(|phase| phase.release(runtime, source).is_ok())
        }) {
            UseDisposition::Committed
        } else {
            UseDisposition::NothingToUse
        }
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

    pub(super) fn expire_quest_encounter_visits(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
    ) {
        let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
            return;
        };
        let now = self.owner_now().get();
        let scope = runtime.encounter_owner_scope();
        let digest = map.overlay.base().digest();
        visits.retain_mut(|visit| {
            if visit.owner_scope != scope {
                return false;
            }
            if visit
                .expires
                .as_ref()
                .is_some_and(|expiry| expiry.scope != scope || expiry.bundle_digest != digest)
            {
                return false;
            }
            if visit.arrival_due.is_some_and(|due| due <= now) {
                visit.arrival_due = None;
                if let Some(link) = visit.room.linked_arrival.as_ref()
                    && let Some(original) = visit.creatures.first().copied()
                    && let Some(delayed) =
                        self.spawn_linked_arrival(runtime, states, map, original, link)
                {
                    visit.creatures.push(delayed);
                }
            }
            let Some(expiry) = &visit.expires else {
                return visit.active(runtime);
            };
            let room_players;
            let participants = if visit
                .room
                .solo_expiry
                .is_some_and(|expiry| expiry.shared.is_some() || expiry.eject_room_players)
            {
                // Crystal actions_portal_brain_head.lua:93 and
                // actions_portal_minis_feaster.lua:102-107 eject the live room census.
                let Ok(census) = runtime.positioned_melee_census() else {
                    return true;
                };
                room_players = census
                    .into_iter()
                    .filter_map(|(actor, at, session)| {
                        session
                            .filter(|&session| {
                                room_contains(&visit.room, at.position())
                                    && runtime.player_control_facts(actor, session).is_ok()
                            })
                            .map(|session| (actor, session))
                    })
                    .collect::<Vec<_>>();
                &room_players
            } else {
                &visit.participants
            };
            if expiry.due > now {
                let occupied = participants.iter().any(|&(actor, session)| {
                    runtime.player_control_facts(actor, session).is_ok()
                        && runtime
                            .read_actor_position(actor)
                            .is_ok_and(|snapshot| room_contains(&visit.room, snapshot.position()))
                });
                if !occupied {
                    super::personal_trial::cleanup(runtime, &mut visit.creatures);
                } else if let (Some(forms), Some(boss)) =
                    (&mut visit.forms, visit.creatures.first_mut())
                {
                    let _ = forms.publish_due(runtime, boss, now);
                }
                return occupied;
            }
            // Recheck the exact incarnation and current position before ejection.
            for &(actor, session) in participants {
                let Ok(expected) = runtime.read_actor_position(actor) else {
                    continue;
                };
                if runtime.player_control_facts(actor, session).is_err()
                    || !room_contains(&visit.room, expected.position())
                {
                    continue;
                }
                let closed = crate::map::floor::BundleDoors(&map.facts);
                let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(expiry.exit),
                    &closed,
                ) else {
                    return true;
                };
                let Ok(landed) = runtime.commit_source_step(proof) else {
                    return true;
                };
                states.stage_special_movement_step(
                    runtime,
                    map,
                    actor,
                    session,
                    now,
                    Some(expected.position()),
                    landed.position(),
                    None,
                );
                states.stage_underwater_step(
                    runtime,
                    actor,
                    session,
                    now,
                    underwater(map, expected.position()),
                    underwater(map, landed.position()),
                );
                map.facts.emit_movement_effect(tile(expiry.exit), 11);
            }
            super::personal_trial::cleanup(runtime, &mut visit.creatures);
            false
        });
    }

    /// Crystal actions_portal_minis_feaster.lua:74-95 clears the previous
    /// encounter after checking for players. Retire only this owner's old visit.
    fn clear_vacant_solo_visit(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        room: &world_callbacks::Encounter,
    ) -> Result<(), UseDisposition> {
        let scope = runtime.encounter_owner_scope();
        let digest = map.overlay.base().digest();
        let mut visits = self
            .states
            .quest_callbacks
            .encounters
            .lock()
            .map_err(|_| UseDisposition::Rejected)?;
        let mut index = 0;
        while index < visits.len() {
            let visit = &visits[index];
            if visit.owner_scope != scope
                || &visit.room != room
                || visit
                    .expires
                    .as_ref()
                    .is_none_or(|expiry| expiry.bundle_digest != digest)
            {
                index += 1;
                continue;
            }
            for actor in visit.creatures.iter().copied() {
                if runtime.contains_live_creature(actor) {
                    runtime
                        .remove_encounter_actor(actor)
                        .map_err(|_| UseDisposition::Occupied)?;
                }
            }
            visits.swap_remove(index);
        }
        Ok(())
    }

    fn spawn_linked_arrival(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        original: ExactActorRef,
        link: &world_callbacks::LinkedArrival,
    ) -> Option<ExactActorRef> {
        let center = local(link.monster.position);
        let census = runtime.positioned_melee_census().ok()?;
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let can_spawn = |at: MovementLocalPosition| {
            native(at).is_some_and(|tile| {
                !map.facts
                    .quest_callback_blocked(map.overlay.base(), tile, false)
                    && crate::movement::relocate::creature_cell_admissible(
                        self.movement_cells(),
                        &doors,
                        at,
                    )
                    && !census
                        .iter()
                        .any(|(_, position, _)| position.position() == at)
            })
        };
        let at = if can_spawn(center) {
            center
        } else {
            // Crystal actions_the_unwelcome_lever.lua:17-23 uses ordinary
            // Game.createMonster. Map::placeCreature (src/map/map.cpp:254-305)
            // considers the eight adjacent cells when its centre is unavailable.
            let adjacent = [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ];
            let candidates: Vec<_> = adjacent
                .into_iter()
                .map(|(dx, dy)| MovementLocalPosition {
                    x: center.x + dx,
                    y: center.y + dy,
                    ..center
                })
                .filter(|&at| can_spawn(at))
                .collect();
            if candidates.is_empty() {
                return None;
            }
            let digest = Sha256::new()
                .chain_update(b"oteryn:linked-arrival-placement:v1")
                .chain_update(original.placement_identity())
                .finalize();
            let mut sample = [0; 8];
            sample.copy_from_slice(&digest[..8]);
            candidates[(u64::from_be_bytes(sample) % candidates.len() as u64) as usize]
        };
        let health = runtime
            .companion_policy(&link.monster.creature)
            .ok()?
            .maximum_health;
        let delayed = *runtime
            .commit_quest_creature_spawn(&[(link.monster.creature.as_str(), health, at)])
            .ok()?
            .first()?;
        if runtime.contains_live_creature(original)
            && states
                .encounter_phases
                .link(
                    runtime,
                    delayed,
                    original,
                    link.radius,
                    link.death_effect,
                    self.movement_cells().clone(),
                )
                .is_err()
        {
            let _ = runtime.remove_encounter_actor(delayed);
            return None;
        }
        map.facts
            .emit_movement_effect(tile(link.monster.position), link.arrival_effect);
        Some(delayed)
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
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_none_or(|native| native.install_companion_policies(runtime).is_err())
        {
            return UseDisposition::Rejected;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        if let Some(expiry) = encounter.solo_expiry
            && expiry.shared.is_some()
        {
            return self.shared_quest_room(
                runtime,
                states,
                actor,
                session,
                map,
                callback,
                encounter,
                expiry,
                key,
                expected_revision,
                after,
                preflight,
                &census,
            );
        }
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
        let census = if preflight && encounter.solo_expiry.is_some() {
            if let Err(disposition) = self.clear_vacant_solo_visit(runtime, map, encounter) {
                return disposition;
            }
            let Ok(current) = runtime.positioned_melee_census() else {
                return UseDisposition::Rejected;
            };
            current
        } else {
            census
        };
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let mut players = Vec::new();
        let mut proofs = Vec::new();
        for member in &encounter.players {
            let mut occupants = census
                .iter()
                .filter(|(_, pos, _)| pos.position() == local(member.from));
            let Some(&(who, expected, Some(member_session))) = occupants.next() else {
                // Cults collects whoever stands on the five source staging tiles.
                if encounter.phase
                    == Some(oteryn_gameplay_quests::quest::world_callbacks::EncounterPhase::Cults)
                    || encounter.linked_arrival.is_some()
                    || callback.effects.threshold_rearm.is_some()
                    || callback.effects.position_forms.is_some()
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
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_entry(
                runtime,
                &self.movement_cells,
                who,
                member_session,
                expected,
                local(member.to),
                blocked,
                &closed,
                encounter.linked_arrival.is_some() || callback.effects.position_forms.is_some(),
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
        let visit_lifetime = encounter
            .solo_expiry
            .map(|expiry| (expiry.exit, expiry.time_limit_ms))
            .or_else(|| {
                encounter
                    .linked_arrival
                    .as_ref()
                    .map(|link| (link.exit, link.time_limit_ms))
            })
            .or_else(|| {
                callback
                    .effects
                    .threshold_rearm
                    .as_ref()
                    .map(|phase| (phase.exit, phase.time_limit_ms))
            })
            .or_else(|| {
                callback
                    .effects
                    .position_forms
                    .as_ref()
                    .map(|forms| (forms.exit, forms.time_limit_ms))
            });
        if callback
            .effects
            .threshold_rearm
            .as_ref()
            .is_some_and(|phase| {
                phase
                    .summons
                    .iter()
                    .any(|key| runtime.companion_policy(key).is_err())
            })
            || callback
                .effects
                .position_forms
                .as_ref()
                .is_some_and(|forms| {
                    forms
                        .regions
                        .iter()
                        .any(|region| runtime.companion_policy(&region.creature).is_err())
                })
        {
            return UseDisposition::Rejected;
        }
        let expiration = if let Some((exit, time_limit_ms)) = visit_lifetime {
            let Some(due) = self
                .owner_now()
                .get()
                .checked_add(u64::from(time_limit_ms) * 1_000)
            else {
                return UseDisposition::Exhausted;
            };
            let Some(base) = map.overlay.base().tile(exit.x, exit.y, exit.floor) else {
                return UseDisposition::Rejected;
            };
            if !base.walkable() {
                return UseDisposition::Rejected;
            }
            Some(VisitExpiry {
                due,
                exit,
                scope: runtime.encounter_owner_scope(),
                bundle_digest: map.overlay.base().digest(),
            })
        } else {
            None
        };
        let arrival_due = match &encounter.linked_arrival {
            Some(link) => {
                if runtime.companion_policy(&link.monster.creature).is_err() {
                    return UseDisposition::Rejected;
                }
                let Some(due) = self
                    .owner_now()
                    .get()
                    .checked_add(u64::from(link.after_ms) * 1_000)
                else {
                    return UseDisposition::Exhausted;
                };
                Some(due)
            }
            None => None,
        };
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
        if let Some(link) = &encounter.linked_arrival {
            for monster in &encounter.monsters {
                map.facts
                    .emit_movement_effect(tile(monster.position), link.arrival_effect);
            }
        }
        if let Some(visits) = &mut visits {
            visits.push(EncounterVisit {
                room: encounter.clone(),
                participants: party,
                creatures: actors.clone(),
                expires: expiration,
                arrival_due,
                join_until: None,
                forms: callback.effects.position_forms.as_ref().map(|spec| {
                    super::super::encounter_phases::position_forms::Forms::new(
                        encounter.monsters[0].creature.clone(),
                        spec.clone(),
                    )
                }),
                threshold: callback
                    .effects
                    .threshold_rearm
                    .as_ref()
                    .zip(actors.first().copied())
                    .map(|(spec, boss)| {
                        super::super::encounter_phases::threshold::Threshold::bind(
                            &callback.key,
                            spec,
                            boss,
                        )
                    }),
                owner_scope: runtime.encounter_owner_scope(),
            });
        }
        if encounter.solo_expiry.is_some()
            || arrival_due.is_some()
            || callback.effects.threshold_rearm.is_some()
            || callback.effects.position_forms.is_some()
        {
            states.owner_wake.notify_one();
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
