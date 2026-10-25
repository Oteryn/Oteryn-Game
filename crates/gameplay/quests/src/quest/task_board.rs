//! TASKBOARD-1: the weekly Task Board's kill tasks as one code-built quest
//! (`OTERYN_GAME_TASKBOARD1_SCOPE_DECISION_2026-10-25.md` §2).
//!
//! One active task at a time: `select-<n>` makes task `n` active and zeroes the kill counter, a
//! kill of its race adds one (`kill-<n>`), and `claim-<n>` at the required count clears the
//! active task, adds the task points, stamps the task's `claimed-at` with database time and
//! carries the fixed experience. A task is selectable again seven days after its last claim.

use std::collections::BTreeMap;

use super::{
    QuestCatalogueError, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:taskboard/weekly";
pub const SELECTED: &str = "oteryn:taskboard/weekly/selected";
pub const KILLS: &str = "oteryn:taskboard/weekly/kills";
pub const POINTS: &str = "oteryn:taskboard/weekly/points";
/// The weekly lockout of a claimed task, in database seconds.
pub const LOCKOUT_SECONDS: i64 = 604_800;
/// The experience of one claim: the shared Beginner formula `25L²-75L+100` at level 10.
pub const EXPERIENCE: i64 = 1_850;
/// Task points of one Beginner kill task (Canary and Crystal agree on 25).
pub const TASK_POINTS: i64 = 25;
const POINTS_MAX: i64 = 1_000_000_000;

/// One kill task: the Bestiary race key (the Creature identity key) and the required kills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillTask {
    pub race: &'static str,
    pub required: i64,
}

/// `TASKBOARD1-RL-03`: the fixed Beginner board, tasks numbered from 1.
pub const TASKS: [KillTask; 5] = [
    KillTask {
        race: "oteryn:creature.rat",
        required: 25,
    },
    KillTask {
        race: "oteryn:creature.cave_rat",
        required: 25,
    },
    KillTask {
        race: "oteryn:creature.rotworm",
        required: 30,
    },
    KillTask {
        race: "oteryn:creature.wolf",
        required: 30,
    },
    KillTask {
        race: "oteryn:creature.troll",
        required: 30,
    },
];

/// Task `number` (1-based).
#[must_use]
pub fn task(number: u8) -> Option<&'static KillTask> {
    TASKS.get(usize::from(number).checked_sub(1)?)
}

#[must_use]
pub fn claimed_at_track(number: u8) -> String {
    format!("{QUEST}/task-{number}/claimed-at")
}

#[must_use]
pub fn select_transition(number: u8) -> String {
    format!("{QUEST}/select-{number}")
}

#[must_use]
pub fn kill_transition(number: u8) -> String {
    format!("{QUEST}/kill-{number}")
}

#[must_use]
pub fn claim_transition(number: u8) -> String {
    format!("{QUEST}/claim-{number}")
}

fn numbers() -> impl Iterator<Item = (u8, &'static KillTask)> {
    (1..).zip(TASKS.iter())
}

fn track(key: impl Into<String>, max: i64) -> QuestTrack {
    QuestTrack {
        key: key.into(),
        quest: QUEST.to_owned(),
        initial: 0,
        min: 0,
        max,
    }
}

fn effect(track: impl Into<String>, from: QuestComparison, effect: QuestEffectKind) -> QuestEffect {
    QuestEffect {
        track: track.into(),
        from,
        effect,
    }
}

fn transition(key: String, effects: Vec<QuestEffect>, experience: Option<i64>) -> QuestTransition {
    QuestTransition {
        key,
        quest: QUEST.to_owned(),
        effects,
        completes: false,
        experience,
    }
}

#[must_use]
pub fn tracks() -> Vec<QuestTrack> {
    let required = TASKS.iter().map(|task| task.required).max().unwrap_or(0);
    let mut tracks = vec![
        track(SELECTED, i64::try_from(TASKS.len()).unwrap_or(i64::MAX)),
        track(KILLS, required),
        track(POINTS, POINTS_MAX),
    ];
    tracks.extend(numbers().map(|(number, _)| track(claimed_at_track(number), i64::MAX)));
    tracks
}

#[must_use]
pub fn transitions() -> Vec<QuestTransition> {
    use QuestComparison as C;
    use QuestEffectKind as K;
    numbers()
        .flat_map(|(number, task)| {
            let active = i64::from(number);
            [
                transition(
                    select_transition(number),
                    vec![
                        effect(SELECTED, C::Eq(0), K::Set(active)),
                        effect(KILLS, C::Any, K::Set(0)),
                        effect(
                            claimed_at_track(number),
                            C::ElapsedAtLeast(LOCKOUT_SECONDS),
                            K::Add(0),
                        ),
                    ],
                    None,
                ),
                transition(
                    kill_transition(number),
                    vec![
                        effect(SELECTED, C::Eq(active), K::Add(0)),
                        effect(KILLS, C::Lt(task.required), K::Add(1)),
                    ],
                    None,
                ),
                transition(
                    claim_transition(number),
                    vec![
                        effect(SELECTED, C::Eq(active), K::Set(0)),
                        effect(KILLS, C::Ge(task.required), K::Set(0)),
                        effect(POINTS, C::Any, K::Add(TASK_POINTS)),
                        effect(claimed_at_track(number), C::Any, K::SetNow),
                    ],
                    Some(EXPERIENCE),
                ),
            ]
        })
        .collect()
}

/// The boot catalogue with the Task Board quest added.
pub fn extend(catalogue: &QuestStateCatalogue) -> Result<QuestStateCatalogue, QuestCatalogueError> {
    catalogue.with_quest(tracks(), transitions())
}

/// The board as one Character's stored track values show it; a missing row reads as 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskBoardState {
    /// The active task's number; 0 when none is active.
    pub selected: u8,
    pub kills: i64,
    pub points: i64,
    /// Per task, the database time of its last claim; 0 when never claimed.
    pub claimed_at: [i64; TASKS.len()],
}

#[must_use]
pub fn state(tracks: &BTreeMap<String, i64>) -> TaskBoardState {
    let value = |key: &str| tracks.get(key).copied().unwrap_or(0);
    let mut claimed_at = [0; TASKS.len()];
    for ((number, _), slot) in numbers().zip(claimed_at.iter_mut()) {
        *slot = value(&claimed_at_track(number));
    }
    TaskBoardState {
        selected: u8::try_from(value(SELECTED)).unwrap_or(0),
        kills: value(KILLS),
        points: value(POINTS),
        claimed_at,
    }
}

/// The kill transition a kill of `race` requests: only for the active task of that race that
/// still needs kills. The writer checks both again under lock.
#[must_use]
pub fn kill_credit(tracks: &BTreeMap<String, i64>, race: &str) -> Option<String> {
    let state = state(tracks);
    let task = task(state.selected)?;
    (task.race == race && state.kills < task.required).then(|| kill_transition(state.selected))
}

#[cfg(test)]
#[path = "task_board_tests.rs"]
mod tests;
