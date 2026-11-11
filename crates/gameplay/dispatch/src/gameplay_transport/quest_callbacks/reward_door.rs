//! A reward door reads the actual once-claim and commits through the native door book.
use super::*;
use crate::map::door::DoorUse;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the selected native door and sealed movement share one owner turn"
    )]
    pub(super) async fn execute_reward_door(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        expected_revision: u64,
        fence: Option<CurrentCharacterGameplayFence>,
        optional_pvp: Option<bool>,
    ) -> UseDisposition {
        let (Some(door), Some(fence), Some((target, _))) = (
            callback.effects.reward_door.as_ref(),
            fence,
            crate::map::view::placement_of(key),
        ) else {
            return UseDisposition::Rejected;
        };
        let doors = map.facts.doors();
        if !doors.contains(key) {
            return UseDisposition::Rejected;
        }
        match self
            .root
            .reward_claim_is_claimed(
                self.character,
                self.holder,
                fence.item_fence(),
                door.claim.clone(),
            )
            .await
        {
            Ok(false) => {}
            Ok(true) => {
                if let Some(message) = &callback.failure_message {
                    self.states.chat.push_quest_feedback(session, message);
                }
                return UseDisposition::NotPossible;
            }
            Err(_) => return UseDisposition::Rejected,
        }
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return UseDisposition::Rejected;
        };
        let Some(origin) = native(expected.position()) else {
            return UseDisposition::Rejected;
        };
        let level = states
            .get(runtime, actor, session)
            .map_or(0, |s| s.character_facts().level);
        if !doors.blocks(target) {
            let Ok(census) = runtime.positioned_actor_census() else {
                return UseDisposition::Rejected;
            };
            let occupants = census
                .iter()
                .filter(|(_, at, _)| at.position() == destination_of(target))
                .collect::<Vec<_>>();
            if occupants.len() > 1 || map.overlay.has_added_item(target) {
                return UseDisposition::Occupied;
            }
            if let Some((who, expected, who_session)) = occupants.first().copied() {
                let blocked = who_session.is_some_and(|s| {
                    super::super::pvp_live::pz_blocked(states, *who, s, self.owner_now())
                });
                let Ok(proof) = crate::map::floor::prepare_quest_door_displacement(
                    runtime,
                    &self.movement_cells,
                    doors,
                    key,
                    *who,
                    *expected,
                    blocked,
                ) else {
                    return UseDisposition::NotPossible;
                };
                let Ok(snapshot) = runtime.commit_source_step_and_project(proof, || {
                    matches!(
                        doors.toggle(key, expected_revision, false, level, true),
                        Some(DoorUse::Toggled { open: false, .. })
                    )
                }) else {
                    return UseDisposition::Rejected;
                };
                if let Some(who_session) = who_session {
                    states.stage_special_movement_step(
                        runtime,
                        map,
                        *who,
                        *who_session,
                        self.owner_now().get(),
                        Some(expected.position()),
                        snapshot.position(),
                        optional_pvp,
                    );
                    states.stage_underwater_step(
                        runtime,
                        *who,
                        *who_session,
                        self.owner_now().get(),
                        underwater(map, expected.position()),
                        underwater(map, snapshot.position()),
                    );
                }
                return UseDisposition::Committed;
            }
            let occupied = super::super::map_door::doorway_occupied(runtime, actor, origin, target);
            return match doors.toggle(key, expected_revision, occupied, level, true) {
                Some(DoorUse::Toggled { .. }) => UseDisposition::Committed,
                Some(DoorUse::Occupied) => UseDisposition::Occupied,
                Some(DoorUse::Stale) => UseDisposition::StaleState,
                Some(DoorUse::Locked) => UseDisposition::Locked,
                None => UseDisposition::Rejected,
            };
        }
        let blocked = super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
        let Ok(proof) = crate::map::floor::prepare_quest_door_opening(
            runtime,
            &self.movement_cells,
            map,
            actor,
            session,
            expected,
            key,
            blocked,
        ) else {
            return UseDisposition::NotPossible;
        };
        let Ok(snapshot) = runtime.commit_source_step_and_project(proof, || {
            matches!(
                doors.toggle(key, expected_revision, false, level, true),
                Some(DoorUse::Toggled { open: true, .. })
            )
        }) else {
            return UseDisposition::Rejected;
        };
        states.stage_special_movement_step(
            runtime,
            map,
            actor,
            session,
            self.owner_now().get(),
            Some(expected.position()),
            snapshot.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            runtime,
            actor,
            session,
            self.owner_now().get(),
            underwater(map, expected.position()),
            underwater(map, snapshot.position()),
        );
        UseDisposition::Committed
    }
}

fn destination_of(target: TilePos) -> MovementLocalPosition {
    local(Position {
        x: target.x,
        y: target.y,
        floor: target.floor,
    })
}
