//! Source death counters, consumed by the existing lethal queue and quest writer.
//! Engine reused from questpc3 5e72a3f0b; the selected trial bindings stay in this lane.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use std::collections::BTreeMap;

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
    /// Preserve the selected authored stage's exact no-op gate and LT bound.
    #[serde(default)]
    pub authored_stage: bool,
    pub world_change: Option<&'static str>,
}

impl Counter {
    pub(crate) fn transition(self) -> QuestTransition {
        QuestTransition {
            world_change: self.world_change.map(str::to_owned),
            key: self.transition.into(),
            quest: self.quest.into(),
            effects: vec![
                QuestEffect {
                    track: self.gate.into(),
                    from: QuestComparison::Eq(1),
                    effect: if self.authored_stage {
                        QuestEffectKind::Set(1)
                    } else {
                        QuestEffectKind::Add(0)
                    },
                },
                QuestEffect {
                    track: self.track.into(),
                    from: if self.authored_stage {
                        QuestComparison::Lt(self.maximum)
                    } else {
                        QuestComparison::Between(0, self.maximum - 1)
                    },
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
        authored_stage: false,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/dragon-slayer-lord",
        creature: "oteryn:creature.dragon_lord",
        quest: "oteryn:quest.authored.dragon_slayer_outfits_quest",
        track: "oteryn:quest-progress/chosen-trial/dragon_slayer/dragon_lords",
        gate: "oteryn:quest-progress/chosen-trial/dragon_slayer/phase",
        maximum: 20,
        authored_stage: false,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/cave-explorer",
        creature: "oteryn:creature.crystal_spider",
        quest: "oteryn:quest.cave_explorer_outfits_quest",
        track: "oteryn:quest-progress/chosen-trial/cave_explorer/kills",
        gate: "oteryn:quest-progress/chosen-trial/cave_explorer/phase",
        maximum: 20,
        authored_stage: false,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/native-kills/fire-fighter",
        creature: "oteryn:creature.lava_lurker",
        quest: "oteryn:quest.fire_fighter_outfits_quest",
        track: "oteryn:quest-progress/chosen-trial/fire_fighter/kills",
        gate: "oteryn:quest-progress/chosen-trial/fire_fighter/phase",
        maximum: 10,
        authored_stage: false,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/s5",
        creature: "oteryn:creature.ahau",
        quest: "oteryn:quest.adventures_of_galthen_quest",
        track: "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/s5",
        gate: "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/s4",
        maximum: 1,
        authored_stage: true,
        world_change: None,
    },
    Counter {
        transition: "oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/s8",
        creature: "oteryn:creature.mitmah_vanguard",
        quest: "oteryn:quest.adventures_of_galthen_quest",
        track: "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/s8",
        gate: "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/s7",
        maximum: 1,
        authored_stage: true,
        world_change: None,
    },
];

#[must_use]
pub fn counter(transition: &str) -> Option<&'static Counter> {
    counters().find(|counter| counter.transition == transition)
}

#[must_use]
pub fn tracks_creature(creature: &str) -> bool {
    counters().any(|counter| counter.creature == creature)
}

#[must_use]
pub fn active(tracks: &BTreeMap<String, i64>) -> Vec<&'static str> {
    counters()
        .filter(|counter| {
            tracks.get(counter.gate) == Some(&1)
                && (0..counter.maximum).contains(&tracks.get(counter.track).copied().unwrap_or(0))
        })
        .map(|counter| counter.transition)
        .collect()
}

pub(crate) fn counters() -> impl Iterator<Item = &'static Counter> {
    COUNTERS
        .iter()
        .chain(qworld::counters().unwrap_or_default())
}

/// Fail closed when a file-authored counter registry cannot be loaded.
pub(super) fn registered_counters() -> Result<impl Iterator<Item = &'static Counter>, String> {
    Ok(COUNTERS.iter().chain(qworld::counters()?))
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.into_values().collect();
    qworld::extend_tracks(&mut tracks)?;
    let mut transitions: Vec<_> = catalogue.transitions.into_values().collect();
    for counter in counters() {
        let exact = counter.transition();
        if let Some(existing) = transitions
            .iter()
            .find(|transition| transition.key == exact.key)
        {
            if existing != &exact {
                return Err(format!("native kill transition changed: {}", exact.key));
            }
        } else if counter.authored_stage {
            // Selected authored stages must come from their authoring source,
            // never from a fallback fabricated catalogue entry.
            return Err(format!(
                "authored native kill transition missing: {}",
                exact.key
            ));
        } else {
            transitions.push(exact);
        }
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
    #[test]
    fn selected_galthen_deaths_require_their_authored_predecessor_and_never_complete_report()
    -> Result<(), String> {
        for (stage, prerequisite, creature) in [
            ("s5", "s4", "oteryn:creature.ahau"),
            ("s8", "s7", "oteryn:creature.mitmah_vanguard"),
        ] {
            let key = format!(
                "oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/{stage}"
            );
            let counter = counter(&key).ok_or("selected authored death")?;
            assert_eq!(counter.creature, creature);
            assert_eq!(
                counter.gate,
                format!(
                    "oteryn:quest-progress/chosen-source/adventures_of_galthen_quest/{prerequisite}"
                )
            );
            assert_eq!(counter.maximum, 1);
            assert_eq!(counter.world_change, None);
            let transition = counter.transition();
            assert!(!transition.completes);
            assert_eq!(transition.effects[0].from, QuestComparison::Eq(1));
            assert_eq!(transition.effects[0].effect, QuestEffectKind::Set(1));
            assert_eq!(transition.effects[1].from, QuestComparison::Lt(1));
            let mut tracks = BTreeMap::new();
            assert!(!active(&tracks).contains(&counter.transition));
            tracks.insert(counter.gate.into(), 0);
            assert!(!active(&tracks).contains(&counter.transition));
            tracks.insert(counter.gate.into(), 1);
            assert!(active(&tracks).contains(&counter.transition));
            tracks.insert(counter.track.into(), 1);
            assert!(!active(&tracks).contains(&counter.transition));
            tracks.insert(counter.track.into(), 0);
            tracks.insert(counter.gate.into(), 2);
            assert!(!active(&tracks).contains(&counter.transition));
        }
        assert!(
            counter("oteryn:quest-transition/chosen-source/adventures_of_galthen_quest/s9")
                .is_none()
        );
        assert!(
            COUNTERS
                .iter()
                .filter(|counter| !counter.authored_stage)
                .all(|counter| counter.transition().effects[0].effect == QuestEffectKind::Add(0))
        );
        Ok(())
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
            assert!(
                !counter.authored_stage,
                "incoming JSON objectives retain ordinary gate semantics"
            );
            assert_eq!(counter.transition().world_change.as_deref(), Some(world));
        }
        Ok(())
    }
}
