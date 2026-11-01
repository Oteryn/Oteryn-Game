//! Crystal Task Board rules. The durable owner supplies calendar, balances and random draws.
//!
//! Reference: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`,
//! `src/io/ioweeklytasks.cpp` and `src/io/iobountytasks.cpp`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{BOUNTY_POINTS, POINTS, TALISMAN_PATHS};

#[path = "task_board_catalogue.rs"]
mod catalogue;
pub use catalogue::{DELIVERY_ITEMS, SHOP_OFFERS, TaskBoardShopKind, TaskBoardShopOffer};

pub const SOULSEALS: &str = "oteryn:taskboard/weekly/soulseals";
pub const BOARD_REVISION: &str = "oteryn:taskboard/weekly/board-revision";
pub const FREE_REROLL_SECONDS: i64 = 20 * 60 * 60;
pub const MAX_DAILY_REROLL_TOKENS: u64 = 10;
pub const TALISMAN_PURCHASE_GOLD: u64 = 5_000;
/// Crystal iobountytasks.hpp:143; PR #2764 ANSWER 6543406906 rejects wrapping.
pub const MAX_BOUNTY_POINTS: u32 = u32::MAX;
/// Crystal ioweeklytasks.hpp:148; independent of the historical 1e9 track bound.
pub const MAX_SOULSEALS: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskBoardData {
    pub weekly: Option<WeeklyBoard>,
    pub bounty: BountyBoard,
    pub expansion: bool,
    /// An index of this shop's completed one-time purchases, not cosmetic authority.
    pub shop_purchases: BTreeSet<u16>,
    /// Paid bonus points read by the existing Wheel allocation owner.
    pub promotion_points: u16,
}

