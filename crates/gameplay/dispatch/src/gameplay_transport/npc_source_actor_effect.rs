//! One retained Ajax condition successor per current actor; no post-commit work queue.
mod hazard;
mod observation;
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use crate::durability::character_condition_snapshot::CharacterConditionSnapshot;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::npc_cosmetics::{NpcCosmeticRequest, NpcSourceActorEffect};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::spell::cast::PlayerSpellState;
pub(in crate::gameplay_transport) use hazard::PendingNpcHazardEffect;
#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct PendingNpcAjaxEffect {
    pub actor: ExactActorRef,
    pub session: GameSessionId,
    fence: CurrentCharacterGameplayFence,
    request: NpcCosmeticRequest,
    effect: NpcSourceActorEffect,
    before: PlayerSpellState,
    after: PlayerSpellState,
}
fn current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
) -> bool {
    runtime.owner_fence().is_ok()
        && runtime.binding().runtime_scope() == fence.runtime_scope
        && runtime.binding().scope_generation() == fence.scope_ownership_generation
        && runtime
            .player_control_facts(actor, fence.game_session_id)
            .is_ok()
        && runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .is_ok_and(|l| {
                l.character_id().as_bytes() == fence.character_id.as_bytes()
                    && l.generation() == fence.character_lease_generation
            })
}
impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn prepare_npc_ajax_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        request: &NpcCosmeticRequest,
        content: [u8; 32],
        now_us: u64,
    ) -> Result<NpcSourceActorEffect, ()> {
        if !crate::durability::npc_cosmetics::requires_ajax_fire(request)
            || !current(runtime, actor, fence)
            || !runtime
                .player_control_facts(actor, fence.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
            || content != runtime.content_pin().server_artifact_digest()
            || self.is_dead(actor)
            || !self.spell_idle(runtime, actor, fence.game_session_id)
            || self
                .pending_npc_ajax_effects
                .iter()
                .any(|p| p.actor == actor)
        {
            return Err(());
        }
        let before = self
            .get(runtime, actor, fence.game_session_id)
            .ok_or(())?
            .clone();
        let after = crate::spell::actor_conditions::stage_ajax_source_fire(&before, now_us)
            .map_err(|_| ())?;
        let snapshot = |state: &PlayerSpellState| -> Result<CharacterConditionSnapshot, ()> {
            Ok(CharacterConditionSnapshot {
                lease_generation: fence.character_lease_generation,
                content_digest: content,
                conditions: state
                    .owned_conditions()
                    .persistent_snapshot(now_us)
                    .map_err(|_| ())?,
            })
        };
        let effect = NpcSourceActorEffect {
            actor_identity: actor.placement_identity(),
            movement_revision: runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .revision(),
            state_revision: before.revision(),
            before: snapshot(&before)?,
            after: snapshot(&after)?,
        };
        self.pending_npc_ajax_effects
            .try_reserve(1)
            .map_err(|_| ())?;
        self.pending_npc_ajax_effects.push(PendingNpcAjaxEffect {
            actor,
            session: fence.game_session_id,
            fence,
            request: request.clone(),
            effect: effect.clone(),
            before,
            after,
        });
        Ok(effect)
    }
    fn settle_npc_ajax_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        pending: &PendingNpcAjaxEffect,
        committed: bool,
    ) -> Result<(), ()> {
        let state = self
            .get(runtime, pending.actor, pending.session)
            .ok_or(())?;
        let is_before = state == &pending.before;
        let is_after = state == &pending.after;
        if committed
            && !is_after
            && (!is_before
                || !self.commit(
                    runtime,
                    pending.actor,
                    pending.session,
                    pending.after.clone(),
                ))
        {
            return Err(());
        }
        if !committed && !is_before {
            return Err(());
        }
        self.pending_npc_ajax_effects
            .retain(|p| p.actor != pending.actor || p.session != pending.session);
        Ok(())
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called only after the original parent command is known terminal or under
    /// its retained unknown-outcome reconciliation. It never restages the effect.
    pub(in crate::gameplay_transport) async fn resolve_npc_ajax_effect(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), ()> {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Some(pending) = states
            .pending_npc_ajax_effects
            .iter()
            .find(|p| p.actor == actor && p.session == session)
            .cloned()
        else {
            return Ok(());
        };
        permit
            .check_scope(pending.fence.runtime_scope)
            .map_err(|_| ())?;
        if !current(&runtime, actor, pending.fence) {
            return Err(());
        }
        let committed = self
            .root
            .read_npc_ajax_actor_effect(
                self.character,
                pending.fence,
                pending.request.clone(),
                pending.effect.clone(),
            )
            .await
            .map_err(|_| ())?;
        states.settle_npc_ajax_effect(&runtime, &pending, committed)
    }
}

