//! STEP alternatives share geometry only when preserved personal phases are disjoint.
use super::{ActorKind, Callback, Trigger};
use std::collections::BTreeMap;

pub fn exclusive(left: &Callback, right: &Callback) -> bool {
    let source_trial = left.trigger == Trigger::Use
        && left.effects.target_transform.is_some()
        && left.effects.target_transform == right.effects.target_transform
        && left.item_on_target == right.item_on_target;
    left.actor_kind == ActorKind::Player
        && right.actor_kind == ActorKind::Player
        && (source_trial || matches!(left.trigger, Trigger::StepIn | Trigger::StepOut))
        && left.trigger == right.trigger
        && left.selector == right.selector
        && left.quest == right.quest
        && left.storage.iter().any(|a| {
            (a.preserve || source_trial)
                && right.storage.iter().any(|b| {
                    (b.preserve || source_trial)
                        && a.track == b.track
                        && (a.maximum < b.minimum || b.maximum < a.minimum)
                })
        })
}

pub fn matches(callback: &Callback, tracks: &BTreeMap<String, i64>) -> bool {
    callback
        .storage
        .iter()
        .filter(|guard| guard.preserve)
        .all(|guard| {
            tracks
                .get(&guard.track)
                .is_some_and(|value| (guard.minimum..=guard.maximum).contains(value))
        })
}

/// Retained chosen-programme witness produced by questpc7's authoring tool at2caf6670.
/// This is a historical source snapshot, not a claim about the moving definition shard.
pub(super) fn authored_source(callback: &Callback, source: &str) -> bool {
    callback.quest == "oteryn:quest.feaster_of_souls_quest"
        && matches!(
            callback.key.as_str(),
            "oteryn:quest-callback/questpc7-feaster-entry"
                | "oteryn:quest-callback/questpc7-feaster-purification-entry"
        )
        && source
            == "canonical:content/quests/definitions/quests-00100-00199.json:1#/records/41/definition:sha256:57af470e521006a832fbc046639ee4a5a5a455b2c6552335afff0a0131783f2c"
}
