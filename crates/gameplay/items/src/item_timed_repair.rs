//! TIMED-ITEM-0 §7 repair planning only; no transaction, permission or durable outcome.
//!
//! The owner must independently resolve the admitted Aldo offer and current Content (including
//! an inactive target's paired active full values), current session/NPC eligibility and locked
//! inventory/bank facts. These snapshots cannot qualify that authority or activation. Applying
//! the plan requires a single native fenced transaction, NpcRepair fee/audit support and timed
//! record CAS/replay hooks; those shared hooks do not exist yet. Do not commit separate writers.

use oteryn_gameplay_domain::domain::currency::{
    BACKPACK_ENTRIES_MAX, COIN_DEFINITION_FAMILY, Coin, CoinStack, FeePlan, FeePlanError,
    plan_fee_within,
};
use oteryn_gameplay_domain::domain::item_definition::{ItemDefinitionFacts, TypedDefinitionRef};
use oteryn_gameplay_domain::domain::timed_item::TimedValues;
use std::collections::BTreeSet;

/// Plain timed-row facts; the durable adapter supplies the locked row independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairTimedState {
    pub revision: u64,
    pub values: Option<TimedValues>,
    pub deadline_at: Option<i64>,
}

/// Offered target and its paired active full values, independently resolved by the owner.
pub struct RepairTargetFacts {
    pub definition: TypedDefinitionRef,
    pub full: TimedValues,
    pub lit_continuous: bool,
}

/// Compatible coin definitions and backpack capacity; no output identities or authority.
pub struct RepairChangeFacts {
    pub platinum: TypedDefinitionRef,
    pub gold: TypedDefinitionRef,
    pub crystal: TypedDefinitionRef,
    pub backpack: ItemDefinitionFacts,
}

pub const REPAIR_PRICE: u64 = 10_000;
pub const REPAIR_COIN_INPUTS_MAX: usize = 19;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairPlanError {
    InvalidSnapshot,
    ItemMissing,
    InvalidTimedState,
    RevisionExhausted,
    InsufficientFunds,
    BudgetExceeded,
    CoinPlan(FeePlanError),
}

/// Raw read data; `parent == None` is not a direct container entry (e.g. equipment).
#[derive(Debug, Clone)]
pub struct RepairEntry<Timed = RepairTimedState> {
    pub item_instance_id: [u8; 16],
    pub parent: Option<[u8; 16]>,
    pub placement_ordinal: u64,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
    pub live: bool,
    pub has_contents: bool,
    pub timed: Option<Timed>,
}

/// A raw fee-planning input, not the transaction's independently fenced activation/account read.
#[derive(Debug, Clone, Copy)]
pub struct RepairFunds {
    pub bank_eligible: bool,
    pub bank_balance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairPayment {
    /// Native lines index the supplied CoinStack slice; never the full entry slice.
    pub coins: FeePlan,
    pub bank_debit: u64,
}

/// Reuses native coin validation, denomination order, whole/partial burns and change capacity.
pub fn plan_repair_payment(
    stacks: &[CoinStack],
    entries: usize,
    capacity: usize,
    funds: RepairFunds,
) -> Result<RepairPayment, RepairPlanError> {
    if entries > capacity || entries > BACKPACK_ENTRIES_MAX {
        return Err(RepairPlanError::InvalidSnapshot);
    }
    let (coins, bank_debit) = match plan_fee_within(REPAIR_PRICE, stacks, entries, capacity) {
        Ok(coins) => (coins, 0),
        Err(FeePlanError::InsufficientFunds) => {
            if !funds.bank_eligible {
                return Err(RepairPlanError::InsufficientFunds);
            }
            // Native planning validated every stack and unique ordinal before InsufficientFunds.
            let eligible: u64 = stacks
                .iter()
                .map(|stack| u64::from(stack.quantity) * stack.coin.worth())
                .sum();
            let debit = REPAIR_PRICE - eligible;
            if funds.bank_balance < debit {
                return Err(RepairPlanError::InsufficientFunds);
            }
            let coins = if eligible == 0 {
                FeePlan {
                    lines: Vec::new(),
                    change: 0,
                    change_platinum: 0,
                    change_gold: 0,
                }
            } else {
                plan_fee_within(eligible, stacks, entries, capacity)
                    .map_err(RepairPlanError::CoinPlan)?
            };
            (coins, debit)
        }
        Err(error) => return Err(RepairPlanError::CoinPlan(error)),
    };
    if coins.lines.len() > REPAIR_COIN_INPUTS_MAX {
        return Err(RepairPlanError::BudgetExceeded);
    }
    Ok(RepairPayment { coins, bank_debit })
}

/// Counts derived from actual native burn/change lines, plus the repaired item and its work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairUsage {
    pub touched: u64,
    pub participants: u64,
    pub work: u64,
}

