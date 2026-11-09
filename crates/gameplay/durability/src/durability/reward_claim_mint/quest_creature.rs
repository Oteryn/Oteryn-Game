//! Sealed exact live creature or self-target admission for a carried quest tool.
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[derive(Clone, Copy, Debug)]
enum Target {
    Live {
        entity: EntityRef,
        actor: ExactActorRef,
    },
    Corpse {
        item: [u8; 16],
        position: [i32; 3],
    },
}

/// A live-target proof cannot escape the runtime owner turn that observed it.
///
/// ```compile_fail
/// use oteryn_gameplay_durability::durability::{
///     reward_claim_mint::quest_creature::QuestCreatureUseProof,
///     character_progression::CurrentCharacterGameplayFence,
/// };
/// use oteryn_gameplay_entities::{ChannelRuntimeV1, ExactActorRef, CommandRef};
/// use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
/// use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent};
/// fn escape(runtime: ChannelRuntimeV1, actor: ExactActorRef,
///     fence: CurrentCharacterGameplayFence, command: CommandRef,
///     callback: &RewardCallback, intent: &UseIntent) -> impl std::fmt::Debug + 'static {
///     QuestCreatureUseProof::read_current(&runtime, actor, fence, command, callback, intent)
///         .expect("current target")
/// }
/// ```
#[derive(Debug)]
pub struct QuestCreatureUseProof<'runtime> {
    _runtime: std::marker::PhantomData<&'runtime ChannelRuntimeV1>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    callback: String,
    key: Option<String>,
    self_target: Option<oteryn_gameplay_quests::quest::reward_callbacks::SelfTarget>,
    target: EntityRef,
    key: String,
    target: Target,
use oteryn_protocol_oteryn::world_object::UseIntent;

mod corpse;
pub use corpse::QuestCorpseOperand;

/// A live target proof binds the complete decoded tool intent and borrows its
/// runtime owner until the quest commit ends.
/// Mutating or releasing that owner while retaining the proof is forbidden.
///
/// ```compile_fail
/// use oteryn_gameplay_durability::durability::reward_claim_mint::quest_creature::QuestCreatureUseProof;
/// let _requires_owner = |runtime: &mut _, actor, fence, command, callback, intent| {
///     let proof = QuestCreatureUseProof::read_current(runtime, actor, fence, command, callback, intent);
///     fn change_runtime<T>(_: &mut T) {}
///     change_runtime(runtime);
///     drop(proof);
/// };
/// ```
#[derive(Debug)]
pub struct QuestCreatureUseProof<'runtime> {
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    callback: String,
    key: String,
    intent: UseIntent,
    corpse: Option<corpse::CorpseUse<'runtime>>,
    _runtime: std::marker::PhantomData<&'runtime ()>,
}
impl<'runtime> QuestCreatureUseProof<'runtime> {
    pub fn read_current(
        runtime: &'runtime ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
        intent: &UseIntent,
    ) -> Option<Self> {
        let key = callback.creature_target.as_ref();
        if callback.corpse_target.is_some() {
        if callback.fresh_corpse {
            return None;
        }
        let key = callback.creature_target.as_ref()?;
        let target = intent.use_with?;
        let lease = runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .ok()?;
        if actor.runtime_scope() != fence.runtime_scope
            || lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || command.game_session_id() != fence.game_session_id
        {
            return None;
        }
        let own = runtime.read_actor_position(actor).ok()?.position();
        if let Some(area) = &callback.self_target {
            let visible = runtime.visible_entities();
            let player = visible.players.iter().find(|p| p.actor == actor)?;
            if player.actor.placement_identity() != target.identity
                || player.generation != target.generation
                || !area.contains(own.x, own.y, own.floor)
                || key.is_some()
            {
                return None;
            }
            return Some(Self {
                fence, command, callback: callback.key.clone(), key: None,
                self_target: Some(area.clone()), target,
            });
        }
        let key = key?;
        let own = runtime.read_actor_position(actor).ok()?.position();
        if let Some(area) = &callback.self_target {
            let visible = runtime.visible_entities();
            let player = visible.players.iter().find(|p| p.actor == actor)?;
            if player.actor.placement_identity() != target.identity
                || player.generation != target.generation
                || !area.contains(own.x, own.y, own.floor)
                || key.is_some()
            {
                return None;
            }
            return Some(Self {
                fence,
                command,
                callback: callback.key.clone(),
                key: None,
                self_target: Some(area.clone()),
                target,
            });
        }
        let key = key?;
        let creature = runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|c| {
                c.actor.placement_identity() == target.identity && c.generation == target.generation
            })?
            .actor;
        runtime.check_mount_taming_subject(creature, key).ok()?;
        let at = runtime.read_actor_position(creature).ok()?.position();
        if at.floor != own.floor || at.x.abs_diff(own.x) > 1 || at.y.abs_diff(own.y) > 1 {
            return None;
        }
        Some(Self {
            _runtime: std::marker::PhantomData,
            fence,
            command,
            callback: callback.key.clone(),
            key: Some(key.clone()),
            self_target: None,
            target,
            key: key.clone(),
            target: Target::Live {
                entity: target,
                actor: creature,
            },
            key: key.clone(),
            intent: intent.clone(),
            corpse: None,
            _runtime: std::marker::PhantomData,
        })
    }
    /// Exact handle observed in the retained runtime turn; used after a committed capture.
    pub fn target_actor(&self) -> Option<ExactActorRef> {
        match self.target {
            Target::Live { actor, .. } => Some(actor),
            Target::Corpse { .. } => None,
        }
    }

