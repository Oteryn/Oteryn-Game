//! Shared current actor/session fence for closed native Item leaves.
use super::*;
use crate::durability::consume_transform::{ConsumeTransformRequest, ConsumeTransformResult};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::novelty_item_use::PendingItemEffect;
use oteryn_gameplay_items::novelty_items::NoveltyEffects;
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) fn scoped_item_position(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
) -> Result<MovementLocalPosition, UseDisposition> {
    Ok(match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { .. } => runtime
            .read_actor_position(actor)
            .map_err(|_| UseDisposition::StaleState)?
            .position(),
        RuntimeScopeRefV1::Instance { .. } => {
            let at = runtime
                .house_actor_state(actor, fence.game_session_id)
                .map_err(|_| UseDisposition::StaleState)?
                .position;
            if !(0..=15).contains(&at.floor) {
                return Err(UseDisposition::StaleState);
            }
            MovementLocalPosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            }
        }
    })
}

pub(super) fn scoped_item_current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
    allow_loss: bool,
) -> bool {
    let facts = match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { .. } => {
            if runtime.owner_fence().is_err() {
                return false;
            }
            if runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || !runtime
                    .bound_attacker_lease(actor, fence.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return false;
            }
            runtime.player_control_facts(actor, fence.game_session_id)
        }
        RuntimeScopeRefV1::Instance { .. } => {
            if !runtime
                .house_actor_state(actor, fence.game_session_id)
                .is_ok_and(|house| {
                    house.scope == fence.runtime_scope
                        && house.generation == fence.scope_ownership_generation
                        && house.lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && house.lease.generation() == fence.character_lease_generation
                })
            {
                return false;
            }
            runtime.house_player_control_facts(actor, fence.game_session_id)
        }
    };
    facts.is_ok_and(|facts| allow_loss || facts.control_loss.is_none())
}

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn prepare_scoped_gift_effect(
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
            || !matches!(effects.source_item_id, 2995 | 5080 | 5791 | 6511 | 6566)
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
            || !scoped_item_current(runtime, actor, fence, false)
            || self.has_pending_spell_commit(actor, fence.game_session_id)
            || matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
                && runtime.assert_actor_spell_unreserved(actor).is_err()
            || native
                .item_policy_for_source_id(effects.source_item_id)
                .is_none_or(|p| !p.record().admission.materializable)
        {
            return Err(UseDisposition::Rejected);
        }
        let state = self
            .actors
            .iter()
            .find(|(a, s, _)| *a == actor && *s == fence.game_session_id)
            .map(|(_, _, s)| s)
            .ok_or(UseDisposition::StaleState)?;
        let before_revision = state.revision();
        let after = effects
            .damage
            .map(|amount| {
                crate::spell::actor_conditions::stage_item_health_loss(state, amount, now_us)
                    .map(|v| v.0)
            })
            .transpose()
            .map_err(|_| UseDisposition::NotPossible)?;
        let death = if after.as_ref().is_some_and(|after| after.health == 0) {
            self.deaths
                .try_reserve(1)
                .map_err(|_| UseDisposition::NotPossible)?;
            let cell = scoped_item_position(runtime, actor, fence)?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)().ok_or(UseDisposition::NotPossible)?,
                cell,
            })
        } else {
            None
        };
        self.pending_item_effects
            .try_reserve(1)
            .map_err(|_| UseDisposition::NotPossible)?;
        let cue_position = if let Some(ground) = &effects.ground {
            MovementLocalPosition {
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
            after,
            death,
        });
        Ok(())
    }
    pub(in crate::gameplay_transport) fn settle_scoped_gift_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
        receipt: &ConsumeTransformResult,
    ) -> Result<Option<NoveltyEffects>, ()> {
        let Some(index) = self.pending_item_effects.iter().position(|m| {
            m.actor == actor
                && m.request.command == request.command
                && matches!(m.effects.source_item_id, 2995 | 5080 | 5791 | 6511 | 6566)
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
        let mut retained = receipt.effects.clone();
        let object = retained.as_object_mut().ok_or(())?;
        if object.remove("gift_node").is_none() {
            return Err(());
        }
        object.remove("gift_after");
        object.remove("gift_currency");
        if self.is_dead(actor)
            || !scoped_item_current(runtime, actor, memo.fence, true)
            || memo.request.source != request.source
            || memo.request.target != request.target
            || memo.request.intent != request.intent
            || state.revision() != memo.before_revision
            || memo
                .after
                .as_ref()
                .is_some_and(|after| state.revision().checked_add(1) != Some(after.revision()))
            || serde_json::to_value(&memo.effects).map_err(|_| ())? != retained
        {
            return Err(());
        }
        let memo = self.pending_item_effects.remove(index);
        if let Some(after) = memo.after {
            self.actors[state_index].2 = after;
        }
        if let Some(death) = memo.death {
            self.deaths
                .push((actor, request.command.game_session_id(), death));
        }
        self.owner_wake.notify_one();
        Ok(Some(memo.effects))
    }
}
