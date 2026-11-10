//! Bounded, once-only authored kill objectives over the existing quest writer.
use super::world_callbacks::Position;
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeathObjective {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guardian: Option<GuardianSpawn>,
    pub key: String,
    pub quest: String,
    pub creature: String,
    pub stage: String,
    pub stage_value: i64,
    pub counter: String,
    pub required: u16,
    pub minimum: Position,
    pub maximum: Position,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GuardianSpawn {
    pub position: Position,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_positions: Vec<Position>,
    pub respawn_ms: u32,
}

impl GuardianSpawn {
    pub fn positions(&self) -> impl Iterator<Item = Position> + '_ {
        std::iter::once(self.position).chain(self.additional_positions.iter().copied())
    }

    fn valid_for(&self, objective: &DeathObjective) -> bool {
        self.additional_positions.len() <= 7
            && (60_000..=3_600_000).contains(&self.respawn_ms)
            && self.positions().collect::<BTreeSet<_>>().len()
                == 1 + self.additional_positions.len()
            && self.positions().all(|position| {
                objective.matches(
                    &objective.creature,
                    i32::from(position.x),
                    i32::from(position.y),
                    -i16::from(position.floor),
                )
            })
    }
}

impl DeathObjective {
    pub fn active(&self, tracks: &BTreeMap<String, i64>) -> bool {
        tracks.get(&self.stage) == Some(&self.stage_value)
            && tracks
                .get(&self.counter)
                .is_some_and(|count| (0..i64::from(self.required)).contains(count))
    }

    pub fn matches(&self, creature: &str, x: i32, y: i32, floor: i16) -> bool {
        self.creature == creature
            && floor == -i16::from(self.minimum.floor)
            && (i32::from(self.minimum.x)..=i32::from(self.maximum.x)).contains(&x)
            && (i32::from(self.minimum.y)..=i32::from(self.maximum.y)).contains(&y)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    objectives: Vec<DeathObjective>,
}

/// Static objective descriptions; independent from the native live actor budget.
pub const MAX_OBJECTIVES: usize = 128;

fn parse(text: &str) -> Result<Vec<DeathObjective>, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut keys = BTreeSet::new();
    if document.schema != 1 || document.objectives.len() > MAX_OBJECTIVES {
        return Err("death objective bounds".into());
    }
    for objective in &document.objectives {
        if !objective.key.starts_with("oteryn:quest-death/")
            || [
                &objective.key,
                &objective.quest,
                &objective.stage,
                &objective.counter,
            ]
            .iter()
            .any(|key| !super::valid_quest_key(key))
            || !keys.insert(&objective.key)
            || !objective.creature.starts_with("oteryn:creature.")
            || !super::valid_quest_key(&objective.creature)
            || objective
                .creature
                .strip_prefix("oteryn:creature.")
                .is_none_or(str::is_empty)
            || objective.stage == objective.counter
            || objective.required == 0
            || !(-15..=0).contains(&objective.minimum.floor)
            || objective.minimum.floor != objective.maximum.floor
            || objective.maximum.x < objective.minimum.x
            || objective.maximum.y < objective.minimum.y
            || objective.maximum.x - objective.minimum.x > 64
            || objective.maximum.y - objective.minimum.y > 64
            || objective
                .guardian
                .as_ref()
                .is_some_and(|guardian| !guardian.valid_for(objective))
            || objective.sources.is_empty()
            || objective.sources.len() > 8
            || objective
                .sources
                .iter()
                .any(|source| source.is_empty() || source.len() > 1024)
        {
            return Err(format!("invalid death objective {}", objective.key));
        }
    }
    Ok(document.objectives)
}

