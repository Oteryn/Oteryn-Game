//! Catalogue-scoped counter views; retained counter declarations keep their existing bytes.
use super::Counter;
use crate::quest::{QuestStateCatalogue, authored_bindings::Trigger};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CounterRef<'a> {
    pub transition: &'a str,
    pub creature: &'a str,
    pub quest: &'a str,
    pub track: &'a str,
    pub gate: &'a str,
    pub gate_value: i64,
    pub minimum: i64,
    pub maximum: i64,
    pub death_area: Option<crate::quest::authored_bindings::kills::DeathArea>,
    pub encounter_source: Option<&'a str>,
}

impl Counter {
    fn view(&self) -> CounterRef<'_> {
        CounterRef {
            transition: self.transition,
            creature: self.creature,
            quest: self.quest,
            track: self.track,
            gate: self.gate,
            gate_value: self.gate_value(),
            minimum: 0,
            maximum: self.maximum,
            death_area: self.death_area,
            encounter_source: None,
        }
    }
}

pub fn counter_for<'a>(catalogue: &'a QuestStateCatalogue, key: &str) -> Option<CounterRef<'a>> {
    let transition = catalogue.transition(key)?;
    if let Some(counter) = super::counter(key) {
        return (counter.quest == transition.quest).then(|| counter.view());
    }
    for quest in &catalogue.authored_bindings()?.quests {
        for stage in &quest.stages {
            if let Trigger::Kill { counter } = &stage.trigger
                && counter.key == key
                && quest.quest == transition.quest
            {
                return Some(counter.counter(&quest.quest));
            }
        }
    }
    None
}

pub fn active_for(catalogue: &QuestStateCatalogue, tracks: &BTreeMap<String, i64>) -> Vec<String> {
    let mut active: Vec<_> = super::active(tracks)
        .into_iter()
        .filter(|key| counter_for(catalogue, key).is_some())
        .map(str::to_owned)
        .collect();
    if let Some(bindings) = catalogue.authored_bindings() {
        for quest in &bindings.quests {
            for stage in &quest.stages {
                let Trigger::Kill { counter } = &stage.trigger else {
                    continue;
                };
                let (Some(gate), Some(count)) = (
                    catalogue.track(&counter.gate),
                    catalogue.track(&counter.track),
                ) else {
                    continue;
                };
                let observed_gate = tracks.get(&counter.gate).copied().unwrap_or(gate.initial);
                let observed_count = tracks.get(&counter.track).copied().unwrap_or(count.initial);
                if observed_gate == counter.gate_value
                    && (counter.minimum..counter.maximum).contains(&observed_count)
                {
                    active.push(counter.key.clone());
                }
            }
        }
    }
    active
}

pub fn tracks_creature_for(catalogue: &QuestStateCatalogue, creature: &str) -> bool {
    super::counters().any(|counter|counter.creature==creature && catalogue.transition(counter.transition).is_some())
        || catalogue.authored_bindings().is_some_and(|bindings|bindings.quests.iter().any(|quest|
            quest.stages.iter().any(|stage|matches!(&stage.trigger,Trigger::Kill {counter} if counter.creature==creature))))
}

impl QuestStateCatalogue {
    /// A kill stage completes at its declared goal, not on its first credited occurrence.
    pub fn completes_after(
        &self,
        transition: &crate::quest::QuestTransition,
        changes: &[crate::quest::QuestTrackChange],
    ) -> bool {
        transition.completes
            && counter_for(self, &transition.key).is_none_or(|counter| {
                changes
                    .iter()
                    .any(|change| change.track == counter.track && change.after == counter.maximum)
            })
    }
}

#[cfg(test)]
mod daily_tests {
    use super::*;

    #[test]
    fn spike_kills_check_victim_species_floor_and_area() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("ctdaily")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for (tier, species, minimum, maximum) in [
            (
                "upper",
                "demon_skeleton",
                [32008, 32522, 8],
                [32365, 32759, 10],
            ),
            (
                "middle",
                "crystalcrusher",
                [32100, 32470, 11],
                [32380, 32725, 12],
            ),
            ("lower", "drillworm", [32120, 32470, 13], [32345, 32710, 15]),
        ] {
            let key = format!("oteryn:quest-transition/native-kills/spike-{tier}-kill");
            let counter = counter_for(catalogue, &key).ok_or("Spike kill counter")?;
            let area = counter.death_area.ok_or("Spike kill area")?;
            assert_eq!(counter.creature, format!("oteryn:creature.{species}"));
            assert_eq!(area.minimum, minimum);
            assert_eq!(area.maximum, maximum);
            assert_eq!(
                super::super::death_position_matches(&key, Some(area), Some([32539, 32000, 15])),
                tier == "lower"
            );
            assert!(!super::super::death_position_matches(
                &key,
                Some(area),
                Some([32539, 32000, 14])
            ));
            assert!(!super::super::death_position_matches(
                &key,
                Some(area),
                Some([32400, 32300, 15])
            ));
            assert!(area.contains(minimum));
            assert!(area.contains(maximum));
            assert!(!area.contains([minimum[0], minimum[1], minimum[2] - 1]));
            let mut tracks =
                BTreeMap::from([(counter.gate.to_owned(), 1), (counter.track.to_owned(), 0)]);
            assert!(active_for(catalogue, &tracks).contains(&key));
            tracks.insert(counter.track.to_owned(), 7);
            assert!(!active_for(catalogue, &tracks).contains(&key));
        }
        Ok(())
    }

    #[test]
    fn bigfoot_and_dangerous_depths_kills_share_the_existing_counter_owner() -> Result<(), String> {
        let loaded = crate::quest::loader::load_embedded_quest_state("ctdaily")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for (mission, names, maximum) in [
            ("bigfoot-exterminators", &["wiggler"][..], 10),
            (
                "dangerous-depths-subterranean",
                &["deepworm", "diremaw"][..],
                50,
            ),
        ] {
            for name in names {
                let key = format!("oteryn:quest-transition/native-kills/{mission}-{name}");
                let counter = counter_for(catalogue, &key).ok_or(key.clone())?;
                assert_eq!(counter.creature, format!("oteryn:creature.{name}"));
                let state = BTreeMap::from([
                    (counter.gate.to_owned(), 1),
                    (counter.track.to_owned(), maximum - 1),
                ]);
                assert!(active_for(catalogue, &state).contains(&key));
                let transition = catalogue.transition(&key).ok_or("kill transition")?;
                let changes = catalogue
                    .evaluate(transition, &state, 0)
                    .map_err(|error| format!("{error:?}"))?;
                assert!(
                    changes
                        .iter()
                        .any(|change| change.track == counter.track && change.after == maximum)
                );
                let mut capped = state;
                capped.insert(counter.track.to_owned(), maximum);
                assert!(!active_for(catalogue, &capped).contains(&key));
                capped.insert(counter.track.to_owned(), 0);
                capped.insert(counter.gate.to_owned(), 2);
                assert!(!active_for(catalogue, &capped).contains(&key));
            }
        }
        Ok(())
    }
}