impl TaskBoardData {
    /// Crystal iobountytasks.cpp:760-780: bonuses end at the completing kill.
    #[must_use]
    pub fn active_talisman_bonuses(
        &self,
        tracks: &BTreeMap<String, i64>,
        race: &str,
        equipped: bool,
    ) -> [i64; 4] {
        if !equipped
            || self.bounty.state != BountyStatus::Active
            || !self
                .bounty
                .active
                .as_ref()
                .is_some_and(|task| task.race == race)
        {
            return [0; 4];
        }
        std::array::from_fn(|path| {
            super::talisman_bonus(path, tracks.get(TALISMAN_PATHS[path]).copied().unwrap_or(0))
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyBoard {
    pub period: i64,
    pub difficulty: u8,
    pub any_required: u32,
    pub any_kills: u32,
    pub kills: Vec<WeeklyKillTask>,
    pub deliveries: Vec<WeeklyDeliveryTask>,
    /// The server-save owner paid this week's points and soulseals.
    pub finished: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyKillTask {
    pub race: String,
    pub required: u32,
    pub current: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklyDeliveryTask {
    pub item_id: u16,
    pub required: u32,
    pub delivered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BountyStatus {
    #[default]
    None,
    Selection,
    Active,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BountyBoard {
    pub state: BountyStatus,
    pub difficulty: u8,
    pub offers: Vec<BountyTask>,
    pub active: Option<BountyTask>,
    pub lists: [BountyListSlot; 5],
    pub next_free_reroll_unix: i64,
}

impl Default for BountyBoard {
    fn default() -> Self {
        let mut lists = std::array::from_fn(|_| BountyListSlot::default());
        lists[0].unlocked = true;
        Self {
            state: BountyStatus::None,
            difficulty: 0,
            offers: Vec::new(),
            active: None,
            lists,
            next_free_reroll_unix: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BountyListSlot {
    pub unlocked: bool,
    pub preferred: Option<String>,
    pub unwanted: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BountyTask {
    pub race: String,
    pub required: u32,
    pub current: u32,
    pub experience: u64,
    pub points: u32,
    /// 0 ordinary, 1 silver, 2 gold.
    pub grade: u8,
    pub difficulty: u8,
    pub slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskBoardBalances {
    pub points: i64,
    pub bounty_points: i64,
    pub soulseals: i64,
    pub talisman_levels: [i64; 4],
}

impl TaskBoardBalances {
    #[must_use]
    pub fn from_tracks(tracks: &BTreeMap<String, i64>) -> Self {
        let value = |key: &str| tracks.get(key).copied().unwrap_or(0);
        Self {
            points: value(POINTS),
            bounty_points: value(BOUNTY_POINTS),
            soulseals: value(SOULSEALS),
            talisman_levels: TALISMAN_PATHS.map(value),
        }
    }

    pub fn store_into_tracks(&self, tracks: &mut BTreeMap<String, i64>) {
        for (key, value) in [
            (POINTS, self.points),
            (BOUNTY_POINTS, self.bounty_points),
            (SOULSEALS, self.soulseals),
        ] {
            tracks.insert(key.to_owned(), value);
        }
        for (key, value) in TALISMAN_PATHS.into_iter().zip(self.talisman_levels) {
            tracks.insert(key.to_owned(), value);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardCreature {
    pub race: String,
    pub bestiary_stars: u8,
    pub experience: u64,
    pub preyable: bool,
    pub prey_exclusive: bool,
}

pub struct TaskBoardContext<'a> {
    pub level: u32,
    /// Existing durable build owner; absence and the no-vocation state are false.
    pub has_vocation: bool,
    /// Database time, never a caller or node clock.
    pub now_unix_seconds: i64,
    /// Current weekly period supplied by the server-save owner.
    pub period: i64,
    pub creatures: &'a [TaskBoardCreature],
    pub available_gold: u64,
    /// Existing durable bounty-reroll resource balance.
    pub reroll_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskBoardAction {
    Query,
    StartWeekly {
        difficulty: u8,
    },
    Deliver {
        slot: u8,
    },
    OpenBounty,
    SelectBounty {
        slot: u8,
    },
    ClaimBounty,
    ChangeBountyDifficulty {
        difficulty: u8,
    },
    RerollBounty,
    ClaimDailyReroll,
    SetPreferred {
        slot: u8,
        race: Option<String>,
    },
    SetUnwanted {
        slot: u8,
        race: Option<String>,
    },
    UnlockList,
    BuyShop {
        offer: u16,
    },
    BuyTalisman,
    UpgradeTalisman {
        path: u8,
    },
    CreditKill {
        race: String,
    },
    /// Internal Store-delivery entitlement; never accepted from the client wire.
    GrantExpansion,
    RefuseLegacy {
        action: u8,
        task: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskBoardRefusal {
    InvalidInput,
    WrongState,
    LevelTooLow,
    InsufficientResources,
    Cooldown,
    LimitReached,
    AlreadyPurchased,
    RequiresOutfit,
    InvalidRandomDraw,
    MissingCatalogue,
    ArithmeticOverflow,
    RetiredAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardItemQuantity {
    pub item_id: u16,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskBoardEffects {
    pub experience: u64,
    /// Ordered weekly awards: any-creature completion precedes the specific race.
    /// The durable progression planner replaces `experience` using the admitted
    /// level policy after each award; one kill may cross a level between the two.
    #[serde(default)]
    pub weekly_completions: u8,
    pub consume_items: Vec<TaskBoardItemQuantity>,
    pub grant_items: Vec<TaskBoardItemQuantity>,
    pub gold_cost: u64,
    pub reroll_tokens_spent: u32,
    pub reroll_tokens_granted: u32,
    pub outfit: Option<(u16, u8)>,
    pub mount: Option<u16>,
    pub promotion_points: u16,
}

/// Crystal ioweeklytasks.cpp:1031-1044. Invalid difficulties never silently become Beginner.
#[must_use]
pub const fn minimum_level(difficulty: u8) -> Option<u32> {
    match difficulty {
        0 => Some(8),
        1 => Some(30),
        2 => Some(150),
        3 => Some(300),
        _ => None,
    }
}

/// Crystal ioweeklytasks.cpp:989-1029, with exact positive rounding in the middle band.
pub fn weekly_experience(level: u32, difficulty: u8) -> Result<u64, TaskBoardRefusal> {
    let cap = match difficulty {
        0 => 200_000,
        1 => 800_000,
        2 => 3_000_000,
        3 => u64::MAX,
        _ => return Err(TaskBoardRefusal::InvalidInput),
    };
    if level == 0 {
        return Err(TaskBoardRefusal::InvalidInput);
    }
    let level = u64::from(level);
    let experience = if level <= 82 {
        25 * level * level + 100 - 75 * level
    } else if level < 1_000 {
        (1_994_008 * level + 500) / 1_000
    } else {
        level
            .checked_mul(level)
            .and_then(|square| square.checked_mul(2))
            .and_then(|value| value.checked_sub(6 * level))
            .and_then(|value| value.checked_add(8))
            .ok_or(TaskBoardRefusal::ArithmeticOverflow)?
    };
    Ok(experience.min(cap))
}

/// Crystal ioweeklytasks.cpp:470-486. Executable code uses x5 at thirteen (the comment says x4).
#[must_use]
pub const fn completion_multiplier(completed: u8) -> u32 {
    match completed {
        17.. => 8,
        13..=16 => 5,
        9..=12 => 3,
        5..=8 => 2,
        _ => 1,
    }
}

impl WeeklyBoard {
    #[must_use]
    pub fn completed_kills(&self) -> u8 {
        u8::from(self.any_required > 0 && self.any_kills >= self.any_required)
            + u8::try_from(
                self.kills
                    .iter()
                    .filter(|task| task.current >= task.required)
                    .count(),
            )
            .unwrap_or(u8::MAX)
    }

    #[must_use]
    pub fn completed_deliveries(&self) -> u8 {
        u8::try_from(self.deliveries.iter().filter(|task| task.delivered).count())
            .unwrap_or(u8::MAX)
    }

    /// Points and soulseals are due only at the reset owner's calendar boundary.
    pub fn rewards(&self) -> Result<(u32, u32), TaskBoardRefusal> {
        let per_kill = match self.difficulty {
            0 => 25,
            1 => 50,
            2 => 100,
            3 => 110,
            _ => return Err(TaskBoardRefusal::InvalidInput),
        };
        let kills = self.completed_kills();
        let deliveries = self.completed_deliveries();
        let total = kills
            .checked_add(deliveries)
            .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
        Ok((
            (u32::from(kills) * per_kill + u32::from(deliveries) * 75)
                * completion_multiplier(total),
            u32::from(total),
        ))
    }
}

/// A refusal leaves both objects untouched. The writer persists effects and the new state in
/// one fenced transaction and returns its immutable receipt on replay before calling this.
pub fn apply_action(
    data: &mut TaskBoardData,
    balances: &mut TaskBoardBalances,
    action: &TaskBoardAction,
    context: &TaskBoardContext<'_>,
    draw: &mut impl FnMut(u32) -> u32,
) -> Result<TaskBoardEffects, TaskBoardRefusal> {
    if context.now_unix_seconds < 0
        || context.period < 0
        || balances.points < 0
        || balances.bounty_points < 0
        || balances.soulseals < 0
        || balances.talisman_levels.iter().any(|level| *level < 0)
    {
        return Err(TaskBoardRefusal::InvalidInput);
    }
    let mut next = data.clone();
    let mut funds = balances.clone();
    settle_period(&mut next, &mut funds, context.period)?;
    let result = apply_inner(&mut next, &mut funds, action, context, draw)?;
    *data = next;
    *balances = funds;
    Ok(result)
}

fn settle_period(
    data: &mut TaskBoardData,
    balances: &mut TaskBoardBalances,
    period: i64,
) -> Result<(), TaskBoardRefusal> {
    if let Some(weekly) = data.weekly.as_mut() {
        if weekly.period > period {
            return Err(TaskBoardRefusal::WrongState);
        }
        if weekly.period < period && !weekly.finished {
            let (points, seals) = weekly.rewards()?;
            balances.points = balances
                .points
                .checked_add(i64::from(points))
                .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
            balances.soulseals = balances
                .soulseals
                .checked_add(i64::from(seals))
                .filter(|sum| *sum <= i64::from(MAX_SOULSEALS))
                .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
            weekly.finished = true;
        }
    }
    Ok(())
}

fn apply_inner(
    data: &mut TaskBoardData,
    balances: &mut TaskBoardBalances,
    action: &TaskBoardAction,
    context: &TaskBoardContext<'_>,
    draw: &mut impl FnMut(u32) -> u32,
) -> Result<TaskBoardEffects, TaskBoardRefusal> {
    let mut effects = TaskBoardEffects::default();
    match action {
        TaskBoardAction::Query => {}
        TaskBoardAction::RefuseLegacy { .. } => return Err(TaskBoardRefusal::RetiredAction),
        TaskBoardAction::StartWeekly { difficulty } => {
            require_level(context.level, *difficulty)?;
            if data.weekly.as_ref().is_some_and(|board| !board.finished) {
                return Err(TaskBoardRefusal::WrongState);
            }
            let mut board = WeeklyBoard {
                period: context.period,
                difficulty: *difficulty,
                any_required: (u32::from(*difficulty) + 1) * 1_000,
                any_kills: 0,
                kills: Vec::new(),
                deliveries: Vec::new(),
                finished: false,
            };
            fill_weekly(&mut board, data.expansion, false, context.creatures, draw)?;
            data.weekly = Some(board);
        }
        TaskBoardAction::Deliver { slot } => {
            let board = data
                .weekly
                .as_mut()
                .filter(|board| !board.finished)
                .ok_or(TaskBoardRefusal::WrongState)?;
            let task = board
                .deliveries
                .get_mut(usize::from(*slot))
                .ok_or(TaskBoardRefusal::InvalidInput)?;
            if task.delivered {
                return Err(TaskBoardRefusal::WrongState);
            }
            effects.consume_items.push(TaskBoardItemQuantity {
                item_id: task.item_id,
                amount: task.required,
            });
            effects.experience = weekly_experience(context.level, board.difficulty)?;
            effects.weekly_completions = 1;
            task.delivered = true;
        }
        TaskBoardAction::OpenBounty => {
            // Crystal game.cpp:9995-9998 gates Rookgaard, not the current level.
            if !context.has_vocation {
                return Err(TaskBoardRefusal::WrongState);
            }
            if data.bounty.state == BountyStatus::None {
                generate_bounty(&mut data.bounty, context.creatures, draw)?;
            }
        }
        TaskBoardAction::SelectBounty { slot } => {
            if data.bounty.state != BountyStatus::Selection {
                return Err(TaskBoardRefusal::WrongState);
            }
            let mut task = data
                .bounty
                .offers
                .get(usize::from(*slot))
                .ok_or(TaskBoardRefusal::InvalidInput)?
                .clone();
            // Crystal iobountytasks.cpp:299: selection records the currently chosen
            // difficulty without recalculating the already generated offer's rewards.
            task.difficulty = data.bounty.difficulty;
            data.bounty.active = Some(task);
            data.bounty.offers.clear();
            data.bounty.state = BountyStatus::Active;
        }
        TaskBoardAction::ClaimBounty => {
            if data.bounty.state != BountyStatus::Completed {
                return Err(TaskBoardRefusal::WrongState);
            }
            // The accepted existing token owner stores a nonnegative BIGINT. Refuse
            // overflow before changing XP, points or tasks, so the refusal is replayable.
            if context.reroll_tokens >= i64::MAX.unsigned_abs() {
                return Err(TaskBoardRefusal::ArithmeticOverflow);
            }
            let task = data
                .bounty
                .active
                .as_ref()
                .ok_or(TaskBoardRefusal::WrongState)?;
            effects.experience = task.experience;
            balances.bounty_points = balances
                .bounty_points
                .checked_add(i64::from(task.points))
                .filter(|sum| *sum <= i64::from(MAX_BOUNTY_POINTS))
                .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
            effects.reroll_tokens_granted = 1;
            generate_bounty(&mut data.bounty, context.creatures, draw)?;
        }
        TaskBoardAction::ChangeBountyDifficulty { difficulty } => {
            require_level(context.level, *difficulty)?;
            data.bounty.difficulty = *difficulty;
        }
        TaskBoardAction::RerollBounty => {
            // Crystal iobountytasks.cpp:331-351 keeps the chosen difficulty
            // after level loss; only ChangeBountyDifficulty checks its level gate.
            if data.bounty.next_free_reroll_unix <= context.now_unix_seconds {
                data.bounty.next_free_reroll_unix = context
                    .now_unix_seconds
                    .checked_add(FREE_REROLL_SECONDS)
                    .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
            } else if context.reroll_tokens > 0 {
                effects.reroll_tokens_spent = 1;
            } else {
                return Err(TaskBoardRefusal::Cooldown);
            }
            generate_bounty(&mut data.bounty, context.creatures, draw)?;
        }
        TaskBoardAction::ClaimDailyReroll => {
            if context.reroll_tokens >= MAX_DAILY_REROLL_TOKENS {
                return Err(TaskBoardRefusal::LimitReached);
            }
            if data.bounty.next_free_reroll_unix > context.now_unix_seconds {
                return Err(TaskBoardRefusal::Cooldown);
            }
            data.bounty.next_free_reroll_unix = context
                .now_unix_seconds
                .checked_add(FREE_REROLL_SECONDS)
                .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
            effects.reroll_tokens_granted = 1;
        }
        TaskBoardAction::SetPreferred { slot, race }
        | TaskBoardAction::SetUnwanted { slot, race } => {
            let preferred = matches!(action, TaskBoardAction::SetPreferred { .. });
            set_list(
                &mut data.bounty,
                balances,
                *slot,
                race.as_deref(),
                preferred,
                context.creatures,
            )?;
        }
        TaskBoardAction::UnlockList => {
            let (index, slot) = data
                .bounty
                .lists
                .iter_mut()
                .enumerate()
                .find(|(_, slot)| !slot.unlocked)
                .ok_or(TaskBoardRefusal::LimitReached)?;
            let cost = i64::try_from(index).map_err(|_| TaskBoardRefusal::InvalidInput)? * 300;
            spend(&mut balances.bounty_points, cost)?;
            slot.unlocked = true;
        }
        TaskBoardAction::BuyShop { offer } => buy_shop(data, balances, *offer, &mut effects)?,
        TaskBoardAction::BuyTalisman => {
            if context.available_gold < TALISMAN_PURCHASE_GOLD {
                return Err(TaskBoardRefusal::InsufficientResources);
            }
            effects.gold_cost = TALISMAN_PURCHASE_GOLD;
            effects.grant_items.push(TaskBoardItemQuantity {
                item_id: 51_978,
                amount: 1,
            });
        }
        TaskBoardAction::UpgradeTalisman { path } => {
            let level = balances
                .talisman_levels
                .get_mut(usize::from(*path))
                .ok_or(TaskBoardRefusal::InvalidInput)?;
            let cost = super::talisman_upgrade_cost_for_path(usize::from(*path), *level)
                .ok_or(TaskBoardRefusal::LimitReached)?;
            spend(&mut balances.bounty_points, cost)?;
            *level += 1;
        }
        TaskBoardAction::CreditKill { race } => credit_kill(data, race, context, &mut effects)?,
        TaskBoardAction::GrantExpansion => {
            if data.expansion {
                return Err(TaskBoardRefusal::AlreadyPurchased);
            }
            data.expansion = true;
            if let Some(board) = data.weekly.as_mut().filter(|board| !board.finished) {
                fill_weekly(board, true, true, context.creatures, draw)?;
            }
        }
    }
    Ok(effects)
}

fn require_level(level: u32, difficulty: u8) -> Result<(), TaskBoardRefusal> {
    let minimum = minimum_level(difficulty).ok_or(TaskBoardRefusal::InvalidInput)?;
    if level < minimum {
        return Err(TaskBoardRefusal::LevelTooLow);
    }
    Ok(())
}

fn spend(balance: &mut i64, cost: i64) -> Result<(), TaskBoardRefusal> {
    if *balance < cost {
        return Err(TaskBoardRefusal::InsufficientResources);
    }
    *balance -= cost;
    Ok(())
}

fn random(draw: &mut impl FnMut(u32) -> u32, low: u32, high: u32) -> Result<u32, TaskBoardRefusal> {
    let width = high
        .checked_sub(low)
        .and_then(|n| n.checked_add(1))
        .ok_or(TaskBoardRefusal::InvalidInput)?;
    let value = draw(width);
    if value >= width {
        return Err(TaskBoardRefusal::InvalidRandomDraw);
    }
    Ok(low + value)
}

fn eligible(creature: &TaskBoardCreature) -> bool {
    creature.experience > 0 && creature.preyable && !creature.prey_exclusive
}

fn stars_match(difficulty: u8, stars: u8, expansion_fill: bool) -> bool {
    match difficulty {
        0 => stars <= 1,
        1 => stars <= 3,
        // Crystal's expansion fill is wider than initial generation: cpp:1189 vs cpp:161.
        2 if expansion_fill => (2..=5).contains(&stars),
        2 => (3..=4).contains(&stars),
        3 => stars >= 4,
        _ => false,
    }
}

fn required_kills(
    difficulty: u8,
    draw: &mut impl FnMut(u32) -> u32,
) -> Result<u32, TaskBoardRefusal> {
    match difficulty {
        0 => random(draw, 50, 100),
        1 => random(draw, 100, 200),
        2 => random(draw, 200, 300),
        3 => random(draw, 300, 600),
        _ => Err(TaskBoardRefusal::InvalidInput),
    }
}

fn fill_weekly(
    board: &mut WeeklyBoard,
    expansion: bool,
    expansion_fill: bool,
    creatures: &[TaskBoardCreature],
    draw: &mut impl FnMut(u32) -> u32,
) -> Result<(), TaskBoardRefusal> {
    let kill_slots = if expansion { 8 } else { 5 };
    let delivery_slots = if expansion { 9 } else { 6 };
    let mut candidates: Vec<_> = creatures
        .iter()
        .filter(|creature| {
            eligible(creature)
                && stars_match(board.difficulty, creature.bestiary_stars, expansion_fill)
                && !board.kills.iter().any(|task| task.race == creature.race)
        })
        .collect();
    if candidates.len() < kill_slots - board.kills.len() {
        return Err(TaskBoardRefusal::MissingCatalogue);
    }
    while board.kills.len() < kill_slots {
        let max =
            u32::try_from(candidates.len() - 1).map_err(|_| TaskBoardRefusal::MissingCatalogue)?;
        let index = usize::try_from(random(draw, 0, max)?)
            .map_err(|_| TaskBoardRefusal::InvalidRandomDraw)?;
        let creature = candidates.swap_remove(index);
        board.kills.push(WeeklyKillTask {
            race: creature.race.clone(),
            required: required_kills(board.difficulty, draw)?,
            current: 0,
        });
    }
    let mut available: Vec<_> = DELIVERY_ITEMS
        .iter()
        .filter(|item| {
            !board
                .deliveries
                .iter()
                .any(|task| task.item_id == item.item_id)
        })
        .collect();
    if available.len() < delivery_slots - board.deliveries.len() {
        return Err(TaskBoardRefusal::MissingCatalogue);
    }
    while board.deliveries.len() < delivery_slots {
        let max =
            u32::try_from(available.len() - 1).map_err(|_| TaskBoardRefusal::MissingCatalogue)?;
        let index = usize::try_from(random(draw, 0, max)?)
            .map_err(|_| TaskBoardRefusal::InvalidRandomDraw)?;
        let item = available.swap_remove(index);
        board.deliveries.push(WeeklyDeliveryTask {
            item_id: item.item_id,
            required: item.amount,
            delivered: false,
        });
    }
    Ok(())
}

fn generate_bounty(
    board: &mut BountyBoard,
    creatures: &[TaskBoardCreature],
    draw: &mut impl FnMut(u32) -> u32,
) -> Result<(), TaskBoardRefusal> {
    let mut candidates: Vec<_> = creatures
        .iter()
        .filter(|creature| {
            eligible(creature)
                && !board
                    .lists
                    .iter()
                    .any(|slot| slot.unlocked && slot.unwanted.as_deref() == Some(&creature.race))
                && (stars_match(board.difficulty, creature.bestiary_stars, false)
                    || board.lists.iter().any(|slot| {
                        slot.unlocked && slot.preferred.as_deref() == Some(&creature.race)
                    }))
        })
        .collect();
    if candidates.len() < 3 {
        return Err(TaskBoardRefusal::MissingCatalogue);
    }
    board.offers.clear();
    for slot in 0..3 {
        let max =
            u32::try_from(candidates.len() - 1).map_err(|_| TaskBoardRefusal::MissingCatalogue)?;
        let index = usize::try_from(random(draw, 0, max)?)
            .map_err(|_| TaskBoardRefusal::InvalidRandomDraw)?;
        let creature = candidates.swap_remove(index);
        let required = required_kills(board.difficulty, draw)?;
        let (min, max, points) = match board.difficulty {
            0 => (30, 50, 3_u32),
            1 => (70, 100, 7),
            2 => (150, 200, 16),
            3 => (300, 400, 27),
            _ => return Err(TaskBoardRefusal::InvalidInput),
        };
        let experience_multiplier = random(draw, min, max)?;
        let grade_roll = random(draw, 1, 100)?;
        let (grade, grade_multiplier) = if grade_roll <= 2 {
            (2, 4_u32)
        } else if grade_roll <= 10 {
            (1, 2)
        } else {
            (0, 1)
        };
        let experience = creature
            .experience
            .checked_mul(u64::from(experience_multiplier))
            .and_then(|value| value.checked_mul(u64::from(grade_multiplier)))
            .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
        board.offers.push(BountyTask {
            race: creature.race.clone(),
            required,
            current: 0,
            experience,
            points: (points * grade_multiplier).min(255),
            grade,
            difficulty: board.difficulty,
            slot,
        });
    }
    board.active = None;
    board.state = BountyStatus::Selection;
    Ok(())
}

fn set_list(
    board: &mut BountyBoard,
    balances: &mut TaskBoardBalances,
    index: u8,
    race: Option<&str>,
    preferred: bool,
    creatures: &[TaskBoardCreature],
) -> Result<(), TaskBoardRefusal> {
    let slot = board
        .lists
        .get_mut(usize::from(index))
        .ok_or(TaskBoardRefusal::InvalidInput)?;
    if !slot.unlocked {
        return Err(TaskBoardRefusal::WrongState);
    }
    if race.is_some_and(|race| !creatures.iter().any(|creature| creature.race == race)) {
        return Err(TaskBoardRefusal::InvalidInput);
    }
    let target = if preferred {
        &mut slot.preferred
    } else {
        &mut slot.unwanted
    };
    if race.is_none() && target.is_some() {
        spend(&mut balances.bounty_points, 10)?;
    }
    *target = race.map(str::to_owned);
    Ok(())
}

fn credit_kill(
    data: &mut TaskBoardData,
    race: &str,
    context: &TaskBoardContext<'_>,
    effects: &mut TaskBoardEffects,
) -> Result<(), TaskBoardRefusal> {
    if let Some(board) = data.weekly.as_mut().filter(|board| !board.finished) {
        let mut completed = 0_u8;
        if context
            .creatures
            .iter()
            .any(|creature| creature.race == race && eligible(creature))
            && board.any_kills < board.any_required
        {
            board.any_kills += 1;
            if board.any_kills == board.any_required {
                completed += 1;
            }
        }
        if let Some(task) = board
            .kills
            .iter_mut()
            .find(|task| task.race == race && task.current < task.required)
        {
            task.current += 1;
            if task.current == task.required {
                completed += 1;
            }
        }
        effects.weekly_completions = completed;
        effects.experience = weekly_experience(context.level, board.difficulty)?
            .checked_mul(u64::from(completed))
            .ok_or(TaskBoardRefusal::ArithmeticOverflow)?;
    }
    if data.bounty.state == BountyStatus::Active {
        let task = data
            .bounty
            .active
            .as_mut()
            .ok_or(TaskBoardRefusal::WrongState)?;
        if task.race == race && task.current < task.required {
            task.current += 1;
            if task.current == task.required {
                data.bounty.state = BountyStatus::Completed;
            }
        }
    }
    Ok(())
}

pub fn shop_price(
    offer: &TaskBoardShopOffer,
    promotion_points: u16,
) -> Result<u32, TaskBoardRefusal> {
    if offer.kind == TaskBoardShopKind::Promotion {
        if promotion_points >= 50 {
            return Err(TaskBoardRefusal::LimitReached);
        }
        let next = u32::from(promotion_points) + 1;
        Ok(100 + 50 * next * (next - 1))
    } else {
        Ok(offer.points)
    }
}

fn buy_shop(
    data: &mut TaskBoardData,
    balances: &mut TaskBoardBalances,
    id: u16,
    effects: &mut TaskBoardEffects,
) -> Result<(), TaskBoardRefusal> {
    let offer = SHOP_OFFERS
        .iter()
        .find(|offer| offer.id == id)
        .ok_or(TaskBoardRefusal::InvalidInput)?;
    let one_time = matches!(
        offer.kind,
        TaskBoardShopKind::Mount | TaskBoardShopKind::Outfit
    );
    if one_time && data.shop_purchases.contains(&id) {
        return Err(TaskBoardRefusal::AlreadyPurchased);
    }
    if offer.kind == TaskBoardShopKind::Outfit && offer.addon > 0 {
        let base = SHOP_OFFERS
            .iter()
            .find(|base| {
                base.kind == TaskBoardShopKind::Outfit
                    && base.appearance_or_item == offer.appearance_or_item
                    && base.addon == 0
            })
            .ok_or(TaskBoardRefusal::MissingCatalogue)?;
        if !data.shop_purchases.contains(&base.id) {
            return Err(TaskBoardRefusal::RequiresOutfit);
        }
    }
    spend(
        &mut balances.points,
        i64::from(shop_price(offer, data.promotion_points)?),
    )?;
    match offer.kind {
        TaskBoardShopKind::Item | TaskBoardShopKind::ItemDouble => {
            effects.grant_items.push(TaskBoardItemQuantity {
                item_id: offer.appearance_or_item,
                amount: 1,
            })
        }
        TaskBoardShopKind::Mount => effects.mount = Some(offer.appearance_or_item),
        TaskBoardShopKind::Outfit => effects.outfit = Some((offer.appearance_or_item, offer.addon)),
        TaskBoardShopKind::Promotion => {
            data.promotion_points += 1;
            effects.promotion_points = 1;
        }
    }
    if one_time {
        data.shop_purchases.insert(id);
    }
    Ok(())
}

#[cfg(test)]
#[path = "task_board_full_tests.rs"]
mod tests;
