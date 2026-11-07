//! Closed native successor for three donor Cupcakes; House custody is independently proved.
use super::*;
use crate::durability::character_condition_snapshot::CharacterConditionSnapshot;
use crate::durability::item_use::cupcakes::CupcakeTransformContext;
use crate::foundation::RuntimeScopeRefV1;
use sha2::{Digest, Sha256};

use super::super::scoped::{scoped_item_current as current, scoped_item_position};

impl ChannelSpellStates {
    pub(super) fn prepare_cupcake_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        preparation: NoveltyEffectRequest<'_>,
    ) -> Result<(), UseDisposition> {
        let NoveltyEffectRequest {
            actor,
            fence,
            request,
            effects,
            target,
            now_us,
        } = preparation;
        if target.is_some()
            || !matches!(effects.source_item_id, 28484..=28486)
            || request.command.game_session_id() != fence.game_session_id
        {
            return Err(UseDisposition::Rejected);
        }
        if let Some(memo) = self
            .pending_item_effects
            .iter()
            .find(|memo| memo.actor == actor && memo.request.command == request.command)
        {
            return if memo.fence == fence
                && memo.request.source == request.source
                && memo.request.intent == request.intent
                && memo.request.target == request.target
            {
                Ok(())
            } else {
                Err(UseDisposition::Rejected)
            };
        }
        if self.is_dead(actor)
            || !current(runtime, actor, fence, false)
            || self.has_pending_spell_commit(actor, fence.game_session_id)
            || matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
                && runtime.assert_actor_spell_unreserved(actor).is_err()
            || native
                .item_policy_for_source_id(effects.source_item_id)
                .is_none_or(|policy| !policy.record().admission.materializable)
        {
            return Err(UseDisposition::Rejected);
        }
        let state = self
            .actors
            .iter()
            .find(|(a, s, _)| *a == actor && *s == fence.game_session_id)
            .map(|(_, _, state)| state)
            .ok_or(UseDisposition::StaleState)?;
        let before_revision = state.revision();
        let after = crate::spell::actor_conditions::stage_cupcake(
            state,
            effects.source_item_id,
            now_us / 1000,
        )
        .map_err(|_| UseDisposition::NotPossible)?;
        self.pending_item_effects
            .try_reserve(1)
            .map_err(|_| UseDisposition::NotPossible)?;
        let cue_position = if let Some(ground) = &effects.ground {
            crate::foundation::MovementLocalPosition {
                x: ground.x,
                y: ground.y,
                floor: ground.floor,
            }
        } else {
            scoped_item_position(runtime, actor, fence)?
        };
        self.pending_item_effects.push(PendingItemEffect {
            actor,
            fence,
            request: request.clone(),
            effects,
            cue_position,
            before_revision,
            after: Some(after),
            death: None,
        });
        Ok(())
    }

    pub(in crate::gameplay_transport) fn cupcake_transform_context(
        &self,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
        digest: [u8; 32],
        now_us: u64,
    ) -> Result<Option<CupcakeTransformContext>, ()> {
        let Some(memo) = self.pending_item_effects.iter().find(|memo| {
            memo.actor == actor
                && memo.request.command == request.command
                && matches!(memo.effects.source_item_id, 28484..=28486)
        }) else {
            return Ok(None);
        };
        if memo.request.source != request.source
            || memo.request.target != request.target
            || memo.request.intent != request.intent
        {
            return Err(());
        }
        let mut hash = Sha256::new();
        hash.update(b"oteryn:cupcake-transform:v1\0");
        hash.update(request.source);
        hash.update(&request.intent);
        Ok(Some(CupcakeTransformContext {
            item_id: memo.effects.source_item_id,
            wire_binding: hash.finalize().into(),
            conditions: CharacterConditionSnapshot {
                lease_generation: memo.fence.character_lease_generation,
                content_digest: digest,
                conditions: memo
                    .after
                    .as_ref()
                    .ok_or(())?
                    .owned_conditions()
                    .persistent_snapshot(now_us / 1000 * 1000)
                    .map_err(|_| ())?,
            },
        }))
    }

    pub(super) fn settle_cupcake_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
        receipt: &ConsumeTransformResult,
    ) -> Result<Option<NoveltyEffects>, ()> {
        let Some(index) = self.pending_item_effects.iter().position(|memo| {
            memo.actor == actor
                && memo.request.command == request.command
                && matches!(memo.effects.source_item_id, 28484..=28486)
        }) else {
            return Ok(None);
        };
        if receipt.disposition != UseDisposition::Committed {
            self.release_novelty_effect(actor, request);
            return Ok(None);
        }
        let memo = &self.pending_item_effects[index];
        let state_index = self
            .actors
            .iter()
            .position(|(a, s, _)| *a == actor && *s == request.command.game_session_id())
            .ok_or(())?;
        let state = &self.actors[state_index].2;
        if self.is_dead(actor)
            || !current(runtime, actor, memo.fence, true)
            || memo.request.source != request.source
            || memo.request.target != request.target
            || memo.request.intent != request.intent
            || state.revision() != memo.before_revision
            || memo
                .after
                .as_ref()
                .is_none_or(|after| state.revision().checked_add(1) != Some(after.revision()))
            || serde_json::to_value(&memo.effects).map_err(|_| ())? != receipt.effects
        {
            return Err(());
        }
        let memo = self.pending_item_effects.remove(index);
        self.actors[state_index].2 = memo.after.ok_or(())?;
        self.owner_wake.notify_one();
        Ok(Some(memo.effects))
    }
}
