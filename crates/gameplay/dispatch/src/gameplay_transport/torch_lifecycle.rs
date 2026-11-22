//! Ordinary USE of a visible torch uses the same command and actor fences as other Item actions.
use super::ComposedFreshAdmission;
use super::connection::{UseCommand, UseOutcome};
use super::item_view::ItemKey;
use crate::durability::torch_lifecycle::TorchUse;
use crate::foundation::{CommandId, CommandRef, ExactActorRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_torch_intent(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        if !matches!(intent.target, UseTarget::Item(_))
            || intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Ok(None);
        }
        let Some(index) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.item_index())
        else {
            return Ok(None);
        };
        if (2920..=2926).any(|id| {
            index
                .definition_ref("Item", &format!("oteryn:item.tibia.i{id}"), "definition-r1")
                .is_none()
        }) {
            return Ok(None);
        }
        let item = match source {
            Some(ItemKey::Instance(id)) => Some(id),
            None => None, // A committed command can be retried after its handle retired.
            _ => return Ok(None),
        };
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, item)
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let request = {
            let runtime = self.runtime.lock().await;
            let pin = runtime.content_pin();
            if runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || !runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
                || self.states.spell_states.lock().await.is_dead(actor)
            {
                return Ok(Some(UseOutcome::rejected()));
            }
            let position = runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .position();
            TorchUse {
                command: CommandRef::new(
                    command.game_session_id,
                    CommandId::new(command.command_id).map_err(|_| ())?,
                ),
                item,
                intent: encode_item_use_intent(intent).map_err(|_| ())?,
                position,
                map_digest: pin.map_revision_digest(),
                frame_digest: pin.frame_binding_digest(),
                content_digest: pin.server_artifact_digest(),
            }
        };
        let mut ground = super::loop_move::GroundChange(Some(&self.states.loop_ground));
        let result = self
            .root
            .use_torch(&permit, self.character, self.holder, fence, request)
            .await
            .map_err(|_| ())?;
        if result != Some(UseDisposition::Committed) {
            ground.0 = None;
        }
        Ok(result.map(|disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }))
    }
}