pub fn load() -> Result<&'static [DeathObjective], String> {
    static DATA: OnceLock<Result<Vec<DeathObjective>, String>> = OnceLock::new();
    DATA.get_or_init(|| {
        let mut objectives: Vec<DeathObjective> = Vec::new();
        let mut keys = BTreeSet::new();
        for source in [
            include_str!("../../../../../content/quests/authored-death-objectives.json"),
            include_str!("../../../../../content/quests/questpc7-death-objectives.json"),
        ] {
            for objective in parse(source)? {
                if !keys.insert(objective.key.clone()) {
                    // QuestPC7 explicitly consolidates the existing smith cache.
                    // Only its provenance changes; never replace another plan.
                    let previous = objectives
                        .iter_mut()
                        .find(|row| row.key == objective.key)
                        .ok_or("missing duplicate objective")?;
                    let mut before = previous.clone();
                    let mut after = objective.clone();
                    before.sources.clear();
                    after.sources.clear();
                    if objective.key != "oteryn:quest-death/authored-smith-guardian"
                        || before != after
                        || !objective.sources.iter().any(|source| {
                            source.starts_with(
                                "reused:questpc8:3b315d18dbb0a3a00149a2a14b9a19fcb42053a1:",
                            )
                        })
                    {
                        return Err("conflicting death objective catalogue".into());
                    }
                    *previous = objective;
                    continue;
                }
                if objectives.len() >= MAX_OBJECTIVES {
                    return Err("duplicate or oversized death objective catalogue".into());
                }
                objectives.push(objective);
            }
        }
        Ok(objectives)
    })
    .as_ref()
    .map(Vec::as_slice)
    .map_err(Clone::clone)
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for objective in load()? {
        let Some(stage) = tracks.iter().find(|track| track.key == objective.stage) else {
            return Err("missing objective stage".into());
        };
        if stage.quest != objective.quest
            || !(stage.min..=stage.max).contains(&objective.stage_value)
        {
            return Err("foreign objective stage".into());
        }
        let Some(counter) = tracks
            .iter_mut()
            .find(|track| track.key == objective.counter)
        else {
            return Err(format!(
                "{}: missing objective counter {}",
                objective.key, objective.counter
            ));
        };
        if counter.quest != objective.quest {
            return Err("foreign objective counter".into());
        }
        counter.max = counter.max.max(i64::from(objective.required));
        let digest: String =
            Sha256::digest(serde_json::to_vec(objective).map_err(|e| e.to_string())?)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
        transitions.push(QuestTransition {
            key: format!("oteryn:quest-death-plan/{digest}"),
            quest: objective.quest.clone(),
            effects: vec![QuestEffect {
                track: objective.stage.clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
            world_change: None,
        });
        transitions.push(QuestTransition {
            key: objective.key.clone(),
            quest: objective.quest.clone(),
            completes: false,
            experience: None,
            world_change: None,
            effects: vec![
                QuestEffect {
                    track: objective.stage.clone(),
                    from: QuestComparison::Eq(objective.stage_value),
                    effect: QuestEffectKind::Add(0),
                },
                QuestEffect {
                    track: objective.counter.clone(),
                    from: QuestComparison::Between(0, i64::from(objective.required) - 1),
                    effect: QuestEffectKind::Add(1),
                },
            ],
        });
    }
    let mut extended = QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))?;
    extended.npc_legacy = catalogue.npc_legacy;
    extended.predecessors = catalogue.predecessors;
    extended.authored_bindings = catalogue.authored_bindings;
    extended.native_recipe_evidence = catalogue.native_recipe_evidence;
    Ok(extended)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merged_catalogue_retains_both_authored_and_questpc7_guardians() -> Result<(), String> {
        let objectives = load()?;
        for key in [
            "oteryn:quest-death/authored-anniversary-cyclops",
            "oteryn:quest-death/questpc7-hydra-guardian",
        ] {
            assert!(
                objectives
                    .iter()
                    .any(|row| row.key == key && row.guardian.is_some())
            );
        }
        assert_eq!(objectives.len(), 61);
        assert_eq!(
            objectives
                .iter()
                .filter(|row| row.key == "oteryn:quest-death/authored-smith-guardian")
                .count(),
            1
        );
        Ok(())
    }
    fn fixture(count: usize) -> Result<serde_json::Value, String> {
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/questpc7-death-objectives.json"
        ))
        .map_err(|error| error.to_string())?;
        let rows: Vec<_> = (0..count)
            .map(|i| {
                let mut row = source["objectives"][0].clone();
                row["key"] = format!("oteryn:quest-death/catalogue-test-{i}").into();
                row
            })
            .collect();
        Ok(serde_json::json!({"schema":1,"objectives":rows}))
    }
    #[test]
    fn static_catalogue_has_its_own_closed_bound() -> Result<(), String> {
        assert_eq!(
            parse(&fixture(MAX_OBJECTIVES)?.to_string())?.len(),
            MAX_OBJECTIVES
        );
        assert!(parse(&fixture(MAX_OBJECTIVES + 1)?.to_string()).is_err());
        Ok(())
    }
    #[test]
    fn expanded_catalogue_preserves_every_spatial_credit_and_source_guard() -> Result<(), String> {
        for case in 0..9 {
            let mut document = fixture(2)?;
            match case {
                0 => document["schema"] = 2.into(),
                1 => document["objectives"][1]["key"] = document["objectives"][0]["key"].clone(),
                2 => document["objectives"][0]["required"] = 0.into(),
                3 => {
                    document["objectives"][0]["counter"] =
                        document["objectives"][0]["stage"].clone()
                }
                4 => document["objectives"][0]["guardian"]["respawn_ms"] = 59_999.into(),
                5 => document["objectives"][0]["guardian"]["respawn_ms"] = 3_600_001.into(),
                6 => {
                    document["objectives"][0]["maximum"]["x"] =
                        (document["objectives"][0]["minimum"]["x"]
                            .as_u64()
                            .ok_or("coordinate")?
                            + 65)
                            .into()
                }
                7 => document["objectives"][0]["sources"] = serde_json::json!([]),
                _ => document["objectives"][0]["maximum"]["floor"] = 1.into(),
            }
            assert!(parse(&document.to_string()).is_err(), "case {case}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod guardian_tests {
    use super::*;

    #[test]
    fn bounded_guardian_groups_reject_duplicate_foreign_and_outside_cells() -> Result<(), String> {
        let objective: DeathObjective = serde_json::from_value(serde_json::json!({
            "key":"oteryn:quest-death/test", "quest":"oteryn:quest.test",
            "creature":"oteryn:creature.demon", "stage":"oteryn:stage",
            "stage_value":1,"counter":"oteryn:counter","required":8,
            "minimum":{"x":0,"y":0,"floor":-1},"maximum":{"x":10,"y":10,"floor":-1},
            "sources":["oteryn-authored:test"],
            "guardian":{"position":{"x":1,"y":1,"floor":-1},
                "additional_positions":[{"x":2,"y":1,"floor":-1}],"respawn_ms":90000}
        }))
        .map_err(|e| e.to_string())?;
        let mut guardian = objective.guardian.clone().ok_or("guardian")?;
        assert!(guardian.valid_for(&objective));
        guardian.additional_positions = (2..=8).map(|x| Position { x, y: 1, floor: -1 }).collect();
        assert!(guardian.valid_for(&objective));
        guardian.additional_positions.push(Position {
            x: 9,
            y: 1,
            floor: -1,
        });
        assert!(!guardian.valid_for(&objective));
        for position in [
            guardian.position,
            Position {
                x: 11,
                y: 1,
                floor: -1,
            },
            Position {
                x: 2,
                y: 1,
                floor: -2,
            },
        ] {
            guardian.additional_positions = vec![position];
            assert!(!guardian.valid_for(&objective));
        }
        guardian.additional_positions = vec![
            Position {
                x: 2,
                y: 1,
                floor: -1
            };
            2
        ];
        assert!(!guardian.valid_for(&objective));
        guardian.additional_positions.clear();
        assert!(guardian.valid_for(&objective));
        assert!(
            serde_json::to_value(&guardian)
                .map_err(|e| e.to_string())?
                .get("additional_positions")
                .is_none(),
            "old single-guardian definition hashes remain stable"
        );
        Ok(())
    }
}
