//! Task Board quest identity, persisted tracks and accepted talisman profile.
//!
//! The historical TASKBOARD-1 definition remains for existing quest pins and tracks.
//! Production weekly tasks, bounty actions, shops and reset rules live in [`full`]
//! and the fenced durable Task Board owner. Legacy select/claim commands record a
//! retired-action refusal; their fixed-board transitions are not a production path.
//! Point balances and talisman levels retain these existing quest-track identities.

use std::collections::BTreeMap;

#[path = "task_board_full.rs"]
pub mod full;

use super::{
    QuestCatalogueError, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:taskboard/weekly";
pub const SELECTED: &str = "oteryn:taskboard/weekly/selected";
pub const KILLS: &str = "oteryn:taskboard/weekly/kills";
pub const POINTS: &str = "oteryn:taskboard/weekly/points";
pub const BOUNTY_POINTS: &str = "oteryn:taskboard/bounty/points";
pub const TALISMAN_DAMAGE: &str = "oteryn:taskboard/bounty/talisman/damage";
pub const TALISMAN_LIFE_LEECH: &str = "oteryn:taskboard/bounty/talisman/life-leech";
pub const TALISMAN_LOOT: &str = "oteryn:taskboard/bounty/talisman/loot";
pub const TALISMAN_BESTIARY: &str = "oteryn:taskboard/bounty/talisman/bestiary";
pub const TALISMAN_ITEM: &str = "oteryn:item.tibia.i51978";
/// The weekly lockout of a claimed task, in database seconds.
pub const LOCKOUT_SECONDS: i64 = 604_800;
/// The experience of one claim: the shared Beginner formula `25L²-75L+100` at level 10.
pub const EXPERIENCE: i64 = 1_850;
/// Task points of one Beginner kill task (Canary and Crystal agree on 25).
pub const TASK_POINTS: i64 = 25;
pub const TALISMAN_MAX_LEVEL: i64 = 355;
pub const TALISMAN_PATHS: [&str; 4] = [
    TALISMAN_DAMAGE,
    TALISMAN_LIFE_LEECH,
    TALISMAN_LOOT,
    TALISMAN_BESTIARY,
];

#[must_use]
pub const fn talisman_upgrade_cost(level: i64) -> Option<i64> {
    if level < 0 || level >= TALISMAN_MAX_LEVEL {
        None
    } else {
        Some(5 + level * 12)
    }
}

/// Path validation shares the accepted Canary upgrade profile (NEWS-HIST-TALISMAN-1).
#[must_use]
pub const fn talisman_upgrade_cost_for_path(path: usize, level: i64) -> Option<i64> {
    if path >= TALISMAN_PATHS.len() {
        None
    } else {
        talisman_upgrade_cost(level)
    }
}

/// Effect in hundredths of one percent. The fourth path is double-Bestiary chance.
#[must_use]
pub const fn talisman_bonus(path: usize, level: i64) -> i64 {
    if level <= 0 || path >= TALISMAN_PATHS.len() {
        return 0;
    }
    let ordinary = if level <= 15 {
        250 + level * 50
    } else if level <= 55 {
        1_000 + (level - 15) * 25
    } else {
        2_000 + (level - 55) * 10
    };
    if path == 3 { ordinary * 2 } else { ordinary }
}

#[must_use]
pub fn talisman_upgrade_transition(path: usize, level: i64) -> Option<String> {
    TALISMAN_PATHS.get(path)?;
    talisman_upgrade_cost(level)?;
    Some(format!("{QUEST}/talisman-upgrade-{path}-{level}"))
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
        // PR #2764 ANSWER 6543406906 replaces the legacy 1e9 bound; existing
        // persisted values and definition pins are not rewritten.
        track(POINTS, i64::MAX),
        track(BOUNTY_POINTS, i64::from(full::MAX_BOUNTY_POINTS)),
        track(full::SOULSEALS, i64::from(full::MAX_SOULSEALS)),
        track(full::BOARD_REVISION, i64::MAX),
    ];
    tracks.extend(numbers().map(|(number, _)| track(claimed_at_track(number), i64::MAX)));
    tracks.extend(TALISMAN_PATHS.map(|key| track(key, TALISMAN_MAX_LEVEL)));
    tracks
}

#[must_use]
pub fn transitions() -> Vec<QuestTransition> {
    use QuestComparison as C;
    use QuestEffectKind as K;
    let mut transitions: Vec<_> = numbers()
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
    for (path, track_key) in TALISMAN_PATHS.iter().enumerate() {
        for level in 0..TALISMAN_MAX_LEVEL {
            let cost = 5 + level * 12;
            transitions.push(transition(
                format!("{QUEST}/talisman-upgrade-{path}-{level}"),
                vec![
                    effect(BOUNTY_POINTS, C::Ge(cost), K::Add(-cost)),
                    effect(*track_key, C::Eq(level), K::Add(1)),
                ],
                None,
            ));
        }
    }
    transitions
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
    pub bounty_points: i64,
    pub talisman_levels: [i64; TALISMAN_PATHS.len()],
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
        bounty_points: value(BOUNTY_POINTS),
        talisman_levels: TALISMAN_PATHS.map(value),
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
