//! Existing condition owner retains one item command until its durable outcome is known.
//! Commit publishes the retained cues once through the live presentation journal.
use super::*;
use crate::durability::consume_transform::{ConsumeTransformRequest, ConsumeTransformResult};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::novelty_item_use::PendingItemEffect;
use oteryn_gameplay_items::novelty_items::NoveltyEffects;
use oteryn_protocol_oteryn::world_object::UseDisposition;

/// Inputs of one retained effect successor, bound to its original item command.
#[derive(Debug)]
pub(in crate::gameplay_transport) struct NoveltyEffectRequest<'a> {
    pub actor: ExactActorRef,
    pub fence: CurrentCharacterItemFence,
    pub request: &'a ConsumeTransformRequest,
    pub effects: NoveltyEffects,
    pub target: Option<ExactActorRef>,
    pub now_us: u64,
}

impl ChannelSpellStates {
    /// Crystal 201189f `data-global/scripts/actions/other/fish_tank.lua:3-5` and
    /// `data-global/scripts/actions/other/music.lua:65-66`: publish the committed item's cues.
    pub(in crate::gameplay_transport) fn publish_house_decoration_visuals(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        position: MovementLocalPosition,
        effect_ids: &[u16],
    ) {
        for effect in effect_ids {
            self.sounds
                .record_house_item_effect(runtime, actor, session, position, *effect);
        }
    }

