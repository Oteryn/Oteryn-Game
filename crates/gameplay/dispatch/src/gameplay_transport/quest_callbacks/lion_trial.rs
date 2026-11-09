//! The source trial checks and changes a separate shared pagoda under the frozen turn.
use super::*;
use crate::map::facts::QuestCallbackProjection;

pub(super) fn prepare(
    map: &BundleMap,
    callback: &Callback,
    replacements: &mut Vec<(u64, crate::map::view::EntryFacts, bool)>,
    now: u64,
) -> Result<Option<QuestCallbackProjection>, UseDisposition> {
    if oteryn_gameplay_quests::quest::lions_rock::binding(&callback.key).is_none() {
        return Ok(None);
    }
    let [(key, after, _)] = replacements.as_slice() else {
        return Err(UseDisposition::Rejected);
    };
    let (position, ordinal) =
        crate::map::view::placement_of(*key).ok_or(UseDisposition::Rejected)?;
    let base = map
        .overlay
        .base()
        .tile(position.x, position.y, position.floor)
        .ok_or(UseDisposition::Rejected)?;
    let id = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .map(|(id, _)| *id)
        .ok_or(UseDisposition::Rejected)?;
    let restore = map
        .facts
        .base_entry(position, ordinal, id)
        .ok_or(UseDisposition::Rejected)?;
    if restore.appearance_id != 2074 || after.appearance_id != 2075 || map.facts.base_hidden(*key) {
        return Err(UseDisposition::NothingToUse);
    }
    let result = QuestCallbackProjection::TimedRemoteTransform {
        key: *key,
        expected: map.facts.object_revision(*key),
        after: *after,
        restore,
        due: now
            .checked_add(15_000_000)
            .ok_or(UseDisposition::Exhausted)?,
    };
    replacements.clear();
    Ok(Some(result))
}
