//! Renewable physical producers retain the ordinary command receipt and mint owners.
use super::{ActorKind, Callback, Trigger};

pub(super) fn admitted(callback: &Callback) -> bool {
    let effects = &callback.effects;
    effects.item_reward_per_use
        && callback.requires_world_admission
        && (match callback.trigger {
            Trigger::Use => callback.selector.native.is_some(),
            Trigger::StepIn => step_reset(callback),
            Trigger::StepOut => false,
        })
        && callback.actor_kind == ActorKind::Player
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

// Crystal 201189f8 data-global/scripts/quests/wrath_of_the_emperor/movements_prison.lua:13-20:
// an exact physical STEP resets one armed release, then returns the stage-specific item.
pub(super) fn step_reset(callback: &Callback) -> bool {
    callback.trigger == Trigger::StepIn
        && callback.effects.item_reward_per_use
        && callback.selector.position.is_some()
        && callback.selector.position_aliases.is_empty()
        && callback.selector.ordinal == Some(0)
        && callback.selector.native.is_none()
        && callback.selector.unique.is_none()
        && callback.selector.action.is_none()
        && callback.selector.appearance.is_none()
        && callback.teleport.is_some()
        && callback.storage.iter().any(|gate| {
            gate.minimum == gate.maximum
                && gate.set != gate.minimum
                && !gate.read_only
                && !gate.preserve
                && !gate.increment
                && !gate.at_least
                && gate.set_if.is_none()
                && gate.add.is_none()
        })
}
