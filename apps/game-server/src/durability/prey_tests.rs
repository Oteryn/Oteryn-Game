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
        let plan =
            plan_set_option(1, &actor, &facts, PremiumGate::CLOSED, Some(&row), option).unwrap();
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
