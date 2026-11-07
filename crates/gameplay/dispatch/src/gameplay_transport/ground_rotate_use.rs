//! Rotate of an actual materialized Channel Ground item, with unchanged custody.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::durability::consume_transform::{ConsumeTransformRequest, GroundActionContext};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn rotate_ground_world_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        observed: Option<std::num::NonZeroU32>,
    ) -> Result<Option<UseOutcome>, ()> {
        let (Some(ItemKey::Instance(instance)), Some(fence), Some(native)) = (
            source,
            command.item_fence,
            self.active_generation.and_then(|a| a.native_gameplay()),
        ) else {
            return Ok(None);
        };
        let Some(source) = self
            .root
            .read_ground_action_source(self.character, fence, instance)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        let Some(room) = self.rooms.gate_ground_item_action() else {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            }));
        };
        let _lane = self.spell_lane_permit().await.ok_or(())?;
        let context = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return Ok(Some(UseOutcome::rejected()));
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
            {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::Exhausted,
                    ..UseOutcome::rejected()
                }));
            }
            GroundActionContext::from_native_owner(
                room,
                &runtime,
                actor,
                command.game_session_id,
                source,
            )
            .map_err(|_| ())?
        };
        let request = ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            source: instance,
            target: None,
            intent: super::world_rotate::bound_intent(intent, observed).map_err(|_| ())?,
        };
        let result = self
            .root
            .consume_transform_on_ground(
                &_lane,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &super::item_ref_admission::NativeItemWeights(native)
                    .with_wheel(self.premium_coordinator),
                &context,
            )
            .await
            .map_err(|_| ())?;
        if result
            .as_ref()
            .is_some_and(|r| r.disposition == UseDisposition::Committed && r.fresh)
        {
            self.states.loop_ground.invalidate();
        }
        Ok(result.map(|r| UseOutcome {
            disposition: r.disposition,
            ..UseOutcome::rejected()
        }))
    }
}
