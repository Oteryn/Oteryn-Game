//! Explicit source-qualified successful-hit objectives, independent of creature death.
use super::{BoundQuest, Trigger};
use crate::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PREFIX: &str = "oteryn:quest-transition/native-hits/";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HitSource {
    Weapon { item: String },
    Rune { item: String, spell: String },
}
impl HitSource {
    fn valid(&self) -> bool {
        let (Self::Weapon { item } | Self::Rune { item, .. }) = self;
        crate::quest::valid_quest_key(item)
            && item.starts_with("oteryn:item.")
            && match self {
                Self::Weapon { .. } => true,
                Self::Rune { spell, .. } => {
                    spell.len() <= crate::quest::QUESTSTATE0_RL_06
                        && (spell.starts_with("candidate:spell/")
                            || spell.starts_with("oteryn:spell."))
                        && !spell.ends_with('/')
                        && spell
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
                }
            }
    }
    pub fn matches(&self, item: &str, spell: Option<&str>) -> bool {
        match self {
            Self::Weapon { item: expected } => item == expected && spell.is_none(),
            Self::Rune {
                item: expected,
                spell: expected_spell,
            } => item == expected && spell == Some(expected_spell.as_str()),
        }
    }
}

/// One successful hit per mission phase. Damage magnitude never selects progress.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HitObjective {
    pub key: String,
    pub creature: String,
    pub source: HitSource,
    pub track: String,
    pub gate: String,
    pub gate_value: i64,
    pub advance_gate_to: i64,
}
impl HitObjective {
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        if !crate::quest::valid_quest_key(&self.key)
            || !self.key.starts_with(PREFIX)
            || !crate::quest::valid_quest_key(&self.creature)
            || !self.creature.starts_with("oteryn:creature.")
            || !self.source.valid()
            || self.track == self.gate
            || self.gate_value == self.advance_gate_to
            || quest
                .tracks
                .iter()
                .find(|track| track.key == self.track)
                .is_none_or(|track| track.initial != 0 || track.minimum > 0 || track.maximum < 1)
            || quest
                .tracks
                .iter()
                .find(|track| track.key == self.gate)
                .is_none_or(|track| {
                    !(track.minimum..=track.maximum).contains(&self.gate_value)
                        || !(track.minimum..=track.maximum).contains(&self.advance_gate_to)
                })
        {
            return Err("authored hit identity/source/owner/bounds".into());
        }
        Ok(())
    }
    pub fn matches(&self, creature: &str, item: &str, spell: Option<&str>) -> bool {
        creature == self.creature && self.source.matches(item, spell)
    }
    pub fn active(&self, catalogue: &QuestStateCatalogue, tracks: &BTreeMap<String, i64>) -> bool {
        let observed = |key: &str| {
            tracks
                .get(key)
                .copied()
                .or_else(|| catalogue.track(key).map(|track| track.initial))
        };
        observed(&self.gate) == Some(self.gate_value) && observed(&self.track) == Some(0)
    }
    pub(super) fn transition(&self, quest: &str, completes: bool) -> QuestTransition {
        QuestTransition {
            key: self.key.clone(),
            quest: quest.into(),
            completes,
            experience: None,
            world_change: None,
            effects: vec![
                QuestEffect {
                    track: self.gate.clone(),
                    from: QuestComparison::Eq(self.gate_value),
                    effect: QuestEffectKind::Set(self.advance_gate_to),
                },
                QuestEffect {
                    track: self.track.clone(),
                    from: QuestComparison::Eq(0),
                    effect: QuestEffectKind::Set(1),
                },
            ],
        }
    }
}

pub fn objectives(catalogue: &QuestStateCatalogue) -> impl Iterator<Item = (&str, &HitObjective)> {
    catalogue
        .authored_bindings()
        .into_iter()
        .flat_map(|bindings| &bindings.quests)
        .flat_map(|quest| {
            quest
                .stages
                .iter()
                .filter_map(move |stage| match &stage.trigger {
                    Trigger::Hit { objective } => Some((quest.quest.as_str(), objective.as_ref())),
                    _ => None,
                })
        })
}

pub fn objective_for<'a>(
    catalogue: &'a QuestStateCatalogue,
    key: &str,
) -> Option<(&'a str, &'a HitObjective)> {
    let transition = catalogue.transition(key)?;
    objectives(catalogue)
        .find(|(quest, objective)| objective.key == key && *quest == transition.quest)
}