pub(in crate::gameplay_transport) struct NpcSourceCall<'a> {
    pub fence: CurrentCharacterGameplayFence,
    pub request: NpcCosmeticRequest,
    pub permit: &'a crate::durability::spell_owner_commit::SpellLanePermit,
    pub native: &'a crate::content::native_gameplay::NativeGameplayState,
    pub premium:
        Option<&'a dyn crate::durability::consume_transform_cosmetics::ItemCosmeticPremium>,
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn resolve_pending_npc_ajax_effect(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), ()> {
        if !self
            .states
            .spell_states
            .lock()
            .await
            .pending_npc_ajax_effects
            .iter()
            .any(|pending| pending.actor == actor && pending.session == session)
        {
            return Ok(());
        }
        let permit = self.spell_lane_permit().await.ok_or(())?;
        self.resolve_npc_ajax_effect(&permit, actor, session).await
    }
    pub(in crate::gameplay_transport) async fn commit_npc_source_actor_owned(
        &self,
        actor: ExactActorRef,
        call: NpcSourceCall<'_>,
        effects: &mut crate::durability::npc_cosmetics::NpcSourceEffects,
    ) -> Result<
        Option<crate::durability::npc_cosmetics::NpcCosmeticOutcome>,
        crate::durability::character_progression::CharacterProgressionError,
    > {
        use crate::durability::character_progression::CharacterProgressionError;
        let unknown = || {
            CharacterProgressionError::Unavailable(
                crate::durability::DurabilityError::InvalidStoredState,
            )
        };
        if call.request.npc == "oteryn:npc.gnomadness" {
            return self.commit_npc_hazard_owned(actor, call, effects).await;
        }
        let fire = crate::durability::npc_cosmetics::requires_ajax_fire(&call.request);
        if !fire && effects.greet.is_none() {
            return self
                .root
                .commit_npc_source_with_effects(
                    self.character,
                    self.holder,
                    call.fence,
                    call.request,
                    call.permit,
                    call.native,
                    call.premium,
                    effects,
                )
                .await;
        }
        // Same mature Tokio actor-owner guard order as Cake food. Root has no
        // callback into these locks; conversation std locks are already released.
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if !current(&runtime, actor, call.fence)
            || !runtime
                .player_control_facts(actor, call.fence.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || !states.spell_idle(&runtime, actor, call.fence.game_session_id)
        {
            return Err(unknown());
        }
        if let Some(greet) = &effects.greet {
            let intent = oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                    identity: greet.npc_identity,
                    generation: greet.npc_generation,
                },
                text: call.request.message.clone(),
            };
            let context = self
                .states
                .npc_conversations
                .current_npc(
                    &self.states.chat,
                    &runtime,
                    actor,
                    call.fence.game_session_id,
                    &intent,
                )
                .ok_or_else(unknown)?;
            let expected_catalogue_revision = context.0.to_string();
            if greet.actor_identity != actor.placement_identity()
                || runtime
                    .read_actor_position(actor)
                    .map_err(|_| unknown())?
                    .revision()
                    != greet.movement_revision
                || expected_catalogue_revision.as_str() != greet.catalogue_revision.as_str()
                || context.1 != greet.conversation_generation
                || context.2 != call.request.npc
            {
                return Err(unknown());
            }
        }
        if call.request.npc == "oteryn:npc.vescu"
            && effects.greet.is_some()
            && effects.actor_observation.is_none()
        {
            effects.actor_observation = Some(
                observation::capture(
                    &runtime,
                    &states,
                    actor,
                    call.fence.game_session_id,
                    self.owner_now().get(),
                )
                .ok_or_else(unknown)?,
            );
        }
        if fire {
            effects.actor_effect = Some(
                states
                    .prepare_npc_ajax_effect(
                        &runtime,
                        actor,
                        call.fence,
                        &call.request,
                        call.native.source_digest(),
                        self.owner_now().get(),
                    )
                    .map_err(|_| unknown())?,
            );
        }
        let result = self
            .root
            .commit_npc_source_with_effects(
                self.character,
                self.holder,
                call.fence,
                call.request.clone(),
                call.permit,
                call.native,
                call.premium,
                effects,
            )
            .await;
        // Test-support only: retry the exact retained native occurrence through
        // a second real Root transaction, while this same actor witness is held.
        #[cfg(any(test, feature = "test-support"))]
        if std::env::var_os("OTERYN_SRV7_READONLY_OWNER_PROBE").is_some()
            && !fire
            && effects.greet.is_some()
            && matches!(
                call.request.npc.as_str(),
                "oteryn:npc.vescu"
                    | "oteryn:npc.bron"
                    | "oteryn:npc.a_majestic_warwolf"
                    | "oteryn:npc.bozo"
            )
            && result
                .as_ref()
                .ok()
                .and_then(Option::as_ref)
                .is_some_and(|outcome| !outcome.committed)
        {
            let replay = self
                .root
                .commit_npc_source_with_effects(
                    self.character,
                    self.holder,
                    call.fence,
                    call.request.clone(),
                    call.permit,
                    call.native,
                    call.premium,
                    effects,
                )
                .await?;
            if serde_json::to_value(result.as_ref().map_err(|_| unknown())?)
                .map_err(|_| unknown())?
                != serde_json::to_value(&replay).map_err(|_| unknown())?
            {
                return Err(unknown());
            }
        }
        if fire {
            let pending = states
                .pending_npc_ajax_effects
                .iter()
                .find(|pending| {
                    pending.actor == actor && pending.session == call.fence.game_session_id
                })
                .cloned()
                .ok_or_else(unknown)?;
            let committed = self
                .root
                .read_npc_ajax_actor_effect(
                    self.character,
                    call.fence,
                    call.request,
                    pending.effect.clone(),
                )
                .await
                .map_err(|_| unknown())?;
            states
                .settle_npc_ajax_effect(&runtime, &pending, committed)
                .map_err(|_| unknown())?;
        }
        result
    }
}
