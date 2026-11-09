//! Source death counters, consumed by the existing lethal queue and quest writer.
//! Engine reused from questpc3 5e72a3f0b; the selected trial bindings stay in this lane.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use std::collections::BTreeMap;

mod fugue;
mod phase_settlements;

mod registry;
pub use registry::{CounterRef, active_for, counter_for, tracks_creature_for};

pub const PREFIX: &str = "oteryn:quest-transition/native-kills/";

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counter {
    pub transition: &'static str,
    pub creature: &'static str,
    pub quest: &'static str,
    pub track: &'static str,
    pub gate: &'static str,
    pub maximum: i64,
    pub world_change: Option<&'static str>,
}

impl Counter {
    #[must_use]
    pub fn gate_bounds(self) -> (i64, i64) {
        if fugue::matches(&self) {
            (2, 2)
        } else {
            phase_settlements::gate_bounds(&self).unwrap_or((1, 1))
        }
    }

    pub(crate) fn transition(self) -> QuestTransition {
        let settlement = if fugue::matches(&self) {
            Some(fugue::effects())
        } else {
            phase_settlements::effects(&self)
        };
        if let Some(effects) = settlement {
            return QuestTransition {
                world_change: None,
                key: self.transition.into(),
                quest: self.quest.into(),
                effects,
                completes: false,
                experience: None,
            };
        }
        QuestTransition {
            world_change: self.world_change.map(str::to_owned),
            key: self.transition.into(),
            quest: self.quest.into(),
            effects: vec![
                QuestEffect {
                    track: self.gate.into(),
                    from: QuestComparison::Eq(1),
                    effect: QuestEffectKind::Add(0),
                },
                QuestEffect {
                    track: self.track.into(),
                    from: QuestComparison::Between(0, self.maximum - 1),
                    effect: QuestEffectKind::Add(1),
                },
            ],
            completes: false,
            experience: None,
        }
    }
}

#[cfg(test)]
#[path = "native_kills/ownership_tests.rs"]
mod ownership_tests;

#[path = "native_kills/qworld.rs"]
mod qworld;

pub const COUNTERS: &[Counter] = &[
    Counter {
        transition: "oteryn:quest-transition/native-kills/dragon-slayer-dragon",
        creature: "oteryn:creature.dragon",
        quest: "oteryn:quest.authored.dragon_slayer_outfits_quest",
        track: "oteryn:quest-progress/chosen-trial/dragon_slayer/dragons",
        gate: "oteryn:quest-progress/chosen-trial/dragon_slayer/phase",
        maximum: 20,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/dragon-slayer-lord",
        creature: "oteryn:creature.dragon_lord",
        quest: "oteryn:quest.authored.dragon_slayer_outfits_quest",
        track: "oteryn:quest-progress/chosen-trial/dragon_slayer/dragon_lords",
        gate: "oteryn:quest-progress/chosen-trial/dragon_slayer/phase",
        maximum: 20,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/cave-explorer",
        creature: "oteryn:creature.crystal_spider",
        quest: "oteryn:quest.cave_explorer_outfits_quest",
        track: "oteryn:quest-progress/chosen-trial/cave_explorer/kills",
        gate: "oteryn:quest-progress/chosen-trial/cave_explorer/phase",
        maximum: 20,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/fire-fighter",
        creature: "oteryn:creature.lava_lurker",
        quest: "oteryn:quest.fire_fighter_outfits_quest",
        track: "oteryn:quest-progress/chosen-trial/fire_fighter/kills",
        gate: "oteryn:quest-progress/chosen-trial/fire_fighter/phase",
        maximum: 10,
        world_change: None,
    },
];

#[must_use]
pub fn counter(transition: &str) -> Option<&'static Counter> {
    counters()
        .chain(phase_settlements::COUNTERS)
        .find(|counter| counter.transition == transition)
}

#[must_use]
pub fn tracks_creature(creature: &str) -> bool {
    counters()
        .chain(phase_settlements::COUNTERS)
        .any(|counter| counter.creature == creature)
}

#[must_use]
pub fn active(tracks: &BTreeMap<String, i64>) -> Vec<&'static str> {
    counters()
        .filter(|counter| {
            let (minimum, maximum) = counter.gate_bounds();
            (minimum..=maximum).contains(&tracks.get(counter.gate).copied().unwrap_or(0))
                && (0..counter.maximum).contains(&tracks.get(counter.track).copied().unwrap_or(0))
        })
        .map(|counter| counter.transition)
        .collect()
}

/// Only the shared encounter admission may capture terminal phase settlement.
#[must_use]
pub fn active_encounter(tracks: &BTreeMap<String, i64>) -> Vec<&'static str> {
    phase_settlements::COUNTERS
        .iter()
        .filter(|counter| {
            let (minimum, maximum) = counter.gate_bounds();
            (minimum..=maximum).contains(&tracks.get(counter.gate).copied().unwrap_or(0))
        })
        .map(|counter| counter.transition)
        .collect()
}

fn counters() -> impl Iterator<Item = &'static Counter> {
    COUNTERS
        .iter()
        .chain(qworld::counters().unwrap_or_default())
        .chain(fugue::COUNTERS)
}

/// Fail closed when a file-authored counter registry cannot be loaded.
pub(super) fn registered_counters() -> Result<impl Iterator<Item = &'static Counter>, String> {
    Ok(COUNTERS.iter().chain(qworld::counters()?))
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.into_values().collect();
    qworld::extend_tracks(&mut tracks)?;
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    for counter in counters().chain(phase_settlements::COUNTERS) {
        transitions.push(counter.transition());
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eligibility_requires_an_active_trial_and_an_unsaturated_counter() {
        let counter = &COUNTERS[0];
        let mut tracks = BTreeMap::new();
        assert!(active(&tracks).is_empty());
        tracks.insert(counter.gate.into(), 1);
        assert!(active(&tracks).contains(&counter.transition));
        tracks.insert(counter.track.into(), counter.maximum);
        assert!(!active(&tracks).contains(&counter.transition));
        tracks.insert(counter.track.into(), -1);
        assert!(!active(&tracks).contains(&counter.transition));
        tracks.insert(counter.track.into(), 0);
        tracks.insert(counter.gate.into(), 2);
        assert!(active(&tracks).is_empty());
        assert!(!tracks_creature("oteryn:creature.rat"));
    }
}

#[cfg(test)]
mod qworld_tests {
    use super::*;
    #[test]
    fn qworld_death_objectives_bind_their_native_world_condition() -> Result<(), String> {
        for (creature, world, maximum) in [
            ("oteryn:creature.fury", "fury_gates", 10),
            ("oteryn:creature.werewolf", "grimvale", 10),
        ] {
            let counter = counters()
                .find(|c| c.creature == creature)
                .ok_or_else(|| format!("missing native objective {creature}"))?;
            assert_eq!(counter.maximum, maximum);
            assert_eq!(counter.transition().world_change.as_deref(), Some(world));
        }
        Ok(())
    }
}
