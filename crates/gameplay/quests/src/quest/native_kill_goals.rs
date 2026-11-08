//! Bounded personal one-shot goals on native creature-death occurrences.
//! Phase -1 is inactive, 0 is active, and 1 is terminal. Only the named activation
//! may enter phase0; a later settlement can therefore never credit a restarted run.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub creature: String,
    pub x: u16,
    pub y: u16,
    pub floor: u8,
    source_pointer: String,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: String,
    sha256: String,
    #[serde(default)]
    lines: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prerequisite {
    pub counter: String,
    pub count: u16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub key: String,
    pub quest: String,
    pub phase: String,
    pub counter: String,
    pub activation: String,
    pub count: u16,
    pub origins: Vec<Origin>,
    #[serde(default)]
    pub requires: Vec<Prerequisite>,
    sources: Vec<Source>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u8,
    goals: Vec<Goal>,
}

impl Goal {
    #[must_use]
    pub fn active(&self, tracks: &BTreeMap<String, i64>) -> bool {
        tracks.get(&self.phase).copied().unwrap_or(-1) == 0
            && (0..i64::from(self.count)).contains(&tracks.get(&self.counter).copied().unwrap_or(0))
            && self.requires.iter().all(|required| {
                tracks.get(&required.counter).copied().unwrap_or(0) == i64::from(required.count)
            })
    }
    #[must_use]
    pub fn matches(&self, creature: &str, x: i32, y: i32, floor: i16) -> bool {
        self.origins.iter().any(|p| {
            p.creature == creature
                && i32::from(p.x) == x
                && i32::from(p.y) == y
                && i16::from(p.floor) == floor
        })
    }
}

