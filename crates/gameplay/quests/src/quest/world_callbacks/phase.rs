//! M33: one physical STEP selector may serve provably disjoint read-only phases.
//! Feaster chosen recipe s2/s6; retained questpc7 route at 2caf66704f73,
//! content/quests/questpc7-world-callbacks.json, programme 0 versus 5.
use super::{ActorKind, Callback, Storage, Trigger};
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
            (left.preserve || left.read_only)
                && b.storage.iter().any(|right| {
                    (right.preserve || right.read_only)
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
                .or_else(|| values.get(legacy_read_track(guard)?))
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
pub const SEA_EAR_TRACK: &str = "oteryn:quest-progress/qc/twenty_miles/ear_found";
pub const SEA_EAR_ITEM: &str = "oteryn:item.tibia.i21754";

/// Whether this stage-19–21 route requires the Sea Ear in current custody.
pub fn sea_ear_route(callback: &Callback) -> Option<bool> {
    match callback.key.as_str() {
        "oteryn:quest-callback/qc-twenty-miles-maelstrom-to-shells" => Some(false),
        "oteryn:quest-callback/qc-twenty-miles-maelstrom-to-grounds-with-ear" => Some(true),
        _ => None,
    }
}

/// A paid hoard and its receipt-backed return share the exact physical source.
pub fn hoard_reentry_pair(left: &Callback, right: &Callback) -> bool {
    let (paid, reentry) = if left.is_read_only() {
        (right, left)
    } else {
        (left, right)
    };
    paid.quest == "oteryn:quest.twenty_miles_beneath_the_sea_quest"
        && paid.quest == reentry.quest
        && paid.selector == reentry.selector
        && paid.trigger == Trigger::Use
        && reentry.trigger == Trigger::Use
        && paid.teleport.is_some()
        && paid.teleport == reentry.teleport
        && paid.requires_world_admission
        && paid.item_on_target.as_ref().is_some_and(|item| {
            item.selected_instance && !item.reusable && item.item == "oteryn:item.tibia.i21746"
        })
        && reentry.is_read_only()
        && reentry.item_on_target.is_none()
        && reentry.key.strip_suffix("-reentry") == Some(paid.key.as_str())
        && paid
            .key
            .starts_with("oteryn:quest-callback/qc-twenty-miles-hoard-")
        && matches!((paid.storage.as_slice(), reentry.storage.as_slice()), ([before], [after])
            if before.track == after.track && before.minimum == 3 && before.maximum == 3
                && before.set == 4 && after.minimum == 4 && after.maximum == 5)
}

// STEP alternatives share geometry only when their personal phases are disjoint.

pub fn exclusive(left: &Callback, right: &Callback) -> bool {
    if disjoint(left, right) {
        return true;
    }
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
            (a.preserve || a.read_only || source_trial)
                && right.storage.iter().any(|b| {
                    (b.preserve || b.read_only || source_trial)
                        && a.track == b.track
                        && (a.maximum < b.minimum || b.maximum < a.minimum)
                })
        })
}

pub fn matches(callback: &Callback, tracks: &BTreeMap<String, i64>) -> bool {
    callback
        .storage
        .iter()
        .filter(|guard| guard.preserve || guard.read_only)
        .all(|guard| {
            tracks
                .get(&guard.track)
                .or_else(|| tracks.get(legacy_read_track(guard)?))
                .is_some_and(|value| (guard.minimum..=guard.maximum).contains(value))
        })
}

/// The roof portal reads the same imported Barbarian Test storage as citizenship.
pub fn legacy_read_track(guard: &Storage) -> Option<&'static str> {
    use crate::quest::barbarian_mead;
    (guard.read_only && guard.track == barbarian_mead::TRACKS[0])
        .then_some(barbarian_mead::LEGACY_QUESTLINE)
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
