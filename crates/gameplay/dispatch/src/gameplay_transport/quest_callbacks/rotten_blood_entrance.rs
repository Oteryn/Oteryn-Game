//! Crystal 201189f movements_blood_entrance.lua:1-44, a World position StepIn.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_rotten_blood_entrance(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        let at = entered.position();
        if (at.x, at.y, at.floor) != (32953, 32398, 9) || origin.position() == at {
            return None;
        }
        let room = self.effective_rooms().gate_monster_ai()?;
        let active = self.active_generation?;
        let map = self.bundle_map()?;
        let runtime = self.runtime.lock().await;
        let context = active.qualify_published_world_context(room, runtime.content_pin())?;
        let frame = context.source_frame();
        if frame.source_position((at.x, at.y, at.floor)) != Some([32953, 32398, 9])
            || !matches!(
                runtime.binding().runtime_scope(),
                crate::foundation::RuntimeScopeRefV1::Channel { .. }
            )
        {
            return None;
        }
        drop(runtime);

        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return Some(entered);
        };
        let access = self
            .root
            .read_rotten_blood_state(self.character, self.holder, fence)
            .await
            .ok()
            .map(|state| state.access());

        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if !context.matches(room, runtime.content_pin())
            || runtime.owner_fence().is_err()
            || runtime.read_actor_position(actor) != Ok(entered)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || states.is_dead(actor)
        {
            return Some(entered);
        }
        if runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || !runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return Some(entered);
        }
        let Some(player) = states.get(&runtime, actor, session) else {
            return Some(entered);
        };
        let (source_destination, message, penalize) = if player.character_facts().level < 250 {
            (None, Some("You need at least level 250 to enter."), false)
        } else {
            match access {
                Some(true) => (Some([34070, 31975, 14]), None, false),
                Some(false) => (
                    Some([32955, 32398, 9]),
                    Some(
                        "You offerings to this sanguine master of this realm have been insufficient. You can not pass.",
                    ),
                    true,
                ),
                None => (None, Some("Sorry, not possible."), false),
            }
        };
        let destination = match source_destination {
            Some(source) => {
                let (x, y, floor) = frame.physical_position(source)?;
                MovementLocalPosition { x, y, floor }
            }
            None => origin.position(),
        };
        let now = self.owner_now();
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let blocked = super::super::pvp_live::pz_blocked(&states, actor, session, now);
        let proof = crate::map::floor::prepare_scripted_quest_teleport_facing(
            &runtime,
            self.movement_cells(),
            actor,
            session,
            entered,
            destination,
            blocked,
            &closed,
            entered.facing(),
            true,
        );
        let returned = proof
            .ok()
            .and_then(|proof| runtime.commit_source_step(proof).ok());
        if let Some(message) = message {
            self.states.server_messages.send_current(
                session,
                oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(message.into()),
            );
        }
        let returned = returned.unwrap_or(entered);
        if returned != entered {
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                now.get(),
                Some(at),
                returned.position(),
                None,
            );
            states.stage_underwater_step(
                &runtime,
                actor,
                session,
                now.get(),
                underwater(map, at),
                underwater(map, returned.position()),
            );
        }
        if penalize {
            states.apply_blood_entrance_health_loss(&runtime, actor, session, now);
        }
        if message.is_none()
            && returned != entered
            && let Some(at) = native(returned.position())
        {
            map.facts.emit_movement_effect(at, 11);
        }
        Some(returned)
    }
}