    pub(super) fn corpse(&self) -> Option<([u8; 16], [i32; 3])> {
        match self.target {
            Target::Corpse { item, position } => Some((item, position)),
            Target::Live { .. } => None,
        }
    }

    /// Issued by the selected-ground owner after resolving its Item handle.
    /// Position is read from the leased actor under this same runtime turn.
    pub fn read_current_corpse(
        runtime: &'runtime ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
        intent: &UseIntent,
        corpse: [u8; 16],
    ) -> Option<Self> {
        let at = runtime.read_actor_position(actor).ok()?.position();
        let position = [at.x, at.y, i32::from(at.floor)];
        callback.corpse_target.as_ref()?;
        let key = callback.creature_target.as_ref()?;
        let lease = runtime
            .bound_attacker_lease(actor, fence.game_session_id)
            .ok()?;
        if callback.capture_creature
            || corpse == [0; 16]
            || actor.runtime_scope() != fence.runtime_scope
            || lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || command.game_session_id() != fence.game_session_id
            || intent.use_with.is_some()
            || intent.use_with_position.is_some()
            || !matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
        {
            return None;
        }
        Some(Self {
            _runtime: std::marker::PhantomData,
            fence,
            command,
            callback: callback.key.clone(),
            key: key.clone(),
            target: Target::Corpse {
                item: corpse,
                position,
            },
        })
    }

    pub(super) async fn record_capture(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        carried: &super::QuestCarriedUse,
    ) -> Result<bool, crate::durability::DurabilityError> {
        let Target::Live { entity: target, .. } = self.target else {
            return Ok(false);
        };
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = self.fence.runtime_scope
        else {
            return Ok(false);
        };
        Ok(sqlx::query("INSERT INTO game_quest_creature_captures(character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,scope_generation,target_identity,target_generation,creature_key,source_item,ingress) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8,$9::text::numeric,$10,encode($11,'hex')::uuid,$12) ON CONFLICT DO NOTHING")
            .bind(self.fence.character_id.as_bytes().as_slice())
            .bind(self.command.game_session_id().as_bytes().as_slice())
            .bind(self.command.command_id().get().to_string()).bind(&self.callback)
            .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
            .bind(self.fence.scope_ownership_generation.get().to_string())
            .bind(target.identity.as_slice()).bind(target.generation.to_string())
            .bind(&self.key).bind(carried.source.as_slice()).bind(&carried.intent)
            .execute(&mut **tx).await?.rows_affected() == 1)
    }

    pub(super) fn permits(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
        carried: &super::QuestCarriedUse,
    ) -> bool {
        self.corpse.is_none()
            && self.fence.item_fence() == fence.item_fence()
            && request.cause == QuestCause::Command(self.command)
            && request.transition_key == self.callback
            && self.key == callback.creature_target
            && self.self_target == callback.self_target
            && oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                &[
                    oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
                    oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
                    oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
                ],
                &carried.intent,
            )
            .is_ok_and(|intent| {
                intent.use_with_position.is_none()
                    && match self.target {
                        Target::Live { entity, .. } => {
                            intent.use_with == Some(entity) && intent.use_with_map_item.is_none()
                        }
                        Target::Corpse { .. } => {
                            intent.use_with.is_none()
                                && matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
                        }
                    }
                intent == self.intent
                    && intent.use_with_map_item.is_none()
                    && intent.use_with_position.is_none()
            })
    }
}
