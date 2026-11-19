//! Crystal actions_portal_brain_head.lua:50-126.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "one current owner turn checks and publishes the native STEP"
    )]
    pub(super) fn shared_quest_room(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        room: &world_callbacks::Encounter,
        expiry: world_callbacks::SoloEncounterExpiry,
        key: u64,
        revision: u64,
        after: EntryFacts,
        preflight: bool,
        census: &[(
            ExactActorRef,
            MovementPositionSnapshot,
            Option<GameSessionId>,
        )],
    ) -> UseDisposition {
        let (Some(admission), [member]) = (expiry.shared, room.players.as_slice()) else {
            return UseDisposition::Rejected;
        };
        let now = self.owner_now().get();
        let scope = runtime.encounter_owner_scope();
        let digest = map.overlay.base().digest();
        let Ok(mut visits) = self.states.quest_callbacks.encounters.lock() else {
            return UseDisposition::Rejected;
        };
        let existing = visits.iter().position(|visit| {
            visit.owner_scope == scope
                && visit.join_until.is_some()
                && visit.room.room_min == room.room_min
                && visit.room.room_max == room.room_max
                && visit
                    .expires
                    .as_ref()
                    .is_some_and(|stored| stored.bundle_digest == digest)
        });
        let occupants = census
            .iter()
            .filter(|(who, position, owner)| {
                *who != actor && owner.is_some() && room_contains(room, position.position())
            })
            .count();
        if occupants >= usize::from(admission.capacity)
            || (occupants > 0
                && existing.is_some_and(|index| {
                    visits[index]
                        .join_until
                        .is_none_or(|deadline| now >= deadline)
                }))
        {
            return UseDisposition::Occupied;
        }
        if states.is_dead(actor)
            || !states.spell_idle(runtime, actor, session)
            || states
                .get(runtime, actor, session)
                .is_none_or(|state| state.character_facts().level < room.minimum_level)
            || runtime.bound_attacker_lease(actor, session).is_err()
            || !self.states.player_trades.physical(session, None)
        {
            return UseDisposition::Rejected;
        }
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return UseDisposition::Rejected;
        };
        if native(expected.position()).is_none_or(|p| {
            !callback
                .selector
                .matches_position(world_callbacks::Position {
                    x: p.x,
                    y: p.y,
                    floor: p.floor,
                })
        }) {
            return UseDisposition::Rejected;
        }
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let blocked =
            super::super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
        let Ok(proof) = crate::map::floor::prepare_quest_encounter_entry(
            runtime,
            self.movement_cells(),
            actor,
            session,
            expected,
            local(member.to),
            blocked,
            &doors,
            true,
        ) else {
            return UseDisposition::NotPossible;
        };
        let deadline = |ms| now.checked_add(u64::from(ms) * 1_000);
        let (Some(join_until), Some(due)) = (
            deadline(admission.join_window_ms),
            deadline(expiry.time_limit_ms),
        ) else {
            return UseDisposition::Exhausted;
        };
        // Crystal actions_portal_brain_head.lua:56-69,120-130 resets for a
        // new admission. An ingress can reach the portal before the scheduled
        // vacancy census, but its cooldown check must leave that roster intact.
        let abandoned = existing.filter(|_| occupants == 0);
        let existing = existing.filter(|_| abandoned.is_none());
        let retired = abandoned.map_or_else(Vec::new, |index| visits[index].creatures.clone());
        let can_add_visit = abandoned.is_some()
            || visits.len() < super::super::timed_encounter::MAX_TIMED_ENCOUNTERS;
        let mut monsters = Vec::new();
        if existing.is_none() {
            if !map
                .overlay
                .base()
                .tile(expiry.exit.x, expiry.exit.y, expiry.exit.floor)
                .is_some_and(|cell| cell.walkable())
            {
                return UseDisposition::Rejected;
            }
            for monster in &room.monsters {
                let position = local(monster.position);
                if !crate::movement::relocate::creature_cell_admissible(
                    self.movement_cells(),
                    &doors,
                    position,
                ) || map.facts.quest_callback_blocked(
                    map.overlay.base(),
                    tile(monster.position),
                    false,
                ) || census.iter().any(|(actor, occupant, _)| {
                    occupant.position() == position && !retired.contains(actor)
                }) {
                    return UseDisposition::Occupied;
                }
                let Ok(policy) = runtime.companion_policy(&monster.creature) else {
                    return UseDisposition::Rejected;
                };
                monsters.push((monster.creature.as_str(), policy.maximum_health, position));
            }
        }
        if preflight {
            return if !monsters.is_empty()
                && (!can_add_visit
                    || runtime
                        .preflight_quest_encounter_replacing_visit(vec![proof], &monsters, &retired)
                        .is_err())
            {
                UseDisposition::Occupied
            } else {
                UseDisposition::Committed
            };
        }
        let mut new_visit = None;
        let visit = if let Some(index) = existing {
            let visit = &mut visits[index];
            // Retire disconnected and departed roster entries before admitting replacements.
            visit.participants.retain(|(who, owner)| {
                census.iter().any(|(current, position, session)| {
                    current == who
                        && *session == Some(*owner)
                        && room_contains(room, position.position())
                })
            });
            visit
        } else {
            if !can_add_visit || (abandoned.is_none() && visits.try_reserve(1).is_err()) {
                return UseDisposition::Exhausted;
            }
            new_visit.insert(EncounterVisit {
                room: room.clone(),
                participants: Vec::new(),
                creatures: Vec::new(),
                expires: Some(VisitExpiry {
                    due,
                    exit: expiry.exit,
                    scope,
                    bundle_digest: digest,
                }),
                arrival_due: None,
                join_until: Some(join_until),
                forms: None,
                threshold: None,
                owner_scope: scope,
            })
        };
        if visit.participants.try_reserve(1).is_err() {
            return UseDisposition::Exhausted;
        }
        let project = || {
            map.facts
                .apply_quest_callback(key, revision, after, &[], &[], false, None)
        };
        let landings = if monsters.is_empty() {
            runtime.commit_quest_relocation(vec![proof], project)
        } else {
            runtime
                .commit_quest_encounter_replacing_visit(vec![proof], &monsters, &retired, project)
                .map(|(landings, creatures)| {
                    visit.creatures = creatures;
                    landings
                })
        };
        let Ok(landings) = landings else {
            return UseDisposition::Rejected;
        };
        let [landed] = landings.as_slice() else {
            return UseDisposition::Rejected;
        };
        if !visit.participants.contains(&(actor, session)) {
            visit.participants.push((actor, session));
        }
        if let Some(visit) = new_visit {
            if let Some(old) = abandoned {
                visits.swap_remove(old);
            }
            visits.push(visit);
            states.owner_wake.notify_one();
            if let Some(message) = &callback.success_message {
                self.states.chat.push_quest_feedback(session, message);
            }
        }
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
        map.facts.emit_movement_effect(tile(member.to), 11);
        UseDisposition::Committed
    }
}
