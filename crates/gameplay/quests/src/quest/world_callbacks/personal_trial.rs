//! A bounded retryable personal trial, admitted by a captured native STEP receipt.
use super::{ActorKind, Callback, Trigger};

/// Personal trial entry may schedule monsters without turning a shared lever.
/// It cannot deliver value or combine another map operation with that entry.
pub fn admitted(callback: &Callback) -> bool {
    callback.trigger == Trigger::StepIn
        && callback.actor_kind == ActorKind::Player
        && callback.requires_world_admission
        && callback.has_personal_state()
        && callback.lever.is_none()
        && !callback.reversible
        && callback
            .effects
            .encounter
            .as_ref()
            .is_some_and(|e| e.timing.is_some())
        && !callback.effects.completes_quest
        && callback.effects.quest_completion.is_none()
        && callback.effects.portals.is_none()
        && callback.effects.item_cycle.is_none()
        && callback.effects.transform.is_none()
        && callback.effects.item_reward.is_none()
        && callback.effects.outfit_reward.is_none()
        && callback.effects.mount_reward.is_none()
        && callback.passages.is_empty()
        && callback.teleport.is_none()
}
