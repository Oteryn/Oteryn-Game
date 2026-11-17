//! Shared bounded storage lowering; inventory facts stay idempotent.
use super::Track;
use crate::quest::QuestEffectKind;
use crate::quest::world_callbacks::Storage;

pub(super) fn valid(storage: &Storage, track: &Track, allow_increment: bool) -> bool {
    if storage.minimum > storage.maximum
        || storage.minimum < track.minimum
        || storage.maximum > track.maximum
        || u8::from(storage.preserve) + u8::from(storage.increment) + u8::from(storage.at_least) > 1
    {
        return false;
    }
    if storage.increment {
        allow_increment
            && storage
                .minimum
                .checked_add(storage.set)
                .zip(storage.maximum.checked_add(storage.set))
                .is_some_and(|(minimum, maximum)| {
                    minimum >= track.minimum && maximum <= track.maximum
                })
    } else {
        (track.minimum..=track.maximum).contains(&storage.set)
    }
}

pub(super) fn effect(storage: &Storage, binding: [u8; 32]) -> QuestEffectKind {
    if storage.preserve {
        QuestEffectKind::Add(0)
    } else if storage.increment {
        QuestEffectKind::AddWithBinding {
            delta: storage.set,
            binding,
        }
    } else if storage.at_least {
        QuestEffectKind::RaiseToWithBinding {
            value: storage.set,
            binding,
        }
    } else {
        QuestEffectKind::SetWithBinding {
            value: storage.set,
            binding,
        }
    }
}
