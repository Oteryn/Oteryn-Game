//! Bounded source state operations reuse the fenced Quest evaluator.
use super::super::{QuestComparison, QuestEffect, QuestEffectKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StateCondition {
    Any,
    Between { minimum: i64, maximum: i64 },
    ElapsedAtLeast { seconds: u32 },
    ElapsedLessThan { seconds: u32 },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StateMutation {
    Add { value: i64 },
    SetNow,
}

// Serde's internally tagged unit variants discard remaining fields even with
// deny_unknown_fields. Empty struct wire variants enforce the same closed shape
// as SQL while preserving the public unit variants and their serialized bytes.
impl<'de> Deserialize<'de> for StateCondition {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Any {},
            Between { minimum: i64, maximum: i64 },
            ElapsedAtLeast { seconds: u32 },
            ElapsedLessThan { seconds: u32 },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Any {} => Self::Any,
            Wire::Between { minimum, maximum } => Self::Between { minimum, maximum },
            Wire::ElapsedAtLeast { seconds } => Self::ElapsedAtLeast { seconds },
            Wire::ElapsedLessThan { seconds } => Self::ElapsedLessThan { seconds },
        })
    }
}

impl<'de> Deserialize<'de> for StateMutation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Add { value: i64 },
            SetNow {},
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Add { value } => Self::Add { value },
            Wire::SetNow {} => Self::SetNow,
        })
    }
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
            StateCondition::ElapsedAtLeast { seconds }
            | StateCondition::ElapsedLessThan { seconds } => {
                (1..=366 * 24 * 60 * 60).contains(&seconds)
            }
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
            StateCondition::ElapsedLessThan { .. } => value > 0,
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
                StateCondition::ElapsedLessThan { seconds } => {
                    QuestComparison::ElapsedLessThan(i64::from(seconds))
                }
            },
            effect: match self.effect {
                StateMutation::Add { value } => QuestEffectKind::Add(value),
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
    #[test]
    fn deadlines_reject_uninitialized_future_and_exact_expiry() -> Result<(), String> {
        let deadline = StateOperation {
            track: "oteryn:clock".into(),
            from: StateCondition::ElapsedLessThan { seconds: 1800 },
            effect: StateMutation::Add { value: 0 },
        };
        assert!(deadline.valid());
        let comparison = deadline.quest_effect().from;
        for (stamp, now, expected) in [
            (0, 5000, false),
            (-1, 5000, false),
            (5001, 5000, false),
            (5000, 5000, true),
            (5000, 6799, true),
            (5000, 6800, false),
            (5000, 6801, false),
            (i64::MAX, i64::MAX, true),
            (1, i64::MAX, false),
        ] {
            assert_eq!(
                comparison.holds(stamp, now),
                expected,
                "stamp={stamp}, now={now}"
            );
        }
        for seconds in [0, 366 * 24 * 60 * 60 + 1, u32::MAX] {
            let mut invalid = deadline.clone();
            invalid.from = StateCondition::ElapsedLessThan { seconds };
            assert!(!invalid.valid());
        }
        let mut bytes = Vec::new();
        QuestComparison::ElapsedLessThan(1800).encode(&mut bytes);
        assert_eq!(bytes[0], 9, "new comparison has its own tag");
        let mut legacy = Vec::new();
        QuestComparison::ElapsedAtLeast(1800).encode(&mut legacy);
        assert_eq!(legacy[0], 8, "legacy comparison encoding stays stable");
        for invalid in [
            r#"{"track":"oteryn:clock","from":{"kind":"elapsed_less_than","seconds":1800,"clock":"client"},"effect":{"kind":"add","value":0}}"#,
            r#"{"track":"oteryn:clock","from":{"kind":"elapsed_less_than","seconds":-1},"effect":{"kind":"add","value":0}}"#,
            r#"{"track":"oteryn:clock","from":{"kind":"any"},"effect":{"kind":"set_now","value":1}}"#,
            r#"{"track":"oteryn:clock","from":{"kind":"any","clock":"client"},"effect":{"kind":"add","value":0}}"#,
        ] {
            assert!(serde_json::from_str::<StateOperation>(invalid).is_err());
        }
        assert_eq!(
            serde_json::to_string(&StateCondition::Any).map_err(|error| error.to_string())?,
            r#"{"kind":"any"}"#
        );
        assert_eq!(
            serde_json::to_string(&StateMutation::SetNow).map_err(|error| error.to_string())?,
            r#"{"kind":"set_now"}"#
        );
        assert_eq!(
            serde_json::from_str::<StateCondition>(r#"{"kind":"any"}"#)
                .map_err(|error| error.to_string())?,
            StateCondition::Any
        );
        assert_eq!(
            serde_json::from_str::<StateMutation>(r#"{"kind":"set_now"}"#)
                .map_err(|error| error.to_string())?,
            StateMutation::SetNow
        );
        Ok(())
    }
}
