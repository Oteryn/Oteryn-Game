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
