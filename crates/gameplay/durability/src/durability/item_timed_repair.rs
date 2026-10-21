//! Native timed repair snapshot adapter; planning lives in the pure Item rules crate.

use crate::domain::currency::{BACKPACK_ENTRIES_MAX, CoinStack};
use crate::durability::item_fee_burn::FeeChangeFacts;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_mint_audit::Type2EventTuple;
use crate::durability::item_timed_state::{StoredTimedState, TimedDefinitionFacts};
use oteryn_gameplay_items::item_timed_repair as rules;

pub use rules::*;
pub type RepairEntry = rules::RepairEntry<StoredTimedState>;
pub type RepairPlan = rules::RepairPlan<StoredTimedState>;

/// A raw fee-planning input, not the transaction's independently fenced activation/account read.
#[derive(Debug, Clone, Copy)]
pub struct RepairFunds {
    pub tuple: Type2EventTuple,
    pub junior: bool,
    pub bank_balance: u64,
}

/// All references are unqualified input facts, including the exact source/target offer binding.
pub struct RepairSnapshot<'a> {
    pub backpack: [u8; 16],
    pub entries: &'a [RepairEntry],
    pub from: &'a TypedDefinitionRef,
    /// Definition is the offered target itself; full values come from its paired active
    /// form when inactive. The caller must resolve that admitted relationship independently.
    pub target: &'a TimedDefinitionFacts,
    pub change: &'a FeeChangeFacts,
    pub funds: RepairFunds,
}

fn funds(funds: RepairFunds) -> rules::RepairFunds {
    rules::RepairFunds {
        bank_eligible: funds.tuple != Type2EventTuple::V1 && !funds.junior,
        bank_balance: funds.bank_balance,
    }
}

fn timed(row: StoredTimedState) -> rules::RepairTimedState {
    rules::RepairTimedState {
        revision: row.revision,
        values: row.values,
        deadline_at: row.deadline_at,
    }
}

fn stored(row: rules::RepairTimedState) -> StoredTimedState {
    StoredTimedState {
        revision: row.revision,
        values: row.values,
        deadline_at: row.deadline_at,
    }
}

/// Reuses native coin validation, denomination order, whole/partial burns and change capacity.
pub fn plan_repair_payment(
    stacks: &[CoinStack],
    entries: usize,
    capacity: usize,
    native_funds: RepairFunds,
) -> Result<RepairPayment, RepairPlanError> {
    rules::plan_repair_payment(stacks, entries, capacity, funds(native_funds))
}

pub fn plan_soft_boots_repair(snapshot: RepairSnapshot<'_>) -> Result<RepairPlan, RepairPlanError> {
    // One extra direct entry preserves the core's over-capacity refusal without copying
    // unrelated inventory or an unbounded invalid snapshot.
    let entries: Vec<_> = snapshot
        .entries
        .iter()
        .filter(|entry| entry.live && entry.parent == Some(snapshot.backpack))
        .take(BACKPACK_ENTRIES_MAX + 1)
        .map(|entry| rules::RepairEntry {
            item_instance_id: entry.item_instance_id,
            parent: entry.parent,
            placement_ordinal: entry.placement_ordinal,
            definition: entry.definition.clone(),
            quantity: entry.quantity,
            live: entry.live,
            has_contents: entry.has_contents,
            timed: entry.timed.map(timed),
        })
        .collect();
    let target = rules::RepairTargetFacts {
        definition: snapshot.target.definition.clone(),
        full: snapshot.target.full,
        lit_continuous: snapshot.target.lit_continuous,
    };
    let change = rules::RepairChangeFacts {
        platinum: snapshot.change.platinum.clone(),
        gold: snapshot.change.gold.clone(),
        crystal: snapshot.change.crystal.clone(),
        backpack: snapshot.change.backpack.clone(),
    };
    let plan = rules::plan_soft_boots_repair(rules::RepairSnapshot {
        backpack: snapshot.backpack,
        entries: &entries,
        from: snapshot.from,
        target: &target,
        change: &change,
        funds: funds(snapshot.funds),
    })?;
    Ok(rules::RepairPlan {
        item_instance_id: plan.item_instance_id,
        placement_ordinal: plan.placement_ordinal,
        before_definition: plan.before_definition,
        after_definition: plan.after_definition,
        timed_before: plan.timed_before.map(stored),
        timed_after: stored(plan.timed_after),
        coin_item_ids: plan.coin_item_ids,
        payment: plan.payment,
        usage: plan.usage,
    })
}
