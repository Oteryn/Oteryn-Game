//! PREY0 §§4–5: nonrandom access, option holds and Account+World wildcard debit preparation.
//!
//! Inputs are caller-qualified facts, not current authority. The future owner must load the
//! initialized, content-validated slot and current build/account facts under its transaction
//! locks, inside the real Premium gate. A matching Account/World/Character binding does not
//! establish liveness, session fencing or content activation. Plans are neither reservations
//! nor receipts, and cannot replace PREY-1's sequencer, ledger, guard or outbox integration.
//!
//! PREY-CONTENT-1 is absent: no list/bonus draw, activation, time tick, fee or Store claim is
//! implemented. Fixed costs below are accepted PREY0 §5.2/§12 values, not inferred tables.

use crate::domain::premium::PremiumGate;
use crate::domain::{AccountId, CharacterId, WorldId};
use crate::durability::character_build::{DurableBuildState, NO_VOCATION};

pub(crate) const WILDCARDS_MAX: u32 = 52;

/// A projection of the admitted actor's current build and Account+World binding.
pub(crate) struct PreyActor<'a> {
    pub character_id: CharacterId,
    pub account_id: AccountId,
    pub world_id: WorldId,
    pub build: &'a DurableBuildState,
}

/// Already loaded Account+World facts, not character-local balances or Store entitlements.
/// A proven missing balance row supplies zero (§4.1); unknown storage is not a zero snapshot.
pub(crate) struct AccountPreyFacts {
    pub account_id: AccountId,
    pub world_id: WorldId,
    pub wildcards: u32,
    pub permanent_slot_2: bool,
    pub permanent_slot_3: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreyOption {
    None,
    AutoReroll,
    Lock,
}

/// Only fields used by set_option; not a complete durable slot schema. Presence means the
/// owning loader validated an initialized slot. This helper never initializes or repairs it.
pub(crate) struct InitializedSlot {
    pub character_id: CharacterId,
    pub slot: u8,
    pub option: PreyOption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreyRefusal {
    InvalidSlot,
    NotEligible,
    SlotLocked,
    AccountWorldMismatch,
    SlotBindingMismatch,
    InvalidWildcardBalance,
    NotEnoughWildcards,
}

/// Validate the command's explicit slot before any other check (§5.2). Premium is the actual
/// native at-use gate; closed/unknown Premium has no historical pre-delivery bypass here.
pub(crate) fn require_slot_access(
    slot: u8,
    actor: &PreyActor<'_>,
    account: &AccountPreyFacts,
    premium: PremiumGate,
) -> Result<(), PreyRefusal> {
    if !(1..=3).contains(&slot) {
        return Err(PreyRefusal::InvalidSlot);
    }
    require_account_world(actor, account)?;
    if actor.build.vocation() == NO_VOCATION {
        return Err(PreyRefusal::NotEligible);
    }
    match slot {
        2 if !account.permanent_slot_2 && premium.require().is_err() => {
            Err(PreyRefusal::SlotLocked)
        }
        3 if !account.permanent_slot_3 => Err(PreyRefusal::SlotLocked),
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SetOptionPlan {
    pub character_id: CharacterId,
    pub account_id: AccountId,
    pub world_id: WorldId,
    pub slot: u8,
    pub before: PreyOption,
    pub after: PreyOption,
    pub wildcards_before: u32,
    pub wildcards_after: u32,
}

/// AUTO_REROLL holds at least one wildcard; LOCK holds five. Setting an option spends none
/// and reserves none (§5.2): another character may spend them before expiry. Both SELECTION
/// and ACTIVE initialized rows are supported; no extra ACTIVE-only rule is invented.
pub(crate) fn plan_set_option(
    slot: u8,
    actor: &PreyActor<'_>,
    account: &AccountPreyFacts,
    premium: PremiumGate,
    initialized: Option<&InitializedSlot>,
    option: PreyOption,
) -> Result<SetOptionPlan, PreyRefusal> {
    require_slot_access(slot, actor, account, premium)?;
    let initialized = initialized.ok_or(PreyRefusal::SlotLocked)?;
    if initialized.character_id != actor.character_id || initialized.slot != slot {
        return Err(PreyRefusal::SlotBindingMismatch);
    }
    require_balance(account.wildcards)?;
    let required = match option {
        PreyOption::None => 0,
        PreyOption::AutoReroll => 1,
        PreyOption::Lock => 5,
    };
    if account.wildcards < required {
        return Err(PreyRefusal::NotEnoughWildcards);
    }
    Ok(SetOptionPlan {
        character_id: actor.character_id,
        account_id: actor.account_id,
        world_id: actor.world_id,
        slot,
        before: initialized.option,
        after: option,
        wildcards_before: account.wildcards,
        wildcards_after: account.wildcards,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WildcardSpend {
    BonusReroll,
    SelectAny,
    AutoRerollExpiry,
    LockExpiry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WildcardDebit {
    pub acting_character_id: CharacterId,
    pub account_id: AccountId,
    pub world_id: WorldId,
    pub before: u32,
    pub amount: u32,
    pub after: u32,
}

/// Arithmetic only, after the owning command/expiry and draw/content checks. This does not
/// authorize a command or model expiry's insufficient-funds fallback. Recompute under the
/// Account+World row lock and compose one SPEND entry, event and character receipt atomically.
/// There is deliberately no credit/Store/test-grant API or arbitrary amount parameter.
pub(crate) fn plan_wildcard_debit(
    actor: &PreyActor<'_>,
    account: &AccountPreyFacts,
    spend: WildcardSpend,
) -> Result<WildcardDebit, PreyRefusal> {
    require_account_world(actor, account)?;
    require_balance(account.wildcards)?;
    let amount = match spend {
        WildcardSpend::BonusReroll | WildcardSpend::AutoRerollExpiry => 1,
        WildcardSpend::SelectAny | WildcardSpend::LockExpiry => 5,
    };
    let after = account
        .wildcards
        .checked_sub(amount)
        .ok_or(PreyRefusal::NotEnoughWildcards)?;
    Ok(WildcardDebit {
        acting_character_id: actor.character_id,
        account_id: actor.account_id,
        world_id: actor.world_id,
        before: account.wildcards,
        amount,
        after,
    })
}

fn require_account_world(
    actor: &PreyActor<'_>,
    account: &AccountPreyFacts,
) -> Result<(), PreyRefusal> {
    if actor.account_id != account.account_id || actor.world_id != account.world_id {
        return Err(PreyRefusal::AccountWorldMismatch);
    }
    Ok(())
}

fn require_balance(balance: u32) -> Result<(), PreyRefusal> {
    if balance > WILDCARDS_MAX {
        return Err(PreyRefusal::InvalidWildcardBalance);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../durability/prey_tests.rs"]
mod tests;
