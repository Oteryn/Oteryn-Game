//! Sealed exact live creature or self-target admission for a carried quest tool.
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::UseIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[derive(Debug)]
pub struct QuestCreatureUseProof {
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    callback: String,
    key: Option<String>,
    self_target: Option<oteryn_gameplay_quests::quest::reward_callbacks::SelfTarget>,
    target: EntityRef,
}
impl QuestCreatureUseProof {
    pub fn read_current(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterGameplayFence,
        command: CommandRef,
        callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
        intent: &UseIntent,
    ) -> Option<Self> {
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
        let own = runtime.read_actor_position(actor).ok()?.position();
        let at = runtime.read_actor_position(creature).ok()?.position();
        if at.floor != own.floor || at.x.abs_diff(own.x) > 1 || at.y.abs_diff(own.y) > 1 {
            return None;
        }
        Some(Self {
            fence,
            command,
            callback: callback.key.clone(),
            key: Some(key.clone()),
            self_target: None,
            target,
        })
    }
    pub(super) fn permits(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
        callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
        carried: &super::QuestCarriedUse,
    ) -> bool {
        self.fence.item_fence() == fence.item_fence()
            && request.cause == QuestCause::Command(self.command)
            && request.transition_key == self.callback
            && self.key == callback.creature_target
            && self.self_target == callback.self_target
            && oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                &[4, 15],
                &carried.intent,
            )
            .is_ok_and(|intent| {
                intent.use_with == Some(self.target)
                    && intent.use_with_map_item.is_none()
                    && intent.use_with_position.is_none()
            })
    }
}
