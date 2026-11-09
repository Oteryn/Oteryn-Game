//! Renewable physical producers retain the ordinary command receipt and mint owners.
use super::{ActorKind, Callback, Trigger};

pub(super) fn admitted(callback: &Callback) -> bool {
    let effects = &callback.effects;
    callback.requires_world_admission
        && callback.selector.native.is_some()
        && callback.actor_kind == ActorKind::Player
        && callback.trigger == Trigger::Use
        && !callback.reversible
        && !callback.opening_only
        && callback.native_rule.is_none()
        && callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && effects.item_reward.is_some()
        && effects.item_reward_cycle.is_none()
        && effects.quest_completion.is_none()
        && !effects.completes_quest
        && effects.mount_reward.is_none()
        && effects.outfit_reward.is_none()
        && effects.encounter.is_none()
}