    pub(in crate::gameplay_transport) fn publish_novelty_visuals(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        position: MovementLocalPosition,
        effects: &NoveltyEffects,
    ) {
        let house_session = runtime
            .house_observer(actor)
            .ok()
            .flatten()
            .map(|(session, _)| session);
        for cue in &effects.cues {
            if let Some(session) = house_session
                && !effects.require_player_target
            {
                self.publish_house_decoration_visuals(
                    runtime,
                    actor,
                    session,
                    position,
                    std::slice::from_ref(&cue.effect_id),
                );
            } else {
                self.sounds.record_item_effect(
                    runtime,
                    actor,
                    position,
                    cue.effect_id,
                    effects.require_player_target,
                );
            }
        }
    }
    pub(in crate::gameplay_transport) fn prepare_novelty_effect(
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
        let session = request.command.game_session_id();
        if fence.game_session_id != session {
            return Err(UseDisposition::Rejected);
        }
        if let Some(existing) = self
            .pending_item_effects
            .iter()
            .find(|memo| memo.actor == actor && memo.request.command == request.command)
        {
            return if existing.fence == fence
                && existing.request.source == request.source
                && existing.request.target == request.target
                && existing.request.intent == request.intent
            {
                Ok(())
            } else {
                Err(UseDisposition::Rejected)
            };
        }
        if self.is_dead(actor) || !self.spell_idle(runtime, actor, session) {
            return Err(UseDisposition::Rejected);
        }
        let current = self
            .get(runtime, actor, session)
            .ok_or(UseDisposition::StaleState)?;
        let before_revision = current.revision();
        let after = if let Some(costume) = &effects.costume {
            Some(
                crate::spell::actor_conditions::stage_item_costume(
                    current,
                    costume.look_type,
                    costume.duration_ms,
                    now_us / 1000,
                )
                .map_err(|_| UseDisposition::NotPossible)?,
            )
        } else if let Some(damage) = effects.damage {
            Some(
                crate::spell::actor_conditions::stage_item_health_loss(current, damage, now_us)
                    .map_err(|_| UseDisposition::NotPossible)?
                    .0,
            )
        } else {
            None
        };
        let death = if after.as_ref().is_some_and(|after| after.health == 0) {
            self.deaths
                .try_reserve(1)
                .map_err(|_| UseDisposition::NotPossible)?;
            Some(PlayerDeath {
                public_cause: None,
                occurrence: (self.mint_death)().ok_or(UseDisposition::NotPossible)?,
                cell: runtime
                    .read_actor_position(actor)
                    .map_err(|_| UseDisposition::StaleState)?
                    .position(),
            })
        } else {
            None
        };
        self.pending_item_effects
            .try_reserve(1)
            .map_err(|_| UseDisposition::NotPossible)?;
        // Capture the donor source/target position before the durable await. A reconnect
        // or a moved target cannot relocate a committed visual to a different tile.
        let cue_position = if let Some(ground) = &effects.ground {
            MovementLocalPosition {
                x: ground.x,
                y: ground.y,
                floor: ground.floor,
            }
        } else {
            runtime
                .read_actor_position(target.unwrap_or(actor))
                .map_err(|_| UseDisposition::StaleState)?
                .position()
        };
        if native
            .item_policy_for_source_id(effects.source_item_id)
            .is_none_or(|policy| !policy.record().admission.materializable)
            || (effects.require_player_target && target.is_none())
        {
            return Err(UseDisposition::NotPossible);
        }
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

    pub(in crate::gameplay_transport) fn release_novelty_effect(
        &mut self,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
    ) {
        if let Some(index) = self
            .pending_item_effects
            .iter()
            .position(|memo| memo.actor == actor && memo.request.command == request.command)
        {
            self.pending_item_effects.remove(index);
        }
    }

    pub(in crate::gameplay_transport) fn settle_novelty_effect(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
        receipt: &ConsumeTransformResult,
    ) -> Result<Option<(NoveltyEffects, MovementLocalPosition)>, ()> {
        let Some(index) = self
            .pending_item_effects
            .iter()
            .position(|memo| memo.actor == actor && memo.request.command == request.command)
        else {
            // A classified replay with no original pending owner work emits nothing.
            return Ok(None);
        };
        if receipt.disposition != UseDisposition::Committed {
            self.release_novelty_effect(actor, request);
            return Ok(None);
        }
        let memo = &self.pending_item_effects[index];
        // Recheck the retained successor's exact lease under this uninterrupted
        // runtime -> states turn, including a normal finish after its durable await.
        if runtime.owner_fence().is_err()
            || runtime.binding().runtime_scope() != memo.fence.runtime_scope
            || runtime.binding().scope_generation() != memo.fence.scope_ownership_generation
            || !runtime
                .bound_attacker_lease(actor, request.command.game_session_id())
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == memo.fence.character_id.as_bytes()
                        && lease.generation() == memo.fence.character_lease_generation
                })
        {
            return Err(());
        }
        if memo.request.source != request.source
            || memo.request.target != request.target
            || memo.request.intent != request.intent
            || self
                .get(runtime, actor, request.command.game_session_id())
                .is_none_or(|current| current.revision() != memo.before_revision)
            || self.is_dead(actor)
        {
            return Err(());
        }
        if memo.effects.kind == oteryn_gameplay_items::novelty_items::NoveltyKind::Cosmetic
            && receipt.effects["kind"] == "cosmetic"
            && receipt.effects["source_item_id"] == memo.effects.source_item_id
            && receipt.effects["cosmetic"]["consumed"] == false
        {
            self.release_novelty_effect(actor, request);
            return Ok(None);
        }
        let expected = serde_json::to_value(&memo.effects).map_err(|_| ())?;
        if !effect_receipt_matches(&expected, &receipt.effects) {
            return Err(());
        }
        let memo = self.pending_item_effects.remove(index);
        // All fallible admission and memory reservation preceded the durable item mutation.
        // These installs run under one uninterrupted runtime -> spell-state owner turn.
        if let Some(after) = memo.after {
            assert!(self.commit(runtime, actor, request.command.game_session_id(), after));
        }
        if let Some(death) = memo.death {
            self.deaths
                .push((actor, request.command.game_session_id(), death));
        }
        self.owner_wake.notify_one();
        Ok(Some((memo.effects, memo.cue_position)))
    }
}

// Cosmetic acquisition adds its immutable grant result to the shared effects.
fn effect_receipt_matches(prepared: &serde_json::Value, receipt: &serde_json::Value) -> bool {
    if prepared["kind"] != "cosmetic" {
        return prepared == receipt;
    }
    let Some(mut receipt) = receipt.as_object().cloned() else {
        return false;
    };
    if receipt.remove("cosmetic").is_none() {
        return false;
    }
    if receipt
        .get("speech")
        .is_some_and(serde_json::Value::is_null)
    {
        receipt.remove("speech");
    }
    prepared == &serde_json::Value::Object(receipt)
}
