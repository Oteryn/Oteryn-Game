//! A paid, cooldown-guarded timestamp is a repeatable entitlement rather than an item MINT.
use super::{RewardCallback, StateCondition, StateMutation};

pub(super) fn guarded(callback: &RewardCallback) -> bool {
    let [stamp] = callback.state_ops.as_slice() else {
        return false;
    };
    callback.fee.is_some()
        && !callback.completes
        && callback.storage.is_empty()
        && callback.take.is_empty()
        && callback.present.is_empty()
        && callback.give.is_none()
        && callback.outfit.is_none()
        && callback.outfits.is_empty()
        && callback.achievement.is_none()
        && callback.achievement_progress.is_empty()
        && callback.participants.is_empty()
        && callback.map_mark.is_none()
        && callback.carried_item.is_none()
        && callback.carried_target.is_none()
        && callback.peer_requires.is_none()
        && callback.creature_target.is_none()
        && stamp.valid()
        && matches!(&stamp.from, StateCondition::ElapsedAtLeast { seconds } if *seconds > 0)
        && matches!(&stamp.effect, StateMutation::SetNow)
}