pub fn load() -> Result<&'static [Goal], String> {
    static GOALS: OnceLock<Result<Vec<Goal>, String>> = OnceLock::new();
    GOALS
        .get_or_init(|| {
            let document: Document = serde_json::from_str(include_str!(
                "../../../../../tools/content-schema/quest-authoring/samples/native-kill-goals.json"
            ))
            .map_err(|e| e.to_string())?;
            if document.schema != 1 || document.goals.is_empty() || document.goals.len() > 16 {
                return Err("native kill goal catalogue bound".into());
            }
            let mut keys = BTreeSet::new();
            let mut counters = BTreeSet::new();
            for goal in &document.goals {
                let origins: BTreeSet<_> = goal
                    .origins
                    .iter()
                    .map(|p| (&p.creature, p.x, p.y, p.floor))
                    .collect();
                if !goal.key.starts_with("oteryn:quest-transition/native-kill/")
                    || [
                        &goal.key,
                        &goal.quest,
                        &goal.phase,
                        &goal.counter,
                        &goal.activation,
                    ]
                    .iter()
                    .any(|k| !super::valid_quest_key(k))
                    || !keys.insert(&goal.key)
                    || !counters.insert(&goal.counter)
                    || goal.phase == goal.counter
                    || !(1..=1000).contains(&goal.count)
                    || goal.origins.is_empty()
                    || goal.origins.len() > 256
                    || origins.len() != goal.origins.len()
                    || goal.origins.iter().any(|p| {
                        p.floor > 15
                            || !p.creature.starts_with("oteryn:creature.")
                            || !super::valid_quest_key(&p.creature)
                            || p.source_pointer.len() > 256
                            || !p.source_pointer.starts_with("/records/")
                    })
                    || goal.sources.is_empty()
                    || goal.sources.len() > 8
                    || goal.sources.iter().any(|s| {
                        s.path.is_empty()
                            || s.path.len() > 256
                            || s.sha256.len() != 64
                            || !s.sha256.bytes().all(|b| b.is_ascii_hexdigit())
                            || s.lines
                                .as_ref()
                                .is_some_and(|l| l.is_empty() || l.len() > 64)
                    })
                {
                    return Err(format!("invalid native kill goal {}", goal.key));
                }
            }
            // Every prerequisite is another complete goal of this same quest.
            // Resolve the bounded graph before exposing any goal; cycles fail closed.
            let mut resolved = BTreeSet::new();
            for goal in &document.goals {
                let required: BTreeSet<_> = goal.requires.iter().map(|p| &p.counter).collect();
                if goal.requires.len() > 16
                    || required.len() != goal.requires.len()
                    || goal.requires.iter().any(|p| {
                        !document.goals.iter().any(|other| {
                            other.quest == goal.quest
                                && other.counter == p.counter
                                && other.counter != goal.counter
                                && other.count == p.count
                        })
                    })
                {
                    return Err(format!("invalid native goal prerequisite {}", goal.key));
                }
            }
            for _ in 0..document.goals.len() {
                for goal in &document.goals {
                    if goal.requires.iter().all(|p| resolved.contains(&p.counter)) {
                        resolved.insert(goal.counter.clone());
                    }
                }
            }
            if resolved.len() != document.goals.len() {
                return Err("cyclic native goal prerequisites".into());
            }
            Ok(document.goals)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for goal in load()? {
        for (key, initial, min, max) in [
            (&goal.phase, -1, -1, 1),
            (&goal.counter, 0, 0, i64::from(goal.count)),
        ] {
            if let Some(existing) = tracks.iter().find(|t| &t.key == key) {
                if existing.quest != goal.quest
                    || existing.initial != initial
                    || existing.min != min
                    || existing.max != max
                {
                    return Err(format!("conflicting native goal track {key}"));
                }
            } else {
                tracks.push(QuestTrack {
                    key: key.clone(),
                    quest: goal.quest.clone(),
                    initial,
                    min,
                    max,
                });
            }
        }
        let prerequisites = goal.requires.iter().map(|required| QuestEffect {
            track: required.counter.clone(),
            from: QuestComparison::Eq(i64::from(required.count)),
            effect: QuestEffectKind::Add(0),
        });
        transitions.push(QuestTransition {
            world_change: None,
            key: goal.key.clone(),
            quest: goal.quest.clone(),
            effects: vec![
                QuestEffect {
                    track: goal.phase.clone(),
                    from: QuestComparison::Eq(0),
                    effect: QuestEffectKind::Add(0),
                },
                QuestEffect {
                    track: goal.counter.clone(),
                    from: QuestComparison::Between(0, i64::from(goal.count) - 1),
                    effect: QuestEffectKind::Add(1),
                },
            ]
            .into_iter()
            .chain(prerequisites)
            .collect(),
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

/// Reject any loaded NPC/world transition that can reopen a run or manufacture kills.
pub fn validate_closed(catalogue: &QuestStateCatalogue) -> Result<(), String> {
    for goal in load()? {
        let activation = catalogue
            .transition(&goal.activation)
            .ok_or("missing native goal activation")?;
        if !activation.effects.iter().any(|e| {
            e.track == goal.phase
                && e.from == QuestComparison::Between(-1, -1)
                && e.effect == QuestEffectKind::Set(0)
        }) {
            return Err("native goal activation must be once-only".into());
        }
        for t in catalogue.transitions.values() {
            for e in &t.effects {
                // Computed contract markers cannot execute or mutate a track.
                if e.effect == QuestEffectKind::Computed {
                    continue;
                }
                if e.track == goal.phase {
                    let valid = match (e.from, e.effect) {
                        (QuestComparison::Between(-1, -1), QuestEffectKind::Set(0)) => {
                            t.key == goal.activation
                        }
                        (
                            QuestComparison::Eq(0)
                            | QuestComparison::Eq(1)
                            | QuestComparison::Between(0, 0)
                            | QuestComparison::Between(0, 1)
                            | QuestComparison::Between(1, 1),
                            QuestEffectKind::Add(0),
                        ) => true,
                        (
                            QuestComparison::Eq(0) | QuestComparison::Between(0, 0),
                            QuestEffectKind::Set(1),
                        ) => true,
                        _ => false,
                    };
                    if !valid {
                        return Err(format!("native goal run may reopen: {}", t.key));
                    }
                }
                if e.track == goal.counter
                    && !(e.effect == QuestEffectKind::Add(0)
                        || (t.key == goal.key
                            && e.effect == QuestEffectKind::Add(1)
                            && e.from == QuestComparison::Between(0, i64::from(goal.count) - 1)))
                {
                    return Err(format!(
                        "native kill counter has a foreign writer: {}",
                        t.key
                    ));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_origins_and_one_shot_counter_bounds() -> Result<(), String> {
        let goal = load()?
            .iter()
            .find(|g| g.counter == "oteryn:quest-progress/elvenbane/native-defenders/dragon")
            .ok_or("goal")?;
        assert!(goal.matches("oteryn:creature.dragon", 32590, 31645, 5));
        for (creature, x, y, floor) in [
            ("oteryn:creature.dragon_lord", 32590, 31645, 5),
            ("oteryn:creature.dragon", 32589, 31645, 5),
            ("oteryn:creature.dragon", 32590, 31645, 4),
        ] {
            assert!(!goal.matches(creature, x, y, floor));
        }
        let mut tracks = BTreeMap::new();
        assert!(!goal.active(&tracks));
        tracks.insert(goal.phase.clone(), 0);
        assert!(goal.active(&tracks));
        tracks.insert(goal.counter.clone(), i64::from(goal.count));
        assert!(!goal.active(&tracks));
        tracks.insert(goal.counter.clone(), 0);
        tracks.insert(goal.phase.clone(), 1);
        assert!(!goal.active(&tracks));
        Ok(())
    }

    #[test]
    fn loaded_counter_rejects_foreign_writes_and_reopened_runs() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let goal = load()?
            .iter()
            .find(|g| g.counter == "oteryn:quest-progress/elvenbane/native-defenders/dragon")
            .ok_or("goal")?;
        validate_closed(lowered.catalogue())?;
        for track in [&goal.phase, &goal.counter] {
            let mut catalogue = lowered.catalogue().clone();
            let marker = QuestTransition {
                world_change: None,
                key: "oteryn:quest-transition/fixture-native-goal-marker".into(),
                quest: goal.quest.clone(),
                effects: vec![QuestEffect {
                    track: track.clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
            };
            catalogue
                .transitions
                .insert(marker.key.clone(), marker.clone());
            validate_closed(&catalogue)?;
            assert!(matches!(
                catalogue.evaluate(&marker, &BTreeMap::new(), 0),
                Err(super::super::QuestRefusal::NotSupported)
            ));
        }
        for (track, from, effect) in [
            (&goal.phase, QuestComparison::Eq(1), QuestEffectKind::Set(0)),
            (
                &goal.counter,
                QuestComparison::Any,
                QuestEffectKind::Set(i64::from(goal.count)),
            ),
        ] {
            let mut catalogue = lowered.catalogue().clone();
            let key = "oteryn:quest-transition/fixture-forbidden-native-goal".to_string();
            catalogue.transitions.insert(
                key.clone(),
                QuestTransition {
                    world_change: None,
                    key,
                    quest: goal.quest.clone(),
                    effects: vec![QuestEffect {
                        track: track.clone(),
                        from,
                        effect,
                    }],
                    completes: false,
                    experience: None,
                },
            );
            assert!(validate_closed(&catalogue).is_err());
        }
        Ok(())
    }
}
