//! Bounded source state operations reuse the fenced Quest evaluator.
use super::super::{QuestComparison, QuestEffect, QuestEffectKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateCondition {
    Any,
    Between { minimum: i64, maximum: i64 },
    ElapsedAtLeast { seconds: u32 },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateMutation {
    Add { value: i64 },
    AddAtLeast { value: i64, minimum: i64 },
    SetNow,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StateOperation {
    pub track: String,
    pub from: StateCondition,
    pub effect: StateMutation,
}

impl StateOperation {
    pub fn valid(&self) -> bool {
        let condition = match self.from {
            StateCondition::Any => true,
            StateCondition::Between { minimum, maximum } => minimum <= maximum,
            StateCondition::ElapsedAtLeast { seconds } => seconds <= 366 * 24 * 60 * 60,
        };
        condition
            && self.track.len() <= 128
            && self.track.strip_prefix("oteryn:").is_some_and(|tail| {
                !tail.is_empty()
                    && tail.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"._/:-".contains(&b)
                    })
            })
    }
    /// Time remains authoritative in the SQL transaction, not the conversation clock.
    pub fn advisory_holds(
        &self,
        catalogue: &super::QuestStateCatalogue,
        stored: &std::collections::BTreeMap<String, i64>,
    ) -> bool {
        let Some(track) = catalogue.track(&self.track) else {
            return false;
        };
        let value = stored.get(&self.track).copied().unwrap_or(track.initial);
        match self.from {
            StateCondition::Any | StateCondition::ElapsedAtLeast { .. } => true,
            StateCondition::Between { minimum, maximum } => (minimum..=maximum).contains(&value),
        }
    }

    pub fn quest_effect(&self) -> QuestEffect {
        QuestEffect {
            track: self.track.clone(),
            from: match self.from {
                StateCondition::Any => QuestComparison::Any,
                StateCondition::Between { minimum, maximum } => {
                    QuestComparison::Between(minimum, maximum)
                }
                StateCondition::ElapsedAtLeast { seconds } => {
                    QuestComparison::ElapsedAtLeast(i64::from(seconds))
                }
            },
            effect: match self.effect {
                StateMutation::Add { value } => QuestEffectKind::Add(value),
                StateMutation::AddAtLeast { value, minimum } => QuestEffectKind::AddAtLeast {
                    delta: value,
                    minimum,
                },
                StateMutation::SetNow => QuestEffectKind::SetNow,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestRefusal, QuestStateCatalogue, QuestTrack, QuestTransition};
    use std::collections::BTreeMap;

    #[test]
    fn operations_use_checked_native_arithmetic_and_the_transaction_clock() -> Result<(), String> {
        let owner = "oteryn:quest.test";
        let add = StateOperation {
            track: "oteryn:counter".into(),
            from: StateCondition::Any,
            effect: StateMutation::Add { value: 1 },
        };
        let stamp = StateOperation {
            track: "oteryn:clock".into(),
            from: StateCondition::Any,
            effect: StateMutation::SetNow,
        };
        let wait = StateOperation {
            track: "oteryn:clock".into(),
            from: StateCondition::ElapsedAtLeast { seconds: 3600 },
            effect: StateMutation::Add { value: 0 },
        };
        let transition = |key: &str, effects| QuestTransition {
            world_change: None,
            key: key.into(),
            quest: owner.into(),
            effects,
            completes: false,
            experience: None,
        };
        let catalogue = QuestStateCatalogue::new(
            "test",
            vec![
                QuestTrack {
                    key: add.track.clone(),
                    quest: owner.into(),
                    initial: 0,
                    min: 0,
                    max: 5,
                },
                QuestTrack {
                    key: stamp.track.clone(),
                    quest: owner.into(),
                    initial: 0,
                    min: 0,
                    max: i64::MAX,
                },
            ],
            vec![
                transition(
                    "oteryn:start",
                    vec![add.quest_effect(), stamp.quest_effect()],
                ),
                transition("oteryn:wait", vec![wait.quest_effect()]),
            ],
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut stored = BTreeMap::from([(add.track.clone(), 4), (stamp.track.clone(), 0)]);
        let changes = catalogue
            .evaluate(
                catalogue.transition("oteryn:start").ok_or("start")?,
                &stored,
                1000,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            changes.iter().map(|c| c.after).collect::<Vec<_>>(),
            vec![5, 1000]
        );
        stored.insert(add.track, 5);
        assert_eq!(
            catalogue.evaluate(
                catalogue.transition("oteryn:start").ok_or("start")?,
                &stored,
                1000
            ),
            Err(QuestRefusal::OutOfRange)
        );
        stored.insert(stamp.track, 1000);
        assert_eq!(
            catalogue.evaluate(
                catalogue.transition("oteryn:wait").ok_or("wait")?,
                &stored,
                4599
            ),
            Err(QuestRefusal::StageMismatch)
        );
        let changes = catalogue
            .evaluate(
                catalogue.transition("oteryn:wait").ok_or("wait")?,
                &stored,
                4600,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(changes[0].after, 1000, "elapsed check preserves the clock");
        Ok(())
    }
}
