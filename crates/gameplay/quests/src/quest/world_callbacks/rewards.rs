//! Bounded opt-in value combinations retain every legacy callback restriction.
use super::{Callback, Trigger};
use std::collections::BTreeSet;

pub(super) fn valid(callback: &Callback) -> bool {
    if !callback.reward_bundle && callback.achievements.is_empty() {
        return true;
    }
    let once = callback
        .storage
        .iter()
        .any(|s| !s.preserve && !s.read_only && !(s.minimum..=s.maximum).contains(&s.set));
    if callback.trigger != Trigger::Use
        || !once
        || callback.reversible
        || callback.native_rule.is_some()
        || callback.monk_reward.is_some()
        || (callback.reward_bundle
            && (!callback.completes
                || callback.item_reward.is_none()
                || callback.effects.outfit_reward.is_none()))
        || callback.achievements.len() > 8
    {
        return false;
    }
    let mut unique = BTreeSet::new();
    callback.achievements.iter().all(|key| {
        key.len() <= 128
            && unique.insert(key)
            && key.strip_prefix("oteryn:achievement/").is_some_and(|tail| {
                !tail.is_empty()
                    && tail.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"_/-".contains(&b)
                    })
            })
    })
}
