//! M33: one physical STEP selector may serve provably disjoint read-only phases.
//! Feaster chosen recipe s2/s6; retained questpc7 route at 2caf66704f73,
//! content/quests/questpc7-world-callbacks.json, programme 0 versus 5.
use super::{ActorKind, Callback, Trigger};
use crate::quest::QuestStateCatalogue;
use std::collections::BTreeMap;

pub(super) fn disjoint(a: &Callback, b: &Callback) -> bool {
    a.quest == b.quest
        && a.selector == b.selector
        && a.trigger == b.trigger
        && matches!(a.trigger, Trigger::StepIn | Trigger::StepOut)
        && a.actor_kind == ActorKind::Player
        && b.actor_kind == ActorKind::Player
        && a.storage.iter().any(|left| {
            left.preserve
                && b.storage.iter().any(|right| {
                    right.preserve
                        && left.track == right.track
                        && (left.maximum < right.minimum || right.maximum < left.minimum)
                })
        })
}

/// Select by current Character state, never registry order. The ordinary fenced
/// commit rechecks every guard, so a changed phase cannot execute the old action.
pub fn select_storage_phase<'a>(
    candidates: &[&'a Callback],
    values: &BTreeMap<String, i64>,
    catalogue: &QuestStateCatalogue,
) -> Result<Option<&'a Callback>, String> {
    if candidates.is_empty() || candidates.len() > super::MAX_EFFECTS {
        return Err("phase candidate bound".into());
    }
    for (index, left) in candidates.iter().enumerate() {
        if candidates[index + 1..]
            .iter()
            .any(|right| !disjoint(left, right))
        {
            return Err("overlapping physical callback phases".into());
        }
    }
    let mut selected = None;
    for callback in candidates {
        let eligible = callback.storage.iter().all(|guard| {
            values
                .get(&guard.track)
                .copied()
                .or_else(|| catalogue.track(&guard.track).map(|track| track.initial))
                .is_some_and(|value| (guard.minimum..=guard.maximum).contains(&value))
        });
        if eligible {
            if selected.is_some() {
                return Err("ambiguous current callback phase".into());
            }
            selected = Some(*callback);
        }
    }
    Ok(selected)
}
// STEP alternatives share geometry only when preserved personal phases are disjoint.

pub fn exclusive(left: &Callback, right: &Callback) -> bool {
    left.actor_kind == ActorKind::Player
        && right.actor_kind == ActorKind::Player
        && matches!(left.trigger, Trigger::StepIn | Trigger::StepOut)
        && left.trigger == right.trigger
        && left.selector == right.selector
        && left.quest == right.quest
        && left.storage.iter().any(|a| {
            a.preserve
                && right.storage.iter().any(|b| {
                    b.preserve
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
