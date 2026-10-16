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
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::domain::premium::PremiumStatus;

    fn uuid(tag: u8) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = tag;
        bytes
    }

    fn build(vocation: &str) -> DurableBuildState {
        DurableBuildState::new(vocation, (0, 0), [(10, 0); 7]).unwrap()
    }

    fn actor(build: &DurableBuildState) -> PreyActor<'_> {
        PreyActor {
            character_id: CharacterId::from_bytes(uuid(1)).unwrap(),
            account_id: AccountId::from_bytes(uuid(2)).unwrap(),
            world_id: WorldId::from_bytes(uuid(3)).unwrap(),
            build,
        }
    }

    fn account(actor: &PreyActor<'_>, wildcards: u32) -> AccountPreyFacts {
        AccountPreyFacts {
            account_id: actor.account_id,
            world_id: actor.world_id,
            wildcards,
            permanent_slot_2: false,
            permanent_slot_3: false,
        }
    }

    fn initialized(actor: &PreyActor<'_>, slot: u8) -> InitializedSlot {
        InitializedSlot {
            character_id: actor.character_id,
            slot,
            option: PreyOption::None,
        }
    }

    #[test]
    fn slot_bounds_are_checked_before_eligibility_facts_balance_or_initialization() {
        let build = build(NO_VOCATION);
        let actor = actor(&build);
        let mut facts = account(&actor, u32::MAX);
        facts.account_id = AccountId::from_bytes(uuid(9)).unwrap();
        for slot in [0, 4, u8::MAX] {
            assert_eq!(
                plan_set_option(
                    slot,
                    &actor,
                    &facts,
                    PremiumGate::CLOSED,
                    None,
                    PreyOption::Lock
                ),
                Err(PreyRefusal::InvalidSlot)
            );
        }
        // Multiple invalid inputs here exercise the explicitly specified error priority only.
    }

    #[test]
    fn slot_one_needs_a_vocation_and_no_premium_or_permanent_purchase() {
        for (vocation, expected) in [
            (NO_VOCATION, Err(PreyRefusal::NotEligible)),
            ("knight", Ok(())),
        ] {
            let build = build(vocation);
            let actor = actor(&build);
            assert_eq!(
                require_slot_access(1, &actor, &account(&actor, 0), PremiumGate::CLOSED),
                expected
            );
        }
    }

    #[test]
    fn slot_two_reads_the_native_current_gate_without_a_pre_delivery_bypass() {
        let build = build("knight");
        let actor = actor(&build);
        let facts = account(&actor, 0);
        assert_eq!(
            require_slot_access(2, &actor, &facts, PremiumStatus::Current.gate()),
            Ok(())
        );
        for closed in [
            PremiumGate::CLOSED,
            PremiumGate::default(),
            PremiumStatus::NotCurrent.gate(),
        ] {
            assert_eq!(
                require_slot_access(2, &actor, &facts, closed),
                Err(PreyRefusal::SlotLocked)
            );
        }
        // These are real native gate classifications, not a live Premium/fence transaction test.
    }

    #[test]
    fn permanent_slots_are_distinct_facts_and_premium_never_opens_slot_three() {
        let build = build("knight");
        let actor = actor(&build);
        let mut facts = account(&actor, 0);
        assert_eq!(
            require_slot_access(3, &actor, &facts, PremiumStatus::Current.gate()),
            Err(PreyRefusal::SlotLocked)
        );
        facts.permanent_slot_2 = true;
        assert!(require_slot_access(2, &actor, &facts, PremiumGate::CLOSED).is_ok());
        assert_eq!(
            require_slot_access(3, &actor, &facts, PremiumGate::CLOSED),
            Err(PreyRefusal::SlotLocked)
        );
        facts.permanent_slot_2 = false;
        facts.permanent_slot_3 = true;
        assert!(require_slot_access(3, &actor, &facts, PremiumGate::CLOSED).is_ok());
        assert_eq!(
            require_slot_access(2, &actor, &facts, PremiumGate::CLOSED),
            Err(PreyRefusal::SlotLocked)
        );
    }

    #[test]
    fn account_and_world_mismatches_cannot_lend_wildcards_or_permanent_slots() {
        let build = build("knight");
        let actor = actor(&build);
        for change_account in [true, false] {
            let mut facts = account(&actor, 52);
            facts.permanent_slot_2 = true;
            if change_account {
                facts.account_id = AccountId::from_bytes(uuid(4)).unwrap();
            } else {
                facts.world_id = WorldId::from_bytes(uuid(4)).unwrap();
            }
            assert_eq!(
                require_slot_access(2, &actor, &facts, PremiumGate::CLOSED),
                Err(PreyRefusal::AccountWorldMismatch)
            );
            assert_eq!(
                plan_wildcard_debit(&actor, &facts, WildcardSpend::SelectAny),
                Err(PreyRefusal::AccountWorldMismatch)
            );
        }
    }

    #[test]
    fn absent_or_mismatched_slot_is_never_initialized_by_a_command() {
        let build = build("knight");
        let actor = actor(&build);
        let facts = account(&actor, 5);
        assert_eq!(
            plan_set_option(
                1,
                &actor,
                &facts,
                PremiumGate::CLOSED,
                None,
                PreyOption::Lock
            ),
            Err(PreyRefusal::SlotLocked)
        );
        for wrong_character in [true, false] {
            let mut row = initialized(&actor, 1);
            if wrong_character {
                row.character_id = CharacterId::from_bytes(uuid(4)).unwrap();
            } else {
                row.slot = 2;
            }
            assert_eq!(
                plan_set_option(
                    1,
                    &actor,
                    &facts,
                    PremiumGate::CLOSED,
                    Some(&row),
                    PreyOption::Lock
                ),
                Err(PreyRefusal::SlotBindingMismatch)
            );
        }
    }

    #[test]
    fn setting_each_option_checks_the_exact_hold_and_debits_nothing() {
        let build = build("knight");
        let actor = actor(&build);
        let row = initialized(&actor, 1);
        for (option, minimum) in [
            (PreyOption::None, 0),
            (PreyOption::AutoReroll, 1),
            (PreyOption::Lock, 5),
        ] {
            let facts = account(&actor, minimum);
            let plan = plan_set_option(1, &actor, &facts, PremiumGate::CLOSED, Some(&row), option)
                .unwrap();
            assert_eq!(plan.before, PreyOption::None);
            assert_eq!(plan.after, option);
            assert_eq!(plan.wildcards_before, minimum);
            assert_eq!(plan.wildcards_after, minimum);
            assert_eq!(
                (plan.character_id, plan.account_id, plan.world_id, plan.slot),
                (actor.character_id, actor.account_id, actor.world_id, 1)
            );
            if minimum > 0 {
                assert_eq!(
                    plan_set_option(
                        1,
                        &actor,
                        &account(&actor, minimum - 1),
                        PremiumGate::CLOSED,
                        Some(&row),
                        option
                    ),
                    Err(PreyRefusal::NotEnoughWildcards)
                );
            }
        }
    }

    #[test]
    fn locked_slot_preserves_its_loaded_row_and_spends_nothing() {
        let build = build("knight");
        let actor = actor(&build);
        let facts = account(&actor, 5);
        let mut row = initialized(&actor, 2);
        row.option = PreyOption::Lock;
        assert_eq!(
            plan_set_option(
                2,
                &actor,
                &facts,
                PremiumGate::CLOSED,
                Some(&row),
                PreyOption::None
            ),
            Err(PreyRefusal::SlotLocked)
        );
        assert_eq!(row.option, PreyOption::Lock);
        assert_eq!(facts.wildcards, 5);
    }

    #[test]
    fn each_closed_spend_kind_has_its_accepted_cost_and_exact_zero_floor() {
        let build = build("knight");
        let actor = actor(&build);
        for (spend, cost) in [
            (WildcardSpend::BonusReroll, 1),
            (WildcardSpend::SelectAny, 5),
            (WildcardSpend::AutoRerollExpiry, 1),
            (WildcardSpend::LockExpiry, 5),
        ] {
            let plan = plan_wildcard_debit(&actor, &account(&actor, cost), spend).unwrap();
            assert_eq!((plan.before, plan.amount, plan.after), (cost, cost, 0));
            assert_eq!(
                (plan.acting_character_id, plan.account_id, plan.world_id),
                (actor.character_id, actor.account_id, actor.world_id)
            );
            assert_eq!(
                plan_wildcard_debit(&actor, &account(&actor, cost - 1), spend),
                Err(PreyRefusal::NotEnoughWildcards)
            );
        }
    }

    #[test]
    fn balance_cap_is_checked_without_clamping_even_for_clearing_an_option() {
        let build = build("knight");
        let actor = actor(&build);
        let row = initialized(&actor, 1);
        assert_eq!(
            plan_wildcard_debit(&actor, &account(&actor, 52), WildcardSpend::BonusReroll)
                .unwrap()
                .after,
            51
        );
        for balance in [53, u32::MAX] {
            let facts = account(&actor, balance);
            assert_eq!(
                plan_wildcard_debit(&actor, &facts, WildcardSpend::BonusReroll),
                Err(PreyRefusal::InvalidWildcardBalance)
            );
            assert_eq!(
                plan_set_option(
                    1,
                    &actor,
                    &facts,
                    PremiumGate::CLOSED,
                    Some(&row),
                    PreyOption::None
                ),
                Err(PreyRefusal::InvalidWildcardBalance)
            );
        }
    }

    #[test]
    fn another_character_spend_requires_rechecking_an_option_at_expiry() {
        let build = build("knight");
        let first = actor(&build);
        let mut other = actor(&build);
        other.character_id = CharacterId::from_bytes(uuid(4)).unwrap();
        let mut facts = account(&first, 5);
        assert!(
            plan_set_option(
                1,
                &first,
                &facts,
                PremiumGate::CLOSED,
                Some(&initialized(&first, 1)),
                PreyOption::Lock
            )
            .is_ok()
        );
        let spent = plan_wildcard_debit(&other, &facts, WildcardSpend::BonusReroll).unwrap();
        facts.wildcards = spent.after;
        assert_eq!(
            plan_wildcard_debit(&first, &facts, WildcardSpend::LockExpiry),
            Err(PreyRefusal::NotEnoughWildcards)
        );
        // Pure changed-input check, not executed concurrency, locking or expiry's fallback draw.
    }
}
