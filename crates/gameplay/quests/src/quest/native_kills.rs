//! Source death counters, consumed by the existing lethal queue and quest writer.
//! Engine reused from questpc3 5e72a3f0b; the selected trial bindings stay in this lane.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use std::collections::BTreeMap;

mod registry;
pub use registry::{CounterRef, active_for, counter_for, tracks_creature_for};

mod fugue;
pub(crate) mod great_dragon_hunt;
mod phase_settlements;

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
    #[must_use]
    pub fn gate_bounds(self) -> (i64, i64) {
        if fugue::matches(&self) {
            (2, 2)
        } else if great_dragon_hunt::owns(self.transition) {
            // Crystal credits the party even before the explorer is searched.
            (0, 2)
        } else {
            phase_settlements::gate_bounds(&self).unwrap_or((self.gate_value(), self.gate_value()))
        }
    }

    /// The selected First Dragon recipe requires both s4 exploration occurrences.
    #[must_use]
    pub fn gate_value(&self) -> i64 {
        if self.transition == "oteryn:quest-transition/native-kills/first-dragon-selected" {
            2
        } else {
            1
        }
    }

    /// Its selected access reward is the canonical quest completion, not an Item MINT.
    #[must_use]
    pub fn completion_track(&self) -> Option<&'static str> {
        (self.transition == "oteryn:quest-transition/native-kills/first-dragon-selected")
            .then_some("oteryn:quest-progress/chosen-source/the_first_dragon_quest/s6")
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
        if great_dragon_hunt::owns(self.transition) {
            // The native writer fences the skeleton revision so an older kill
            // cannot cross the explorer's reset. A kill only changes the count.
            return QuestTransition {
                world_change: None,
                key: self.transition.into(),
                quest: self.quest.into(),
                effects: vec![QuestEffect {
                    track: self.track.into(),
                    from: QuestComparison::Between(0, self.maximum - 1),
                    effect: QuestEffectKind::Add(1),
                }],
                completes: false,
                experience: None,
            };
        }
        let mut effects = vec![
            QuestEffect {
                track: self.gate.into(),
                from: QuestComparison::Eq(self.gate_value()),
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
        ];
        if let Some(track) = self.completion_track() {
            effects.push(QuestEffect {
                track: track.into(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            });
        }
        QuestTransition {
            world_change: self.world_change.map(str::to_owned),
            key: self.transition.into(),
            quest: self.quest.into(),
            effects,
            completes: self.completion_track().is_some()
                || super::world_change_journals::completes(self.transition),
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
    // Chosen recipe quests-00200-00299.json record23 s4/s5/s6 deliberately omits
    // the full source phases. Crystal201189f creaturescripts_death_first_dragon.lua:21-30
    // establishes the authoritative death; selected access completion joins that receipt.
    Counter {
        authored_stage: false,
        transition: "oteryn:quest-transition/native-kills/first-dragon-selected",
        creature: "oteryn:creature.the_first_dragon",
        quest: "oteryn:quest.the_first_dragon_quest",
        track: "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s5",
        gate: "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s4",
        maximum: 1,
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
    counters().any(|counter| counter.creature == creature)
}

#[must_use]
pub fn active(tracks: &BTreeMap<String, i64>) -> Vec<&'static str> {
    counters()
        .filter(|counter| {
            let (minimum, maximum) = counter.gate_bounds();
            (minimum..=maximum).contains(&tracks.get(counter.gate).copied().unwrap_or(0))
                && counter
                    .completion_track()
                    .is_none_or(|track| tracks.get(track).copied().unwrap_or(0) == 0)
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

// Ordinary lethal counters exclude terminal settlements: only an admitted
// encounter may capture those through active_encounter. The complete registry
// below still owns their canonical transition and definition hash.
pub(crate) fn counters() -> impl Iterator<Item = &'static Counter> {
    COUNTERS
        .iter()
        .chain(qworld::counters().unwrap_or_default())
        .chain(fugue::COUNTERS)
        .chain(great_dragon_hunt::COUNTERS)
        .chain(
            super::world_change_journals::counters()
                .into_iter()
                .flatten(),
        )
}

/// Fail closed when a file-authored counter registry cannot be loaded.
pub(super) fn registered_counters() -> Result<impl Iterator<Item = &'static Counter>, String> {
    Ok(COUNTERS
        .iter()
        .chain(fugue::COUNTERS)
        .chain(great_dragon_hunt::COUNTERS)
        .chain(phase_settlements::COUNTERS)
        .chain(qworld::counters()?)
        .chain(super::world_change_journals::counters()?))
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks.clone();
    let quest = "oteryn:quest.the_first_dragon_quest";
    for (key, maximum) in [
        (
            "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s4",
            2,
        ),
        (
            "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s5",
            1,
        ),
        (
            "oteryn:quest-progress/chosen-source/the_first_dragon_quest/s6",
            1,
        ),
    ] {
        let expected = super::QuestTrack {
            key: key.into(),
            quest: quest.into(),
            initial: 0,
            min: 0,
            max: maximum,
        };
        if let Some(current) = tracks.get(key) {
            if current != &expected {
                return Err(format!("selected First Dragon track conflict: {key}"));
            }
        } else {
            tracks.insert(key.into(), expected);
        }
    }
    let mut tracks: Vec<_> = tracks.into_values().collect();
    qworld::extend_tracks(&mut tracks)?;
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for counter in registered_counters()? {
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
    let mut extended = QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))?;
    for (quest, hash) in &catalogue.hashes {
        let unchanged_tracks = extended
            .tracks
            .values()
            .filter(|t| &t.quest == quest)
            .all(|track| catalogue.tracks.get(&track.key) == Some(track));
        let unchanged_transitions = extended
            .transitions
            .values()
            .filter(|t| &t.quest == quest)
            .all(|transition| catalogue.transitions.get(&transition.key) == Some(transition));
        if unchanged_tracks && unchanged_transitions {
            extended.hashes.insert(quest.clone(), *hash);
        }
    }
    extended.npc_legacy = catalogue.npc_legacy;
    extended.predecessors = catalogue.predecessors;
    extended.authored_bindings = catalogue.authored_bindings;
    extended.native_recipe_evidence = catalogue.native_recipe_evidence;
    extended.completion_sets = catalogue.completion_sets;
    Ok(extended)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eligibility_requires_an_active_trial_and_an_unsaturated_counter() {
        let counter = &COUNTERS[0];
        let mut tracks = BTreeMap::new();
        let unconditional = [
            "oteryn:quest-transition/native-kills/great-dragon-hunt/dragon",
            "oteryn:quest-transition/native-kills/great-dragon-hunt/dragon_lord",
        ];
        assert_eq!(active(&tracks), unconditional);
        tracks.insert(counter.gate.into(), 1);
        assert!(active(&tracks).contains(&counter.transition));
        tracks.insert(counter.track.into(), counter.maximum);
        assert!(!active(&tracks).contains(&counter.transition));
        tracks.insert(counter.track.into(), -1);
        assert!(!active(&tracks).contains(&counter.transition));
        tracks.insert(counter.track.into(), 0);
        tracks.insert(counter.gate.into(), 2);
        assert_eq!(active(&tracks), unconditional);
        assert!(!tracks_creature("oteryn:creature.rat"));
    }
    #[test]
    fn m18_first_dragon_death_requires_both_trials_and_completes_selected_access_once()
    -> Result<(), String> {
        let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let counter = counter("oteryn:quest-transition/native-kills/first-dragon-selected")
            .ok_or("selected counter")?;
        assert_eq!(
            counter_for(catalogue, counter.transition)
                .ok_or("runtime counter view")?
                .gate_value,
            2
        );
        let transition = catalogue
            .transition(counter.transition)
            .ok_or("selected transition")?;
        assert!(transition.completes);
        let mut stored = BTreeMap::new();
        for value in [0, 1] {
            stored.insert(counter.gate.into(), value);
            assert!(!active(&stored).contains(&counter.transition));
            assert!(catalogue.evaluate(transition, &stored, 0).is_err());
        }
        stored.insert(counter.gate.into(), 2);
        assert!(active(&stored).contains(&counter.transition));
        for change in catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|error| format!("{error:?}"))?
        {
            stored.insert(change.track, change.after);
        }
        assert_eq!(stored.get(counter.track), Some(&1));
        assert_eq!(
            stored.get(counter.completion_track().ok_or("completion track")?),
            Some(&1)
        );
        assert!(!active(&stored).contains(&counter.transition));
        assert!(catalogue.evaluate(transition, &stored, 0).is_err());
        Ok(())
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

#[cfg(test)]
#[test]
fn great_dragon_hunt_loads_both_species_and_only_credits_source_areas() -> Result<(), String> {
    let loaded = super::loader::load_embedded_quest_state("great-dragon-hunt")
        .map_err(|error| format!("{error:?}"))?;
    for species in ["dragon", "dragon_lord"] {
        let key = format!("oteryn:quest-transition/native-kills/great-dragon-hunt/{species}");
        let counter = counter_for(loaded.catalogue(), &key).ok_or("native hunt counter")?;
        assert_eq!(counter.creature, format!("oteryn:creature.{species}"));
        assert_eq!(
            counter.track,
            "oteryn:quest-progress/quest/u10_80/the_great_dragon_hunt/dragon_counter"
        );
        assert_eq!(
            counter_for(loaded.catalogue(), &key).map(|c| c.maximum),
            Some(i64::from(i32::MAX))
        );
        assert!(
            active_for(loaded.catalogue(), &BTreeMap::new()).contains(&key),
            "Crystal credits kills before the explorer resets the count"
        );
        for position in [
            (33061, 32646, -6),
            (33027, 32634, -7),
            (32983, 32616, -7),
            (33007, 32612, -6),
            (32987, 32621, -6),
            (33002, 32614, -5),
            (32993, 32632, -7),
        ] {
            assert!(super::world_change_journals::admits_position(
                &key,
                Some(position)
            ));
        }
        for position in [
            None,
            Some((33061, 32646, -8)),
            Some((32982, 32616, -7)),
            Some((33061, 32627, -6)),
        ] {
            assert!(!super::world_change_journals::admits_position(
                &key, position
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn demon_wars_trial_requires_briefing_and_twenty_one_one_before_report() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let mut tracks = BTreeMap::new();
    let callbacks = super::reward_callbacks::load_for(catalogue)?;
    let find_callback = |key| {
        callbacks
            .iter()
            .find(|callback| callback.key == key && callback.npc == "oteryn:npc.a_dark_priestess")
            .ok_or("Dark Priestess callback")
    };
    let briefing = find_callback("oteryn:quest-reward/demon-wars/briefing")?;
    let report = find_callback("oteryn:quest-reward/demon-wars/report")?;
    let briefing = catalogue
        .transition(&briefing.key)
        .ok_or("briefing transition")?;
    let report = catalogue
        .transition(&report.key)
        .ok_or("report transition")?;
    let kills = [("demon", 20), ("lord", 1), ("prince", 1)];
    let key = |creature| format!("oteryn:quest-transition/native-kills/demon-wars/{creature}");
    let evaluate = |transition, tracks: &mut BTreeMap<String, i64>| {
        for change in catalogue
            .evaluate(transition, tracks, 0)
            .map_err(|error| format!("{error:?}"))?
        {
            tracks.insert(change.track, change.after);
        }
        Ok::<_, String>(())
    };
    assert!(catalogue.evaluate(report, &tracks, 0).is_err());
    assert!(!active_for(catalogue, &tracks).contains(&key("demon")));
    evaluate(briefing, &mut tracks)?;
    for (creature, count) in kills {
        let transition_key = key(creature);
        let transition = catalogue
            .transition(&transition_key)
            .ok_or("kill transition")?;
        let counter = counter_for(catalogue, &transition_key).ok_or("native counter")?;
        assert_eq!(
            counter.creature,
            format!("oteryn:creature.shaburak_{creature}")
        );
        let area = counter.death_area.ok_or("Demonwar Dungeon bounds")?;
        assert!(area.contains([33260, 31930, 13]));
        assert!(!area.contains([33260, 31930, 7]));
        for _ in 0..count {
            assert!(catalogue.evaluate(report, &tracks, 0).is_err());
            assert!(active_for(catalogue, &tracks).contains(&transition_key));
            evaluate(transition, &mut tracks)?;
        }
        assert!(!active_for(catalogue, &tracks).contains(&transition_key));
    }
    assert!(report.completes);
    evaluate(report, &mut tracks)?;
    assert!(catalogue.evaluate(report, &tracks, 0).is_err());
    Ok(())
}
