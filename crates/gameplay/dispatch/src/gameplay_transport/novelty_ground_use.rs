//! Ground actions share the carried action receipt and pending effect owner.
use super::*;
use crate::durability::consume_transform::GroundActionContext;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::gameplay_transport::actor_spell::NoveltyEffectRequest;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_ground_event_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        request: &ConsumeTransformRequest,
        permit: &SpellLanePermit,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(None);
        };
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(source) = self
            .root
            .read_ground_action_source(self.character, fence, request.source)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        if !novelty_items::is_ground_action(&source.definition().production_key) {
            return Ok(None);
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(request.source))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let Some(room) = self.rooms().gate_ground_item_action() else {
            return Ok(Some(outcome(UseDisposition::NotPossible)));
        };
        let context = {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let definition = source.definition().clone();
            let context = match GroundActionContext::from_native_owner(
                room,
                &runtime,
                actor,
                command.game_session_id,
                source,
            ) {
                Ok(context) => context,
                Err(_) => return Ok(Some(outcome(UseDisposition::TargetIllegal))),
            };
            let plan = if oteryn_gameplay_items::event_items::is_event_reward(
                &definition.production_key,
            ) {
                oteryn_gameplay_items::event_items::plan(
                    native,
                    &definition,
                    *command.game_session_id.as_bytes(),
                    command.command_id,
                    request.source,
                    Some(context.effect_position()),
                )
            } else {
                novelty_items::plan_ground(
                    native,
                    &definition,
                    *command.game_session_id.as_bytes(),
                    command.command_id,
                    request.source,
                    context.effect_position(),
                    context.in_house(),
                )
            };
            let Some(plan) = plan else {
                return Ok(Some(outcome(UseDisposition::NotPossible)));
            };
            let effects: NoveltyEffects = serde_json::from_value(plan.effects).map_err(|_| ())?;
            if intent.use_with_map_item.is_some()
                || intent.use_with_position.is_some()
                || (!effects.require_player_target && intent.use_with.is_some())
            {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            }
            let target = if effects.require_player_target {
                let Some(wanted) = intent.use_with else {
                    return Ok(Some(outcome(UseDisposition::NoTarget)));
                };
                let visible = runtime.visible_entities();
                let Some(target) = visible.players.iter().find(|entry| {
                    entry.actor.placement_identity() == wanted.identity
                        && entry.generation == wanted.generation
                }) else {
                    return Ok(Some(outcome(UseDisposition::TargetIllegal)));
                };
                let from = runtime
                    .read_actor_position(actor)
                    .map_err(|_| ())?
                    .position();
                if !oteryn_gameplay_items::reach::within_reach(
                    (from.x, from.y, i32::from(from.floor)),
                    (
                        target.position.x,
                        target.position.y,
                        i32::from(target.position.floor),
                    ),
                ) {
                    return Ok(Some(outcome(UseDisposition::TooFar)));
                }
                Some(target.actor)
            } else {
                None
            };
            if let Err(disposition) = states.prepare_novelty_effect(
                &runtime,
                native,
                NoveltyEffectRequest {
                    actor,
                    fence,
                    request,
                    effects,
                    target,
                    now_us: self.owner_now().get(),
                },
            ) {
                return Ok(Some(outcome(disposition)));
            }
            context
        };
        let content = NativeItemWeights(native);
        let receipt = self
            .root
            .consume_transform_on_ground(
                permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                &context,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        self.finish_novelty_item(actor, request, receipt)
            .await
            .map(Some)
    }
}
