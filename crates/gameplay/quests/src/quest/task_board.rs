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
pub const BOUNTY_SELECTED: &str = "oteryn:taskboard/weekly/bounty/selected";
pub const BOUNTY_KILLS: &str = "oteryn:taskboard/weekly/bounty/kills";
pub const BOUNTY_POINTS: &str = "oteryn:taskboard/weekly/bounty/points";
pub const REROLL_TOKENS: &str = "oteryn:taskboard/weekly/bounty/reroll-tokens";
pub const BOUNTY_DIFFICULTY: &str = "oteryn:taskboard/weekly/bounty/next-difficulty";
pub const REROLL_TOKEN_CAP: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BountyOffer {
    pub race: &'static str,
    pub required: i64,
    pub points: i64,
    pub experience: i64,
    pub variant_multiplier: i64,
}

/// The three qualified Beginner offers. The silver and gold slots apply Global's x2/x4 reward
/// variants; completing any offer also awards one reroll token.
pub const BOUNTY_OFFERS: [BountyOffer; 3] = [
    BountyOffer {
        race: "oteryn:creature.rat",
        required: 50,
        points: 3,
        experience: EXPERIENCE,
        variant_multiplier: 1,
    },
    BountyOffer {
        race: "oteryn:creature.cave_rat",
        required: 75,
        points: 6,
        experience: EXPERIENCE * 2,
        variant_multiplier: 2,
    },
    BountyOffer {
        race: "oteryn:creature.rotworm",
        required: 100,
        points: 12,
        experience: EXPERIENCE * 4,
        variant_multiplier: 4,
    },
];

#[must_use]
pub fn bounty_select_transition(number: u8) -> String {
    format!("{QUEST}/bounty/select-{number}")
}
#[must_use]
pub fn bounty_kill_transition(number: u8) -> String {
    format!("{QUEST}/bounty/kill-{number}")
}
#[must_use]
pub fn bounty_claim_transition(number: u8) -> String {
    format!("{QUEST}/bounty/claim-{number}")
}

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
        track(BOUNTY_SELECTED, BOUNTY_OFFERS.len() as i64),
        track(BOUNTY_KILLS, 600),
        track(BOUNTY_POINTS, POINTS_MAX),
        track(REROLL_TOKENS, REROLL_TOKEN_CAP),
        track(BOUNTY_DIFFICULTY, 4),
    ];
    tracks.extend(numbers().map(|(number, _)| track(claimed_at_track(number), i64::MAX)));
    tracks
}

#[must_use]
pub fn transitions() -> Vec<QuestTransition> {
    use QuestComparison as C;
    use QuestEffectKind as K;
    let mut result: Vec<_> = numbers()
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
        .collect();
    for (index, offer) in (1_u8..).zip(BOUNTY_OFFERS) {
        let selected = i64::from(index);
        result.extend([
            transition(
                bounty_select_transition(index),
                vec![
                    effect(BOUNTY_SELECTED, C::Eq(0), K::Set(selected)),
                    effect(BOUNTY_KILLS, C::Any, K::Set(0)),
                ],
                None,
            ),
            transition(
                bounty_kill_transition(index),
                vec![
                    effect(BOUNTY_SELECTED, C::Eq(selected), K::Add(0)),
                    effect(BOUNTY_KILLS, C::Lt(offer.required), K::Add(1)),
                ],
                None,
            ),
            transition(
                bounty_claim_transition(index),
                vec![
                    effect(BOUNTY_SELECTED, C::Eq(selected), K::Set(0)),
                    effect(BOUNTY_KILLS, C::Ge(offer.required), K::Set(0)),
                    effect(BOUNTY_POINTS, C::Any, K::Add(offer.points)),
                    effect(REROLL_TOKENS, C::Lt(REROLL_TOKEN_CAP), K::Add(1)),
                ],
                Some(offer.experience),
            ),
        ]);
    }
    result
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
    pub bounty_selected: u8,
    pub bounty_kills: i64,
    pub bounty_points: i64,
    pub reroll_tokens: i64,
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
        bounty_selected: u8::try_from(value(BOUNTY_SELECTED)).unwrap_or(0),
        bounty_kills: value(BOUNTY_KILLS),
        bounty_points: value(BOUNTY_POINTS),
        reroll_tokens: value(REROLL_TOKENS),
    }
}

#[must_use]
pub fn bounty_kill_credit(tracks: &BTreeMap<String, i64>, race: &str) -> Option<String> {
    let state = state(tracks);
    let offer = BOUNTY_OFFERS.get(usize::from(state.bounty_selected).checked_sub(1)?)?;
    (offer.race == race && state.bounty_kills < offer.required)
        .then(|| bounty_kill_transition(state.bounty_selected))
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
