//! Crystal 201189f8, threatened_dreams_quest/movement_kroazur_room.lua:19-78.
//! A committed player StepIn requests the shared exclusive boss room.
use super::ComposedFreshAdmission;
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementPositionSnapshot, RuntimeScopeRefV1,
};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_entities::carrier::native_translation_events::CommittedMovementOccurrence;
use oteryn_gameplay_quests::quest::threatened_dreams::KROAZUR_BOSS;
use oteryn_protocol_oteryn::{CommandStatus, boss_encounter as wire};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn kroazur_step_in(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        let Some(map) = self.bundle_map() else {
            return None;
        };
        if !matches!(actor.runtime_scope(), RuntimeScopeRefV1::Channel { .. }) {
            return None;
        }
        let p = entered.position();
        // This action also marks the exit, which has its own unconditional callback.
        if (p.x, p.y, p.floor) == (33591, 32321, 10) {
            return None;
        }
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(p.x),
            u16::try_from(p.y),
            i8::try_from(-i32::from(p.floor)),
        ) else {
            return None;
        };
        let pos = TilePos { x, y, floor };
        let Some(base) = map.overlay.base().tile(x, y, floor) else {
            return None;
        };
        if !(0..base.depths().iter().filter(|depth| **depth == 0).count()).any(|ordinal| {
            u8::try_from(ordinal)
                .ok()
                .and_then(|o| placement_key(pos, o))
                .is_some_and(|key| {
                    !map.facts.base_hidden(key) && map.facts.action_at(key) == Some(25029)
                })
        }) {
            return None;
        }
        let movement = {
            let runtime = self.runtime.lock().await;
            let Ok(Some(movement)) = runtime.committed_movement_occurrence(actor) else {
                return None;
            };
            if movement.after() != entered
                || movement.before().position() == p
                || runtime.read_actor_position(actor) != Ok(entered)
            {
                return None;
            }
            movement
        };
        let Some(admitted) = self.activity_runtimes.connection(session) else {
            return self.return_from_kroazur_entrance(session, movement).await;
        };

        let Ok(payload) = wire::encode_boss_key(KROAZUR_BOSS) else {
            return None;
        };
        let retry = super::AdmissionRetryBudget::for_command(self.root);
        let reply = retry
            .retry_unavailable(async || {
                self.boss_activity_step(admitted, &payload, movement)
                    .await
                    .ok_or(super::connection::AdmissionRefusal::Unavailable)
            })
            .await;
        match reply {
            Ok((CommandStatus::Accepted, _)) => None,
            Ok((_, payload)) => {
                let returned = self.return_from_kroazur_entrance(session, movement).await;
                let reason = match wire::decode_boss_activity_reply(&payload) {
                    Ok(wire::BossActivityReply::Refused(wire::BossActivityRefusal::Cooldown)) => {
                        "You have to wait before you can fight Kroazur again."
                    }
                    Ok(wire::BossActivityReply::Refused(wire::BossActivityRefusal::NoCapacity)) => {
                        "Someone is already fighting Kroazur."
                    }
                    _ => "You cannot enter Kroazur's room.",
                };
                self.states.chat.push_quest_feedback(session, reason);
                returned
            }
            Err(_) => {
                super::operator_event("kroazur_entry_pending");
                None
            }
        }
    }

    async fn return_from_kroazur_entrance(
        &self,
        session: GameSessionId,
        movement: CommittedMovementOccurrence,
    ) -> Option<MovementPositionSnapshot> {
        let (Some(permit), Some(map)) = (self.spell_lane_permit().await, self.bundle_map()) else {
            return None;
        };
        let actor = movement.mover();
        let policy = self.special_movement_policy(&permit, actor, session).await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.committed_movement_occurrence(actor) != Ok(Some(movement))
            || runtime.read_actor_position(actor) != Ok(movement.after())
            || states.is_dead(actor)
        {
            return None;
        }
        let origin = movement.before().position();
        let entered = movement.after().position();
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            movement.after(),
            origin,
            false,
            &doors,
        ) else {
            return None;
        };
        let returned = runtime.commit_source_step(proof).ok()?;
        let now = self.owner_now().get();
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            now,
            Some(entered),
            origin,
            policy,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            now,
            super::paced_movement::underwater_drown::underwater(map, entered),
            super::paced_movement::underwater_drown::underwater(map, origin),
        );
        Some(returned)
    }
}
