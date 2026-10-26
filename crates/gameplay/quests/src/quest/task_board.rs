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
pub const FREE_REROLL_AT: &str = "oteryn:taskboard/weekly/bounty/free-reroll-at";
pub const TALISMAN_OWNED: &str = "oteryn:taskboard/weekly/bounty/talisman-owned";
pub const TALISMAN_LEVEL: &str = "oteryn:taskboard/weekly/bounty/talisman-level";
pub const TALISMAN_GOLD_COST: i64 = 5_000;
pub const DAILY_SECONDS: i64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BountyDifficulty {
    Beginner = 1,
    Adept = 2,
    Expert = 3,
    Master = 4,
}

impl BountyDifficulty {
    #[must_use]
    pub fn from_number(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Beginner),
            2 => Some(Self::Adept),
            3 => Some(Self::Expert),
            4 => Some(Self::Master),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BountyOffer {
    pub race: &'static str,
    pub required: i64,
    pub points: i64,
    pub experience: i64,
    pub variant_multiplier: i64,
    pub difficulty: BountyDifficulty,
}

/// Pinned Canary/Crystal fallback catalogue (owner decision 22). It is intentionally marked as
/// not verified against Global; the three grades are the normal, silver and gold x1/x2/x4 slots.
pub const BOUNTY_SOURCE_QUALIFICATION: &str = "Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446 / Crystal 201189f871d23597b82caf88fd611454a2ddefb8; not verified against Global";
pub const BOUNTY_OFFERS: [BountyOffer; 12] = [
    BountyOffer {
        race: "oteryn:creature.rat",
        required: 50,
        points: 3,
        experience: EXPERIENCE,
        variant_multiplier: 1,
        difficulty: BountyDifficulty::Beginner,
    },
    BountyOffer {
        race: "oteryn:creature.cave_rat",
        required: 75,
        points: 6,
        experience: EXPERIENCE * 2,
        variant_multiplier: 2,
        difficulty: BountyDifficulty::Beginner,
    },
    BountyOffer {
        race: "oteryn:creature.rotworm",
        required: 100,
        points: 12,
        experience: EXPERIENCE * 4,
        variant_multiplier: 4,
        difficulty: BountyDifficulty::Beginner,
    },
    BountyOffer {
        race: "oteryn:creature.troll",
        required: 100,
        points: 7,
        experience: 7_500,
        variant_multiplier: 1,
        difficulty: BountyDifficulty::Adept,
    },
    BountyOffer {
        race: "oteryn:creature.cyclops",
        required: 150,
        points: 14,
        experience: 15_000,
        variant_multiplier: 2,
        difficulty: BountyDifficulty::Adept,
    },
    BountyOffer {
        race: "oteryn:creature.minotaur",
        required: 200,
        points: 28,
        experience: 30_000,
        variant_multiplier: 4,
        difficulty: BountyDifficulty::Adept,
    },
    BountyOffer {
        race: "oteryn:creature.dragon",
        required: 200,
        points: 16,
        experience: 40_000,
        variant_multiplier: 1,
        difficulty: BountyDifficulty::Expert,
    },
    BountyOffer {
        race: "oteryn:creature.hero",
        required: 300,
        points: 32,
        experience: 80_000,
        variant_multiplier: 2,
        difficulty: BountyDifficulty::Expert,
    },
    BountyOffer {
        race: "oteryn:creature.hydra",
        required: 400,
        points: 64,
        experience: 160_000,
        variant_multiplier: 4,
        difficulty: BountyDifficulty::Expert,
    },
    BountyOffer {
        race: "oteryn:creature.demon",
        required: 300,
        points: 27,
        experience: 150_000,
        variant_multiplier: 1,
        difficulty: BountyDifficulty::Master,
    },
    BountyOffer {
        race: "oteryn:creature.grim_reaper",
        required: 450,
        points: 54,
        experience: 300_000,
        variant_multiplier: 2,
        difficulty: BountyDifficulty::Master,
    },
    BountyOffer {
        race: "oteryn:creature.destroyer",
        required: 600,
        points: 108,
        experience: 600_000,
        variant_multiplier: 4,
        difficulty: BountyDifficulty::Master,
    },
];

pub fn offers(difficulty: BountyDifficulty) -> impl Iterator<Item = (u8, &'static BountyOffer)> {
    (1_u8..)
        .zip(BOUNTY_OFFERS.iter())
        .filter(move |(_, offer)| offer.difficulty == difficulty)
}

#[must_use]
pub fn bounty_reroll_transition() -> String {
    format!("{QUEST}/bounty/reroll")
}
#[must_use]
pub fn bounty_free_reroll_transition() -> String {
    format!("{QUEST}/bounty/free-reroll")
}
#[must_use]
pub fn bounty_difficulty_transition(number: u8) -> String {
    format!("{QUEST}/bounty/difficulty-{number}")
}
#[must_use]
pub fn talisman_purchase_transition() -> String {
    format!("{QUEST}/bounty/talisman/purchase")
}
#[must_use]
pub fn talisman_upgrade_transition(level: u8) -> String {
    format!("{QUEST}/bounty/talisman/upgrade-{level}")
}

/// The Global caps, expressed in basis points. Upgrade costs remain deliberately unqualified;
/// callers must not expose upgrades until a sourced curve is installed.
#[must_use]
pub fn talisman_benefits(level: u8) -> (u16, u16, u16, u16) {
    let regular = u16::from(level.min(10)) * 500;
    (regular, regular, regular, u16::from(level.min(10)) * 1_000)
}

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
#[must_use]
pub fn bounty_claim_at_cap_transition(number: u8) -> String {
    format!("{QUEST}/bounty/claim-at-cap-{number}")
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
        track(FREE_REROLL_AT, i64::MAX),
        track(TALISMAN_OWNED, 1),
        track(TALISMAN_LEVEL, 10),
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
            transition(
                bounty_claim_at_cap_transition(index),
                vec![
                    effect(BOUNTY_SELECTED, C::Eq(selected), K::Set(0)),
                    effect(BOUNTY_KILLS, C::Ge(offer.required), K::Set(0)),
                    effect(BOUNTY_POINTS, C::Any, K::Add(offer.points)),
                    effect(REROLL_TOKENS, C::Eq(REROLL_TOKEN_CAP), K::Add(0)),
                ],
                Some(offer.experience),
            ),
        ]);
    }
    result.push(transition(
        bounty_reroll_transition(),
        vec![
            effect(BOUNTY_SELECTED, C::Eq(0), K::Add(0)),
            effect(REROLL_TOKENS, C::Ge(1), K::Add(-1)),
        ],
        None,
    ));
    result.push(transition(
        bounty_free_reroll_transition(),
        vec![
            effect(BOUNTY_SELECTED, C::Eq(0), K::Add(0)),
            effect(FREE_REROLL_AT, C::ElapsedAtLeast(DAILY_SECONDS), K::SetNow),
        ],
        None,
    ));
    for difficulty in 1..=4 {
        result.push(transition(
            bounty_difficulty_transition(difficulty),
            vec![
                effect(BOUNTY_SELECTED, C::Eq(0), K::Add(0)),
                effect(BOUNTY_DIFFICULTY, C::Any, K::Set(i64::from(difficulty))),
            ],
            None,
        ));
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
    pub bounty_difficulty: u8,
    pub talisman_owned: bool,
    pub talisman_level: u8,
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
        bounty_difficulty: u8::try_from(value(BOUNTY_DIFFICULTY)).unwrap_or(0),
        talisman_owned: value(TALISMAN_OWNED) == 1,
        talisman_level: u8::try_from(value(TALISMAN_LEVEL)).unwrap_or(0),
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

pub const WEEK_SECONDS: i64 = 604_800;
pub const FIRST_MONDAY_UNIX: i64 = 345_600;
pub const WEEKLY_INITIAL_KILL_SLOTS: usize = 6;
pub const WEEKLY_INITIAL_DELIVERY_SLOTS: usize = 6;
pub const WEEKLY_EXPANDED_SLOTS: usize = 3;
pub const SOULSEAL_ITEM_KEY: &str = "oteryn:item.soulseal";
pub const BOUNTY_TALISMAN_ITEM_KEY: &str = "oteryn:item.bounty_talisman";

/// Stable Monday 00:00 UTC epoch. Database settlement stores this value with the generated seed.
#[must_use]
pub fn monday_epoch(unix_seconds: i64) -> i64 {
    (unix_seconds - FIRST_MONDAY_UNIX).div_euclid(WEEK_SECONDS)
}

/// SplitMix64 selection used after the seed has been durably generated. The persisted seed makes
/// reconnects and retries reproduce the exact 6+6 (or expanded 9+9) offer set.
#[must_use]
pub fn weekly_selection(seed: u64, pool_len: usize, expanded: bool) -> Vec<usize> {
    let wanted =
        (WEEKLY_INITIAL_KILL_SLOTS + usize::from(expanded) * WEEKLY_EXPANDED_SLOTS).min(pool_len);
    let mut candidates: Vec<_> = (0..pool_len).collect();
    let mut state = seed;
    for index in 0..wanted {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^= mixed >> 31;
        let swap = index + (mixed as usize % (pool_len - index));
        candidates.swap(index, swap);
    }
    candidates.truncate(wanted);
    candidates
}

#[must_use]
pub const fn weekly_reward_multiplier(completed: u8) -> i64 {
    match completed {
        0..=3 => 1,
        4..=7 => 2,
        8..=11 => 3,
        12..=15 => 5,
        _ => 8,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeeklySettlement {
    pub consumed: u32,
    pub hunting_task_points: i64,
    pub soulseals: u8,
}

/// Computes the indivisible delivery mutation. Its SQL consumer locks inventory/stash lines,
/// removes exactly `required`, and writes the epoch receipt and both balances in one transaction.
#[must_use]
pub fn delivery_settlement(
    inventory: u32,
    stash: u32,
    required: u32,
    completed_before: u8,
) -> Option<WeeklySettlement> {
    (inventory.checked_add(stash)? >= required).then(|| WeeklySettlement {
        consumed: required,
        hunting_task_points: 75 * weekly_reward_multiplier(completed_before.saturating_add(1)),
        soulseals: 1,
    })
}

#[must_use]
pub fn kill_settlement(completed_before: u8) -> WeeklySettlement {
    WeeklySettlement {
        consumed: 0,
        hunting_task_points: 25 * weekly_reward_multiplier(completed_before.saturating_add(1)),
        soulseals: 1,
    }
}

/// All four talisman consumers share this equipped-item predicate (owner decision 23).
#[must_use]
pub fn equipped_bounty_talisman(item_key: &str, equipped: bool) -> bool {
    equipped && item_key == BOUNTY_TALISMAN_ITEM_KEY
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TalismanBenefits {
    pub damage_bps: u16,
    pub leech_bps: u16,
    pub loot_bps: u16,
    pub bestiary_bps: u16,
}

#[must_use]
pub fn equipped_talisman_benefits(item_key: &str, equipped: bool, level: u8) -> TalismanBenefits {
    if !equipped_bounty_talisman(item_key, equipped) {
        return TalismanBenefits {
            damage_bps: 0,
            leech_bps: 0,
            loot_bps: 0,
            bestiary_bps: 0,
        };
    }
    let (damage_bps, leech_bps, loot_bps, bestiary_bps) = talisman_benefits(level);
    TalismanBenefits {
        damage_bps,
        leech_bps,
        loot_bps,
        bestiary_bps,
    }
}

/// Hunting Task Shop's atomic balance predicate; the store performs debit and grant together.
#[must_use]
pub const fn hunting_task_shop_permitted(points: i64, price: i64) -> bool {
    price > 0 && points >= price
}

#[cfg(test)]
#[path = "task_board_tests.rs"]
mod tests;
