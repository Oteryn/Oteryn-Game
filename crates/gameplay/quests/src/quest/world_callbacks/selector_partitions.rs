//! Reused objects may select mutually exclusive phases of one quest.
use super::Callback;

pub(super) fn disjoint(left: &Callback, right: &Callback) -> bool {
    left.quest == right.quest
        && left.actor_kind == right.actor_kind
        && left.storage.iter().any(|left| {
            !left.at_least
                && left.minimum <= left.maximum
                && right.storage.iter().any(|right| {
                    left.track == right.track
                        && !right.at_least
                        && right.minimum <= right.maximum
                        && (left.maximum < right.minimum || right.maximum < left.minimum)
                })
        })
}
