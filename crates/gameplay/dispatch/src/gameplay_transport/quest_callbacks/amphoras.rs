//! Ape City mission nine starts a shared five-minute window on the fourth distinct strike.
use super::*;
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::EntryFacts;

pub(super) const GATE: &str = "oteryn:quest-callback/ape-city-catacombs-amphora-gate";

pub(super) fn is_strike(callback: &Callback) -> bool {
    callback
        .key
        .starts_with("oteryn:quest-callback/ape-city-amphora-")
}

pub(super) fn prepare(
    callbacks: &[Callback],
    map: &BundleMap,
    source: u64,
    restore: EntryFacts,
    projection: Option<QuestCallbackProjection>,
) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
    let Some(QuestCallbackProjection::TimedTransform { due, .. }) = projection else {
        return Err(UseDisposition::Rejected);
    };
    let mut others = Vec::with_capacity(3);
    let mut all_broken = true;
    for callback in callbacks.iter().filter(|callback| is_strike(callback)) {
        let selector = callback
            .selector
            .native
            .as_ref()
            .ok_or(UseDisposition::Rejected)?;
        let position = tile(selector.position);
        let key = placement_key(position, selector.ordinal).ok_or(UseDisposition::Rejected)?;
        if key == source {
            continue;
        }
        let current = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
            .and_then(|base| {
                base.ids()
                    .iter()
                    .zip(base.depths())
                    .filter(|(_, depth)| **depth == 0)
                    .nth(usize::from(selector.ordinal))
            })
            .and_then(|(id, _)| map.facts.base_entry(position, selector.ordinal, *id))
            .filter(|_| !map.facts.base_hidden(key))
            .ok_or(UseDisposition::Rejected)?;
        let broken = callback.effects.transform.ok_or(UseDisposition::Rejected)?;
        if current.appearance_id == selector.appearance {
            all_broken = false;
        } else if current.appearance_id != broken {
            return Err(UseDisposition::Rejected);
        }
        let restore = map
            .facts
            .quest_callback_replacement(key, selector.appearance)
            .ok_or(UseDisposition::Rejected)?;
        others.push((key, map.facts.object_revision(key), restore));
    }
    let others: [(u64, u64, EntryFacts); 3] =
        others.try_into().map_err(|_| UseDisposition::Rejected)?;
    if !all_broken {
        return Ok(None);
    }
    if !map
        .facts
        .quest_callback_capacity(source, map.facts.object_revision(source), 8)
        || others
            .iter()
            .any(|(key, revision, _)| !map.facts.quest_callback_capacity(*key, *revision, 0))
    {
        return Err(UseDisposition::Exhausted);
    }
    Ok(Some(QuestCallbackProjection::TimedAmphoras {
        due,
        restore,
        others,
    }))
}
