//! Compose live level admission with physically bound, once-only trial writes.
use super::{ActorKind, Callback, Trigger};

pub(super) fn level_guard_admitted(callback: &Callback) -> bool {
    callback.minimum_level.is_some_and(|level| level > 0)
        && callback.requires_world_admission
        && callback.selector.native.is_some()
        && callback.actor_kind == ActorKind::Player
        && callback.trigger == Trigger::Use
        && !callback.reversible
        && !callback.opening_only
        && callback.native_rule.is_none()
        && callback.storage.iter().any(|state| !state.preserve)
}