impl RepairUsage {
    /// TIMED-ITEM-0 §7; registration and combined-envelope byte measurement remain CP work.
    pub fn check(self) -> Result<(), RepairPlanError> {
        if self.touched > 22 || self.participants > 22 || self.work > 68 {
            Err(RepairPlanError::BudgetExceeded)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub struct RepairPlan<Timed = RepairTimedState> {
    pub item_instance_id: [u8; 16],
    pub placement_ordinal: u64,
    pub before_definition: TypedDefinitionRef,
    pub after_definition: TypedDefinitionRef,
    pub timed_before: Option<Timed>,
    pub timed_after: Timed,
    /// Mapping in the exact order of native payment input indices.
    pub coin_item_ids: Vec<[u8; 16]>,
    pub payment: RepairPayment,
    pub usage: RepairUsage,
}

/// All references are unqualified input facts, including the exact source/target offer binding.
pub struct RepairSnapshot<'a> {
    pub backpack: [u8; 16],
    pub entries: &'a [RepairEntry],
    pub from: &'a TypedDefinitionRef,
    /// Definition is the offered target itself; full values come from its paired active
    /// form when inactive. The caller must resolve that admitted relationship independently.
    pub target: &'a RepairTargetFacts,
    pub change: &'a RepairChangeFacts,
    pub funds: RepairFunds,
}

pub fn plan_soft_boots_repair(snapshot: RepairSnapshot<'_>) -> Result<RepairPlan, RepairPlanError> {
    let capacity = snapshot
        .change
        .backpack
        .container_capacity
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| (1..=BACKPACK_ENTRIES_MAX).contains(value))
        .ok_or(RepairPlanError::InvalidSnapshot)?;
    if snapshot.from == &snapshot.target.definition
        || (snapshot.from.family == COIN_DEFINITION_FAMILY
            && Coin::from_production_key(&snapshot.from.production_key).is_some())
        || snapshot.target.lit_continuous
        || snapshot.target.full.remaining_ms() == Some(0)
    {
        return Err(RepairPlanError::InvalidSnapshot);
    }
    let direct: Vec<_> = snapshot
        .entries
        .iter()
        .filter(|entry| entry.live && entry.parent == Some(snapshot.backpack))
        .collect();
    let mut ids = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    for entry in &direct {
        if entry.item_instance_id == snapshot.backpack
            || entry.placement_ordinal == 0
            || !ids.insert(entry.item_instance_id)
            || !ordinals.insert(entry.placement_ordinal)
        {
            return Err(RepairPlanError::InvalidSnapshot);
        }
    }
    if direct.len() > capacity {
        return Err(RepairPlanError::InvalidSnapshot);
    }
    let repaired = direct
        .iter()
        .filter(|entry| &entry.definition == snapshot.from)
        .max_by_key(|entry| entry.placement_ordinal)
        .ok_or(RepairPlanError::ItemMissing)?;
    if repaired.quantity != 1 || repaired.has_contents {
        return Err(RepairPlanError::InvalidSnapshot);
    }
    let revision = match repaired.timed {
        Some(row) if row.revision == 0 || row.values.is_some() || row.deadline_at.is_some() => {
            return Err(RepairPlanError::InvalidTimedState);
        }
        Some(row) => row.revision,
        None => 0,
    };
    let next_revision = revision
        .checked_add(1)
        .ok_or(RepairPlanError::RevisionExhausted)?;
    let mut stacks = Vec::new();
    let mut coin_item_ids = Vec::new();
    for entry in &direct {
        let Some(coin) = Coin::from_production_key(&entry.definition.production_key) else {
            continue;
        };
        if entry.definition.family != COIN_DEFINITION_FAMILY {
            continue;
        }
        let compatible = match coin {
            Coin::Gold => &snapshot.change.gold,
            Coin::Platinum => &snapshot.change.platinum,
            Coin::Crystal => &snapshot.change.crystal,
        };
        if &entry.definition != compatible || entry.has_contents || entry.timed.is_some() {
            return Err(RepairPlanError::InvalidSnapshot);
        }
        stacks.push(CoinStack {
            coin,
            quantity: entry.quantity,
            placement_ordinal: entry.placement_ordinal,
        });
        coin_item_ids.push(entry.item_instance_id);
    }
    let payment = plan_repair_payment(&stacks, direct.len(), capacity, snapshot.funds)?;
    let inputs = payment.coins.lines.len() as u64;
    let change = payment.coins.change_outputs() as u64;
    if change != 0
        && direct
            .iter()
            .map(|entry| entry.placement_ordinal)
            .max()
            .and_then(|ordinal| ordinal.checked_add(change))
            .is_none()
    {
        return Err(RepairPlanError::CoinPlan(FeePlanError::ChangeDoesNotFit));
    }
    let bank = u64::from(payment.bank_debit != 0);
    let whole = payment
        .coins
        .lines
        .iter()
        .filter(|line| line.burned == stacks[line.input].quantity)
        .count() as u64;
    // Native fee usage formula, plus repair's transform/timed reset/participant work (§7).
    let usage = RepairUsage {
        touched: inputs + change + 1,
        participants: inputs + change + bank + 1,
        work: 3 * whole + 2 * (inputs - whole) + 2 * change + 2 * bank + 4,
    };
    usage.check()?;
    Ok(RepairPlan {
        item_instance_id: repaired.item_instance_id,
        placement_ordinal: repaired.placement_ordinal,
        before_definition: repaired.definition.clone(),
        after_definition: snapshot.target.definition.clone(),
        timed_before: repaired.timed,
        timed_after: RepairTimedState {
            revision: next_revision,
            values: Some(snapshot.target.full),
            deadline_at: None,
        },
        coin_item_ids,
        payment,
        usage,
    })
}
