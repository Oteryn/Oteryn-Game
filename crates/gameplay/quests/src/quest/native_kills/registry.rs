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
            gate_value: 1,
            minimum: 0,
            maximum: self.maximum,
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
