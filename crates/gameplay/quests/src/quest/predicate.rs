//! Read-only quest predicates (QUEST-STATE-0 §7, ruling A3; QUEST-PRED-1).
//!
//! A predicate reads the session's quest copy (the tracks and quest states loaded at admission
//! and advanced by each committed receipt) and never writes. It is advisory: the transition
//! writer re-checks every `from` comparison under lock.
//!
//! The closed kinds of §7: `track op value`, `elapsed(track) >= s`, `quest_completed(key)` (the
//! Character's own record), `account_completed(key)` (the Character's own completion, D46, or
//! the account's, D45, only while the ruleset enables account completion and the quest's pinned
//! revision declares `grant`), `level >= n` and
//! `holds_item(definition)`. The last two, and the account facts, come from their own owners
//! through [`QuestPredicateFacts`].

use super::{QuestComparison, QuestStateCatalogue};

/// One §7 predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestPredicate {
    /// `track op value`: the closed comparisons of §4 on the copy's value, or the track's
    /// declared initial value when the copy has no row.
    Track {
        track: String,
        comparison: QuestComparison,
    },
    /// `elapsed(track) >= s`: `now` minus the track value, in seconds.
    Elapsed { track: String, seconds: i64 },
    /// The Character's own completed quest state.
    QuestCompleted(String),
    /// The account's completion of the quest (D45), or the Character's own (D46).
    AccountCompleted(String),
    /// `level >= n`.
    LevelAtLeast(u32),
    /// The Character holds at least one Item of this definition.
    HoldsItem(String),
}

/// What a predicate reads: the session copy and the facts of the other owners.
pub trait QuestPredicateFacts {
    /// The copy's stored value of `track`; `None` when it has no row.
    fn track_value(&self, track: &str) -> Option<i64>;
    /// The Character's quest state for `quest` is completed.
    fn quest_completed(&self, quest: &str) -> bool;
    /// The ruleset of the Character's World enables account completion.
    fn account_completion_enabled(&self) -> bool;
    /// The revision `quest` is pinned to declares `grant`.
    fn quest_grants_account_completion(&self, quest: &str) -> bool;
    /// The account holds an `AccountQuestCompletion` for `quest` in this profile family.
    fn account_quest_completed(&self, quest: &str) -> bool;
    /// The Character's level, from its progression owner.
    fn level(&self) -> u32;
    /// The Character holds an Item of `definition`, from its inventory owner.
    fn holds_item(&self, definition: &str) -> bool;
    /// The clock `elapsed` reads, in Unix seconds.
    fn now(&self) -> i64;
}

impl QuestPredicate {
    /// Evaluate against `catalogue` (the Character's content revision) and `facts`. A track or
    /// quest the catalogue does not declare, a negative `elapsed` bound, or an inverted
    /// `in [a, b]` reads as false: a predicate fails closed.
    #[must_use]
    pub fn holds(&self, catalogue: &QuestStateCatalogue, facts: &impl QuestPredicateFacts) -> bool {
        match self {
            Self::Track { track, comparison } => {
                let valid = match *comparison {
                    QuestComparison::Between(low, high) => low <= high,
                    QuestComparison::ElapsedAtLeast(seconds) => seconds >= 0,
                    _ => true,
                };
                valid
                    && track_value(catalogue, facts, track)
                        .is_some_and(|value| comparison.holds(value, facts.now()))
            }
            Self::Elapsed { track, seconds } => {
                *seconds >= 0
                    && track_value(catalogue, facts, track).is_some_and(|value| {
                        QuestComparison::ElapsedAtLeast(*seconds).holds(value, facts.now())
                    })
            }
            Self::QuestCompleted(quest) => {
                declared(catalogue, quest) && facts.quest_completed(quest)
            }
            // The policy gates only the shared account fact; the own completion always counts.
            Self::AccountCompleted(quest) => {
                declared(catalogue, quest)
                    && (facts.quest_completed(quest)
                        || (facts.account_completion_enabled()
                            && facts.quest_grants_account_completion(quest)
                            && facts.account_quest_completed(quest)))
            }
            Self::LevelAtLeast(level) => facts.level() >= *level,
            Self::HoldsItem(definition) => facts.holds_item(definition),
        }
    }
}

/// The catalogue declares `quest`: it owns a track or a transition.
fn declared(catalogue: &QuestStateCatalogue, quest: &str) -> bool {
    catalogue.definition_hash(quest).is_some()
}

