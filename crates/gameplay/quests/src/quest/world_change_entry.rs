//! Source-gated admission clocks use the existing Quest tracks and receipts.
//! Warpath page76270/rev944151:24hours; an early revisit restarts the clock.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
pub const CHANGE: &str = "warpath";
pub const QUEST: &str = "oteryn:quest.world_change_warpath";
pub const TRACK: &str = "oteryn:quest-progress/world-change/warpath/last-entry";
pub const ENTER: &str = "oteryn:quest-transition/world-change/warpath/enter";
pub const REFRESH: &str = "oteryn:quest-transition/world-change/warpath/early-revisit";
pub const SECONDS: i64 = 24 * 60 * 60;
pub fn for_transition(key: &str) -> bool {
    matches!(key, ENTER | REFRESH)
}
pub fn requires_native_source(catalogue: &QuestStateCatalogue, key: &str) -> bool {
    for_transition(key)
        || catalogue.transition(key).is_some_and(|transition| {
            transition
                .effects
                .iter()
                .any(|effect| effect.track == TRACK)
        })
}
pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    tracks.push(QuestTrack {
        key: TRACK.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: i64::MAX,
    });
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for (key, from) in [
        (ENTER, QuestComparison::ElapsedAtLeast(SECONDS)),
        (REFRESH, QuestComparison::Any),
    ] {
        transitions.push(QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: TRACK.into(),
                from,
                effect: QuestEffectKind::SetNow,
            }],
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn early_revisit_restarts_twenty_four_hours_and_boundary_is_inclusive() -> Result<(), String> {
        let catalogue = extend(QuestStateCatalogue::empty("test").ok_or("empty")?)?;
        let enter = catalogue.transition(ENTER).ok_or("enter")?;
        let refresh = catalogue.transition(REFRESH).ok_or("refresh")?;
        let mut values = std::collections::BTreeMap::new();
        let start = 100_000;
        let apply = |transition: &QuestTransition,
                     values: &std::collections::BTreeMap<String, i64>,
                     now| {
            catalogue
                .evaluate(transition, values, now)
                .map_err(|e| format!("{e:?}"))
        };
        values.insert(TRACK.into(), apply(enter, &values, start)?[0].after);
        assert!(apply(enter, &values, start + SECONDS - 1).is_err());
        values.insert(
            TRACK.into(),
            apply(refresh, &values, start + SECONDS - 1)?[0].after,
        );
        assert!(apply(enter, &values, start + SECONDS).is_err());
        assert!(apply(enter, &values, start + 2 * SECONDS - 1).is_ok());
        assert!(requires_native_source(&catalogue, ENTER));
        Ok(())
    }
}
