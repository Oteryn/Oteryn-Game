//! Carried branches may share a tile only when their physical or state gates disjoin.
use super::Callback;

pub(super) fn disjoint(left: &Callback, right: &Callback) -> bool {
    if left.carried_use.is_none() || right.carried_use.is_none() {
        return false;
    }
    let (Some(left_item), Some(right_item)) = (&left.item_on_target, &right.item_on_target) else {
        return false;
    };
    let left_items = std::iter::once(&left_item.item).chain(&left_item.alternatives);
    let right_items = std::iter::once(&right_item.item).chain(&right_item.alternatives);
    if left_items
        .clone()
        .all(|item| !right_items.clone().any(|other| item == other))
    {
        return true;
    }
    left.storage.iter().any(|a| {
        right
            .storage
            .iter()
            .any(|b| a.track == b.track && (a.maximum < b.minimum || b.maximum < a.minimum))
    })
}
