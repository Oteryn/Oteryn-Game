//! Authored counter defaults never rewrite an existing track declaration.
use super::{Callback, QuestTrack};

pub(super) fn valid(callback: &Callback) -> bool {
    callback.track_initials.iter().all(|(key, initial)| {
        callback.native_rule.is_none()
            && callback.storage.iter().any(|storage| {
                storage.track == *key
                    && !storage.preserve
                    && *initial <= storage.maximum.max(storage.set)
            })
    })
}

pub(super) fn check_existing(callback: &Callback, track: &QuestTrack) -> Result<(), String> {
    if callback
        .track_initials
        .get(&track.key)
        .is_some_and(|initial| *initial != track.initial || callback.quest != track.quest)
    {
        return Err(format!(
            "conflicting callback counter default {}",
            track.key
        ));
    }
    Ok(())
}
