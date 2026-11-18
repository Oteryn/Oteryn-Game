//! Only statically disjoint, fenced USE routes may share an authored source.
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

/// An achievement station can use disjoint durable progress bands to select
/// its award bundle. This is selection advice; the writer still checks the
/// selected callback's complete storage predicates under the Character fence.
pub fn disjoint_achievement_routes(left: &Callback, right: &Callback) -> bool {
    let fenced = |callback: &Callback| {
        callback.actor_kind == ActorKind::Player
            && callback.trigger == Trigger::Use
            && callback.requires_world_admission
            && !callback.achievements.is_empty()
            && callback.native_rule.is_none()
            && callback.lever.is_none()
            && callback.item_on_target.is_none()
            && callback.effects.target_transform.is_none()
    };
    left.selector == right.selector
        && fenced(left)
        && fenced(right)
        && super::selector_partitions::disjoint(left, right)
}
