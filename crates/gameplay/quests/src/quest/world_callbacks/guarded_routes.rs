//! Only statically disjoint, fenced tool routes may share an authored source.
use super::{ActorKind, Callback, Trigger};

pub(super) fn disjoint(a: &Callback, b: &Callback) -> bool {
    let fenced = |c: &Callback| {
        c.actor_kind == ActorKind::Player
            && c.trigger == Trigger::Use
            && c.requires_world_admission
            && c.native_rule.is_none()
            && c.lever.is_none()
            && c.item_on_target
                .as_ref()
                .is_some_and(|i| i.selected_instance)
    };
    fenced(a)
        && fenced(b)
        && a.storage.iter().any(|left| {
            b.storage.iter().any(|right| {
                left.track == right.track
                    && (left.maximum < right.minimum || right.maximum < left.minimum)
            })
        })
}
