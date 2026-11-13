//! Rafzan's winches commit scenery and the complete departure census together.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use oteryn_gameplay_quests::quest::goblin_merchant as elevator;

pub(super) fn owns(key: &str) -> bool {
    matches!(key, elevator::UPPER_KEY | elevator::LOWER_KEY)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "one Channel owner turn binds the source winch, map, rider and movement effects"
    )]
    pub(super) fn execute_goblin_elevator(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        optional_pvp: Option<bool>,
    ) -> UseDisposition {
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return UseDisposition::Rejected;
        };
        let at = if callback.key == elevator::UPPER_KEY {
            elevator::UPPER
        } else {
            elevator::LOWER
        };
        let mut entries = Vec::new();
        if crate::map::view::compose_into(
            map.overlay.base(),
            None,
            &map.facts,
            tile(at),
            &mut entries,
        )
        .is_err()
        {
            return UseDisposition::Rejected;
        }
        let here = entries
            .iter()
            .any(|entry| entry.facts.appearance_id == 17938);
        let trip = match elevator::trip(&callback.key, here, expected.position() == local(at)) {
            Ok(trip) => trip,
            Err(message) => {
                self.states.chat.push_quest_feedback(session, message);
                return UseDisposition::Rejected;
            }
        };
        let transforms = trip
            .transforms
            .map(|t| (tile(t.position), t.before, t.after));
        let Some(batch) = map.facts.prepare_quest_platform_batch(
            map.overlay.base(),
            tile(trip.from),
            tile(trip.to),
            &trip.items,
            &transforms,
        ) else {
            return UseDisposition::Rejected;
        };
        if trip.riding {
            let blocked =
                super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
            let Ok(proof) = crate::map::floor::prepare_quest_platform_rider(
                runtime,
                &self.movement_cells,
                map,
                &batch,
                actor,
                session,
                expected,
                blocked,
            ) else {
                return UseDisposition::NotPossible;
            };
            let Ok(after) = runtime.commit_quest_platform_rider(proof, || {
                map.facts
                    .apply_quest_map_item_batch(map.overlay.base(), &batch)
            }) else {
                return UseDisposition::Rejected;
            };
            self.stage_platform_player(
                runtime,
                states,
                map,
                actor,
                session,
                expected,
                after,
                optional_pvp,
            );
        } else {
            let to = local(trip.evacuate);
            let cell = LogicalCell {
                x: to.x,
                y: to.y,
                z: i32::from(to.floor),
            };
            if !self
                .movement_cells
                .index()
                .lookup(self.movement_cells.scope(), cell)
                .is_ok_and(|c| c == CollisionClass::Walkable)
            {
                return UseDisposition::NotPossible;
            }
            let Ok(census) = runtime.positioned_melee_census() else {
                return UseDisposition::Rejected;
            };
            let occupants: Vec<_> = census
                .into_iter()
                .filter(|(_, p, _)| p.position() == local(trip.from))
                .collect();
            for &(who, _, member_session) in &occupants {
                if let Some(member_session) = member_session
                    && (runtime.bound_attacker_lease(who, member_session).is_err()
                        || states.is_dead(who)
                        || !states.spell_idle(runtime, who, member_session)
                        || !self.states.player_trades.physical(member_session, None)
                        || self.has_pending_quest_callback(member_session)
                        || (super::super::pvp_live::pz_blocked(
                            states,
                            who,
                            member_session,
                            self.owner_now(),
                        ) && !self
                            .movement_cells
                            .spell_tiles()
                            .lookup(self.movement_cells.scope(), cell)
                            .is_ok_and(|tile| !tile.flags().protection_zone)))
                {
                    return UseDisposition::Rejected;
                }
            }
            let Ok(moved) = runtime.commit_quest_platform_displacement(
                local(trip.from),
                to,
                |who, member_session, _| {
                    occupants
                        .iter()
                        .any(|(actor, _, session)| *actor == who && *session == member_session)
                },
                || {
                    map.facts
                        .apply_quest_map_item_batch(map.overlay.base(), &batch)
                },
            ) else {
                return UseDisposition::Rejected;
            };
            for displacement in moved {
                if let Some(member_session) = displacement.session {
                    self.stage_platform_player(
                        runtime,
                        states,
                        map,
                        displacement.actor,
                        member_session,
                        displacement.before,
                        displacement.after,
                        (displacement.actor == actor)
                            .then_some(optional_pvp)
                            .flatten(),
                    );
                }
            }
        }
        map.facts
            .emit_movement_speech(crate::map::facts::MovementSpeech {
                speaker: actor,
                position: tile(trip.sound_position),
                talk_type: 37,
                text: trip.sound.into(),
                captured_name: None,
            });
        UseDisposition::Committed
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "shared consequences retain each displaced player's exact before and after snapshots"
    )]
    fn stage_platform_player(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        before: MovementPositionSnapshot,
        after: MovementPositionSnapshot,
        optional_pvp: Option<bool>,
    ) {
        states.stage_special_movement_step(
            runtime,
            map,
            actor,
            session,
            self.owner_now().get(),
            Some(before.position()),
            after.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            runtime,
            actor,
            session,
            self.owner_now().get(),
            underwater(map, before.position()),
            underwater(map, after.position()),
        );
    }
}