fn track_value(
    catalogue: &QuestStateCatalogue,
    facts: &impl QuestPredicateFacts,
    track: &str,
) -> Option<i64> {
    let declared = catalogue.track(track)?;
    Some(facts.track_value(track).unwrap_or(declared.initial))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::super::QuestTrack;
    use super::*;

    const QUEST: &str = "oteryn:quest/test.rats";
    const STAGE: &str = "oteryn:quest-progress/test.rats.stage";
    const TIMER: &str = "oteryn:quest-progress/test.rats.timer";
    const UNDECLARED: &str = "oteryn:quest-progress/test.rats.none";
    const SWORD: &str = "oteryn:item/sword";

    #[derive(Default)]
    struct Facts {
        tracks: BTreeMap<String, i64>,
        completed: BTreeSet<String>,
        account_enabled: bool,
        grants: BTreeSet<String>,
        account_completed: BTreeSet<String>,
        level: u32,
        items: BTreeSet<String>,
        now: i64,
    }

    impl QuestPredicateFacts for Facts {
        fn track_value(&self, track: &str) -> Option<i64> {
            self.tracks.get(track).copied()
        }
        fn quest_completed(&self, quest: &str) -> bool {
            self.completed.contains(quest)
        }
        fn account_completion_enabled(&self) -> bool {
            self.account_enabled
        }
        fn quest_grants_account_completion(&self, quest: &str) -> bool {
            self.grants.contains(quest)
        }
        fn account_quest_completed(&self, quest: &str) -> bool {
            self.account_completed.contains(quest)
        }
        fn level(&self) -> u32 {
            self.level
        }
        fn holds_item(&self, definition: &str) -> bool {
            self.items.contains(definition)
        }
        fn now(&self) -> i64 {
            self.now
        }
    }

    fn catalogue() -> QuestStateCatalogue {
        let track = |key: &str, initial, min, max| QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial,
            min,
            max,
        };
        QuestStateCatalogue::new(
            "content-1",
            vec![track(STAGE, -1, -1, 10), track(TIMER, 0, 0, i64::MAX)],
            vec![],
        )
        .expect("catalogue")
    }

    fn on_track(comparison: QuestComparison) -> QuestPredicate {
        QuestPredicate::Track {
            track: STAGE.into(),
            comparison,
        }
    }

    fn elapsed(seconds: i64) -> QuestPredicate {
        QuestPredicate::Elapsed {
            track: TIMER.into(),
            seconds,
        }
    }

    fn with_stage(value: i64) -> Facts {
        Facts {
            tracks: BTreeMap::from([(STAGE.to_owned(), value)]),
            ..Facts::default()
        }
    }

    #[test]
    fn track_comparisons_read_the_copy_at_every_bound() {
        use QuestComparison as C;
        let catalogue = catalogue();
        for (comparison, value, holds) in [
            (C::Any, 3, true),
            (C::Eq(3), 3, true),
            (C::Eq(3), 4, false),
            (C::Ne(3), 3, false),
            (C::Ne(3), 4, true),
            (C::Lt(3), 2, true),
            (C::Lt(3), 3, false),
            (C::Le(3), 3, true),
            (C::Le(3), 4, false),
            (C::Gt(3), 3, false),
            (C::Gt(3), 4, true),
            (C::Ge(3), 3, true),
            (C::Ge(3), 2, false),
            (C::Between(2, 4), 1, false),
            (C::Between(2, 4), 2, true),
            (C::Between(2, 4), 4, true),
            (C::Between(2, 4), 5, false),
            (C::Between(3, 3), 3, true),
            (C::Between(4, 2), 3, false),
            (C::ElapsedAtLeast(-1), 3, false),
        ] {
            assert_eq!(
                on_track(comparison).holds(&catalogue, &with_stage(value)),
                holds,
                "{comparison:?} on {value}"
            );
        }
    }

    #[test]
    fn a_track_without_a_row_reads_its_declared_initial_value() {
        let catalogue = catalogue();
        let none = Facts::default();
        assert!(on_track(QuestComparison::Eq(-1)).holds(&catalogue, &none));
        assert!(!on_track(QuestComparison::Ge(0)).holds(&catalogue, &none));
        assert!(
            !on_track(QuestComparison::Eq(-1)).holds(&catalogue, &with_stage(0)),
            "a stored row wins over the initial value"
        );
    }

    #[test]
    fn an_undeclared_track_fails_closed() {
        let catalogue = catalogue();
        let mut facts = with_stage(1);
        facts.tracks.insert(UNDECLARED.into(), 1);
        for predicate in [
            QuestPredicate::Track {
                track: UNDECLARED.into(),
                comparison: QuestComparison::Any,
            },
            QuestPredicate::Elapsed {
                track: UNDECLARED.into(),
                seconds: 0,
            },
        ] {
            assert!(!predicate.holds(&catalogue, &facts), "{predicate:?}");
        }
    }

    #[test]
    fn elapsed_is_now_minus_the_value_at_its_bound() {
        let catalogue = catalogue();
        let at = |stored: i64, now: i64| Facts {
            tracks: BTreeMap::from([(TIMER.to_owned(), stored)]),
            now,
            ..Facts::default()
        };
        assert!(elapsed(60).holds(&catalogue, &at(1_000, 1_060)));
        assert!(!elapsed(60).holds(&catalogue, &at(1_000, 1_059)));
        assert!(elapsed(0).holds(&catalogue, &at(1_000, 1_000)));
        assert!(
            !elapsed(0).holds(&catalogue, &at(1_000, 999)),
            "a value in the future has not elapsed"
        );
        assert!(
            elapsed(1_522_018_605).holds(&catalogue, &at(0, 1_522_018_605)),
            "no row reads the initial 0"
        );
        assert!(
            elapsed(i64::MAX).holds(&catalogue, &at(0, i64::MAX)),
            "no overflow at the i64 bound"
        );
        assert!(!elapsed(i64::MAX).holds(&catalogue, &at(1, i64::MAX)));
        assert!(
            !elapsed(-1).holds(&catalogue, &at(1_000, 1_000)),
            "a negative bound fails closed"
        );
        assert!(
            QuestPredicate::Track {
                track: TIMER.into(),
                comparison: QuestComparison::ElapsedAtLeast(60),
            }
            .holds(&catalogue, &at(1_000, 1_060)),
            "the comparison form agrees"
        );
    }

    #[test]
    fn quest_completed_reads_the_characters_own_record() {
        let catalogue = catalogue();
        let predicate = QuestPredicate::QuestCompleted(QUEST.into());
        assert!(!predicate.holds(&catalogue, &Facts::default()));
        let account_only = Facts {
            account_enabled: true,
            grants: BTreeSet::from([QUEST.to_owned()]),
            account_completed: BTreeSet::from([QUEST.to_owned()]),
            ..Facts::default()
        };
        assert!(
            !predicate.holds(&catalogue, &account_only),
            "an account completion is not the Character's own"
        );
        let own = Facts {
            completed: BTreeSet::from([QUEST.to_owned()]),
            ..Facts::default()
        };
        assert!(predicate.holds(&catalogue, &own));
    }

    #[test]
    fn completion_of_an_undeclared_quest_fails_closed() {
        let catalogue = catalogue();
        let other = "oteryn:quest/test.retired";
        let all = BTreeSet::from([other.to_owned()]);
        let facts = Facts {
            completed: all.clone(),
            account_enabled: true,
            grants: all.clone(),
            account_completed: all,
            ..Facts::default()
        };
        for predicate in [
            QuestPredicate::QuestCompleted(other.into()),
            QuestPredicate::AccountCompleted(other.into()),
        ] {
            assert!(!predicate.holds(&catalogue, &facts), "{predicate:?}");
        }
    }

    #[test]
    fn account_completed_gates_only_the_account_fact() {
        let catalogue = catalogue();
        let predicate = QuestPredicate::AccountCompleted(QUEST.into());
        let facts = |enabled: bool, grant: bool, account: bool, own: bool| {
            let set = |on: bool| {
                if on {
                    BTreeSet::from([QUEST.to_owned()])
                } else {
                    BTreeSet::new()
                }
            };
            Facts {
                account_enabled: enabled,
                grants: set(grant),
                account_completed: set(account),
                completed: set(own),
                ..Facts::default()
            }
        };
        assert!(predicate.holds(&catalogue, &facts(true, true, true, false)));
        assert!(
            predicate.holds(&catalogue, &facts(true, true, false, true)),
            "D46: the Character's own completion is accepted"
        );
        assert!(!predicate.holds(&catalogue, &facts(true, true, false, false)));
        assert!(
            !predicate.holds(&catalogue, &facts(false, true, true, false)),
            "the ruleset disables account completion"
        );
        assert!(
            !predicate.holds(&catalogue, &facts(true, false, true, false)),
            "the pinned revision does not declare grant"
        );
        assert!(
            predicate.holds(&catalogue, &facts(false, false, false, true)),
            "the own completion counts outside the account policy"
        );
    }

    #[test]
    fn level_is_at_least_at_its_bound() {
        let catalogue = catalogue();
        let at = |level| Facts {
            level,
            ..Facts::default()
        };
        assert!(QuestPredicate::LevelAtLeast(8).holds(&catalogue, &at(8)));
        assert!(QuestPredicate::LevelAtLeast(8).holds(&catalogue, &at(9)));
        assert!(!QuestPredicate::LevelAtLeast(8).holds(&catalogue, &at(7)));
        assert!(QuestPredicate::LevelAtLeast(0).holds(&catalogue, &at(0)));
        assert!(QuestPredicate::LevelAtLeast(u32::MAX).holds(&catalogue, &at(u32::MAX)));
    }

    #[test]
    fn holds_item_asks_the_inventory_owner() {
        let catalogue = catalogue();
        let predicate = QuestPredicate::HoldsItem(SWORD.into());
        assert!(!predicate.holds(&catalogue, &Facts::default()));
        let holding = Facts {
            items: BTreeSet::from([SWORD.to_owned()]),
            ..Facts::default()
        };
        assert!(predicate.holds(&catalogue, &holding));
        assert!(!QuestPredicate::HoldsItem("oteryn:item/axe".into()).holds(&catalogue, &holding));
    }
}
