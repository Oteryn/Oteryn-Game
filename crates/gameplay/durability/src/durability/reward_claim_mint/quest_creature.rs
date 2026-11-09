//! Sealed exact live creature or self-target admission for a carried quest tool.
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

mod capture;
#[derive(Clone, Copy, Debug)]
enum Target {
    FreshCorpse,
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
    _runtime: std::marker::PhantomData<&'runtime ChannelRuntimeV1>,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    callback: String,
    key: Option<String>,
    self_target: Option<oteryn_gameplay_quests::quest::reward_callbacks::SelfTarget>,
    target: Target,
    intent: UseIntent,
    corpse: Option<corpse::CorpseUse<'runtime>>,
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
        if callback.corpse_target.is_some() || callback.fresh_corpse {
            return None;
        }
        let key = callback.creature_target.as_ref();
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
                _runtime: std::marker::PhantomData,
                fence, command, callback: callback.key.clone(), key: None,
                self_target: Some(area.clone()), target: Target::Live { entity: target, actor },
                intent: intent.clone(), corpse: None,
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
            intent: intent.clone(), corpse: None,
            target: Target::Live {
                entity: target,
                actor: creature,
            },
        })
    }
    /// Exact handle observed in the retained runtime turn; used after a committed capture.
    pub fn target_actor(&self) -> Option<ExactActorRef> {
        match self.target {
            Target::Live { actor, .. } => Some(actor),
            Target::Corpse { .. } | Target::FreshCorpse => None,
        }
    }

    pub(super) fn corpse(&self) -> Option<([u8; 16], [i32; 3])> {
        match self.target {
            Target::Corpse { item, position } => Some((item, position)),
            Target::Live { .. } | Target::FreshCorpse => None,
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
            key: Some(key.clone()), self_target: None,
            intent: intent.clone(), corpse: None,
            target: Target::Corpse {
                item: corpse,
                position,
            },
        })
    }
    /// Exact server actor checked by the live runtime owner. Capture retirement
    /// can retain this identity instead of reconstructing it from a client handle.
    /// The proof still borrows that runtime; this accessor grants no mutation.
    pub fn live_target(&self) -> Option<ExactActorRef> {
        self.target_actor()
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
                intent == self.intent &&
                intent.use_with_position.is_none()
                    && match self.target {
                        Target::FreshCorpse => false,
                        Target::Live { entity, .. } => {
                            intent.use_with == Some(entity) && intent.use_with_map_item.is_none()
                        }
                        Target::Corpse { .. } => {
                            intent.use_with.is_none()
                                && matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
                        }
                    }
            })
    }
}
