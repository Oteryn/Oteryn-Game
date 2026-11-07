//! Closed Santa gift USE shares the current House source preflight and native cue owner.
use super::*;
use crate::durability::consume_transform::{ConsumeTransformRequest, HouseActionContext};
use crate::gameplay_transport::actor_spell::NoveltyEffectRequest;
use crate::gameplay_transport::connection::{UseCommand, UseOutcome};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_items::novelty_items::{GroundEffect, NoveltyEffects, santa_gifts};
use oteryn_protocol_oteryn::world_object::UseDisposition;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_house_santa_gift(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &crate::content::native_gameplay::NativeGameplayState,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(item) = self
            .root
            .read_house_interactive_source(self.holder, context.clone(), request.source)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        if !item
            .definition
            .production_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u32>().ok())
            .is_some_and(santa_gifts::recognizes)
        {
            return Ok(None);
        }
        let position = item.position;
        let at = crate::foundation::MovementLocalPosition {
            x: position[0],
            y: position[1],
            floor: i16::try_from(position[2]).map_err(|_| ())?,
        };
        let Some(plan) = santa_gifts::plan(
            native,
            &item.definition,
            *command.game_session_id.as_bytes(),
            command.command_id,
            request.source,
            Some(GroundEffect {
                x: at.x,
                y: at.y,
                floor: at.floor,
            }),
        ) else {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            }));
        };
        let effects: NoveltyEffects = serde_json::from_value(plan.effects).map_err(|_| ())?;
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if let Err(disposition) = states.prepare_scoped_gift_effect(
                &runtime,
                native,
                NoveltyEffectRequest {
                    actor,
                    fence: command.item_fence.ok_or(())?,
                    request: &request,
                    effects: effects.clone(),
                    target: None,
                    now_us: self.owner_now().get(),
                },
            ) {
                return Ok(Some(UseOutcome {
                    disposition,
                    ..UseOutcome::rejected()
                }));
            }
        }
        // The dragon's fifth branch is on the player; speech retains clicked source.
        let cue_at = if matches!(effects.source_item_id, 5791 | 6566) && effects.damage == Some(1) {
            let p = context.position();
            crate::foundation::MovementLocalPosition {
                x: p[0],
                y: p[1],
                floor: i16::try_from(p[2]).map_err(|_| ())?,
            }
        } else {
            at
        };
        let reservation = match self
            .prepare_house_item_cues(actor, command, cue_at, &effects)
            .await
        {
            Ok(reservation) => reservation,
            Err(()) => {
                self.states
                    .spell_states
                    .lock()
                    .await
                    .release_novelty_effect(actor, &request);
                return Err(());
            }
        };
        let result = self
            .root
            .use_santa_gift_in_house(
                self.holder,
                context,
                request.clone(),
                native,
                &NativeItemWeights(native),
                item.definition,
                item.state_revision,
            )
            .await
            .map_err(|_| ())?;
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            states.settle_scoped_gift_effect(&runtime, actor, &request, &result)?;
        }
        self.finish_house_item_cues(actor, command, reservation, &result)
            .await?;
        Ok(Some(UseOutcome {
            disposition: result.disposition,
            ..UseOutcome::rejected()
        }))
    }
}
