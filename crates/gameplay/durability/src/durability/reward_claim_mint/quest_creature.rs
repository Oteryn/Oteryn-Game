//! Sealed exact live-creature admission for a carried quest tool.
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
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
        let creature = runtime
            .visible_entities()
            .creatures
            .iter()
            .find(|c| {
                c.actor.placement_identity() == target.identity && c.generation == target.generation
            })?
            .actor;
        runtime.check_mount_taming_subject(creature, key).ok()?;
        let own = runtime.read_actor_position(actor).ok()?.position();
        let at = runtime.read_actor_position(creature).ok()?.position();
        if at.floor != own.floor || at.x.abs_diff(own.x) > 1 || at.y.abs_diff(own.y) > 1 {
            return None;
        }
        Some(Self {
            fence,
            command,
            callback: callback.key.clone(),
            key: key.clone(),
            intent: intent.clone(),
            corpse: None,
            _runtime: std::marker::PhantomData,
        })
    }
    pub(super) fn permits(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        key: &str,
        carried: &super::QuestCarriedUse,
    ) -> bool {
        self.corpse.is_none()
            && self.fence.item_fence() == fence.item_fence()
            && request.cause == QuestCause::Command(self.command)
            && request.transition_key == self.callback
            && self.key == key
            && oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                &[4, 15],
                &carried.intent,
            )
            .is_ok_and(|intent| {
                intent == self.intent
                    && intent.use_with_map_item.is_none()
                    && intent.use_with_position.is_none()
            })
    }
}
