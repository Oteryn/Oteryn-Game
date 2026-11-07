//! Three placed House foods use the existing pending native successor and durable burn.
use super::*;
use crate::foundation::RuntimeScopeRefV1;
use crate::gameplay_transport::actor_spell::NoveltyEffectRequest;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_house_cupcake(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. })
            || !matches!(intent.target, UseTarget::Item(_))
            || intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
        {
            return Ok(None);
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(None);
        };
        let encoded =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let pending = {
            let states = self.states.spell_states.lock().await;
            states
                .pending_item_effects
                .iter()
                .find(|memo| {
                    memo.actor == actor
                        && memo.request.command == command_ref
                        && memo.request.intent == encoded
                        && matches!(memo.effects.source_item_id, 28484..=28486)
                })
                .map(|memo| memo.request.source)
        };
        let source_id = pending
            .or(match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            })
            .unwrap_or([0; 16]);
        let request = ConsumeTransformRequest {
            command: command_ref,
            source: source_id,
            target: None,
            intent: encoded,
        };
        if let Some(receipt) = self
            .root
            .probe_cupcake_transform_receipt(self.character, fence, request.clone())
            .await
            .map_err(|_| ())?
        {
            return self
                .finish_novelty_item(actor, &request, receipt)
                .await
                .map(Some);
        }
        if source_id == [0; 16] {
            return Ok(None);
        }
        let _lane = self.spell_lane_permit().await.ok_or(())?;
        let context = {
            let runtime = self.runtime.lock().await;
            if crate::gameplay_transport::item_ref_admission::pinned_index(
                &runtime,
                self.active_generation,
            )
            .is_none()
            {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::StaleState,
                    ..UseOutcome::rejected()
                }));
            }
            HouseActionContext::from_native_owner(&runtime, actor, command.game_session_id, fence)
                .map_err(|_| ())?
        };
        let Some(snapshot) = self
            .root
            .read_house_action_items(self.holder, context.clone())
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        let Some(item) = snapshot.items.iter().find(|item| {
            item.item_instance_id == source_id
                && oteryn_gameplay_items::seasonal_cupcakes::source_id(
                    &item.definition.production_key,
                )
                .is_some()
        }) else {
            return Ok(None);
        };
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(source_id))
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let Some(plan) = oteryn_gameplay_items::seasonal_cupcakes::plan(native, &item.definition)
        else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let effects = serde_json::from_value(plan.effects).map_err(|_| ())?;
        let cupcake = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let now_us = self.owner_now().get();
            if let Err(disposition) = states.prepare_novelty_effect(
                &runtime,
                native,
                NoveltyEffectRequest {
                    actor,
                    fence,
                    request: &request,
                    effects,
                    target: None,
                    now_us,
                },
            ) {
                return Ok(Some(UseOutcome {
                    disposition,
                    ..UseOutcome::rejected()
                }));
            }
            states
                .cupcake_transform_context(actor, &request, native.source_digest(), now_us)?
                .ok_or(())?
        };
        let result = self
            .root
            .consume_cupcake_in_house(self.holder, context, request.clone(), native, &cupcake)
            .await
            .map_err(|_| ())?;
        self.finish_novelty_item(actor, &request, result)
            .await
            .map(Some)
    }
}
