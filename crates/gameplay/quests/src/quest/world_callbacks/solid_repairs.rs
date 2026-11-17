//! Opt-in solid remote replacements for bounded native repair transactions.
use super::{ActorKind, Callback, Trigger};

pub(super) fn valid(callback: &Callback) -> bool {
    if callback
        .passages
        .iter()
        .all(|p| p.replacement_passable.is_none())
    {
        return true;
    }
    callback.actor_kind == ActorKind::Player
        && callback.trigger == Trigger::Use
        && callback.selector.native.is_some()
        && callback.requires_world_admission
        && !callback.reversible
        && !callback.opening_only
        && callback.lever.is_none()
        && callback.native_rule.is_none()
        && callback.teleport.is_none()
        && callback.storage.iter().any(|state| !state.preserve)
        && callback.effects.transform.is_some()
        && callback.effects.reset_after_ms.is_none()
        && callback.effects.passages_reset_after_ms.is_none()
        && callback.item_on_target.as_ref().is_some_and(|item| {
            !item.reusable
                && (1..=64).contains(&item.count)
                && item.alternatives.is_empty()
                && item.replacement.is_none()
                && item.target_replacement.is_none()
        })
        && callback.selector.native.as_ref().is_some_and(|source| {
            callback.passages.iter().any(|passage| {
                passage.position == source.position
                    && passage.appearance == source.appearance
                    && passage.replacement == callback.effects.transform
            })
        })
        && callback.passages.iter().all(|passage| {
            passage.replacement_passable == Some(false)
                && passage
                    .replacement
                    .is_some_and(|id| id != 0 && id != passage.appearance)
                && !passage.if_present
                && passage.create_on_close.is_none()
                && callback.world_requirements.iter().any(|required| {
                    required.position == passage.position
                        && required.appearance == passage.appearance
                        && required.ground_stack.is_none()
                })
        })
}
