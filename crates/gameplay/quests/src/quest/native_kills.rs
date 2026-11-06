//! Source death counters, consumed by the existing lethal queue and quest writer.
//! Canary 666a60f6: the_first_dragon/creaturescripts_kill_dragon.lua:3-9;
//! threatened_dreams/nightmare_monsters_kills.lua:16-27. Both use onDeathForParty.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use std::collections::BTreeMap;

pub const PREFIX: &str = "oteryn:quest-transition/native-kills/";

#[derive(Debug, Clone, Copy)]
pub struct Counter {
    pub transition: &'static str,
    pub creature: &'static str,
    pub quest: &'static str,
    pub track: &'static str,
    pub gate: &'static str,
    pub maximum: i64,
}

pub const COUNTERS: &[Counter] = &[
    Counter {
        transition: "oteryn:quest-transition/native-kills/first-dragon",
        creature: "oteryn:creature.dragon",
        quest: "oteryn:quest.the_first_dragon_quest",
        track: "oteryn:quest-progress/quest/u11_02/the_first_dragon/dragon_counter",
        // The historical lowered counter starts at zero rather than source -1.
        // Require the source quest start so that an unstarted task cannot count.
        gate: "oteryn:quest-progress/quest/u11_02/the_first_dragon/questline",
        maximum: 200,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/threatened-enfeebled",
        creature: "oteryn:creature.enfeebled_silencer",
        quest: "oteryn:quest.threatened_dreams_quest",
        track: "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/enfeebled_count",
        gate: super::threatened_dreams::NIGHTMARE,
        maximum: 2_147_483_647,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/threatened-frazzlemaw",
        creature: "oteryn:creature.weakened_frazzlemaw",
        quest: "oteryn:quest.threatened_dreams_quest",
        track: "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/frazzlemaws_count",
        gate: super::threatened_dreams::NIGHTMARE,
        maximum: 2_147_483_647,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/threatened-kroazur",
        creature: "oteryn:creature.kroazur",
        quest: "oteryn:quest.threatened_dreams_quest",
        track: "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_kill",
        gate: super::threatened_dreams::NIGHTMARE,
        maximum: 1,
    },
];

#[must_use]
pub fn counter(transition: &str) -> Option<&'static Counter> {
    COUNTERS
        .iter()
        .find(|counter| counter.transition == transition)
}

#[must_use]
pub fn tracks_creature(creature: &str) -> bool {
    COUNTERS.iter().any(|counter| counter.creature == creature)
}

#[must_use]
pub fn active(tracks: &BTreeMap<String, i64>) -> Vec<&'static str> {
    COUNTERS
        .iter()
        .filter(|counter| {
            tracks.get(counter.gate) == Some(&1)
                && (0..counter.maximum).contains(&tracks.get(counter.track).copied().unwrap_or(0))
        })
        .map(|counter| counter.transition)
        .collect()
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    for counter in COUNTERS {
        transitions.push(QuestTransition {
            key: counter.transition.into(),
            quest: counter.quest.into(),
            effects: vec![
                QuestEffect {
                    track: counter.gate.into(),
                    from: QuestComparison::Eq(1),
                    effect: QuestEffectKind::Add(0),
                },
                QuestEffect {
                    track: counter.track.into(),
                    from: QuestComparison::Between(0, counter.maximum - 1),
                    effect: QuestEffectKind::Add(1),
                },
            ],
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::QuestRefusal;

    #[test]
    fn native_dragon_counter_requires_start_and_stops_at_two_hundred() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("native-counter-test")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let counter = &COUNTERS[0];
        let transition = catalogue
            .transition(counter.transition)
            .ok_or("dragon binding")?;
        let mut stored = BTreeMap::new();
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(QuestRefusal::StageMismatch)
        );
        stored.insert(counter.gate.into(), 1);
        for expected in 1..=200 {
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|error| format!("{error:?}"))?;
            let count = changes
                .iter()
                .find(|change| change.track == counter.track)
                .ok_or("dragon count")?;
            assert_eq!(count.after, expected);
            for change in changes {
                stored.insert(change.track, change.after);
            }
        }
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(QuestRefusal::StageMismatch)
        );
        assert!(active(&stored).is_empty());
        Ok(())
    }

    #[test]
    fn native_kroazur_counts_once_only_during_nightmare_mission() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("native-counter-test")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let counter = counter("oteryn:quest-transition/native-kills/threatened-kroazur")
            .ok_or("Kroazur binding")?;
        let transition = catalogue
            .transition(counter.transition)
            .ok_or("Kroazur transition")?;
        for stage in [0, 2, 3, 8] {
            let stored = BTreeMap::from([(counter.gate.into(), stage)]);
            assert_eq!(
                catalogue.evaluate(transition, &stored, 0),
                Err(QuestRefusal::StageMismatch)
            );
        }
        let mut stored = BTreeMap::from([(counter.gate.into(), 1)]);
        let changes = catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|error| format!("{error:?}"))?;
        for change in changes {
            stored.insert(change.track, change.after);
        }
        assert_eq!(stored.get(counter.track), Some(&1));
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(QuestRefusal::StageMismatch)
        );
        assert!(!tracks_creature("oteryn:creature.cockroach"));
        Ok(())
    }
}
