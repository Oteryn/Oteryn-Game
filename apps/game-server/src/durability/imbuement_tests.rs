#![allow(clippy::expect_used)]

use super::*;
use crate::domain::imbuement::{
    ApplyRoute, MAX_REMAINING_MS, OutstandingAccess, SlotBinding, StateError, Tier,
    check_target_slot, outstanding_access,
};
use crate::domain::premium::{PremiumGate, PremiumRefusal};

fn uuid(tag: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = tag;
    value
}

fn item(tag: u8) -> ItemInstanceId {
    ItemInstanceId::from_bytes(uuid(tag)).expect("native UUIDv7 constructor")
}

fn binding(tag: u8, slot: u8) -> SlotBinding {
    // Synthetic raw row bindings, not admitted definitions or current authority.
    let mut applying_occurrence = uuid(100);
    applying_occurrence[0] = tag;
    applying_occurrence[1] = slot;
    let mut transaction_id = uuid(101);
    transaction_id[0] = tag;
    transaction_id[1] = slot;
    SlotBinding {
        item: item(tag),
        slot,
        key: format!("fixture:imbue-{slot}"),
        content_revision: "definition-r1".into(),
        category: format!("fixture-category-{slot}"),
        tier: Tier::Basic,
        applying_occurrence,
        transaction_id,
    }
}

fn row(binding: SlotBinding, remaining: u64) -> SlotRecord {
    SlotRecord::new(binding, remaining).expect("structural row fixture")
}

fn change(tag: u8, slot: u8, before: u64, after: u64) -> SlotCheckpoint {
    SlotCheckpoint {
        before: row(binding(tag, slot), before),
        after: row(binding(tag, slot), after),
    }
}

#[test]
fn structural_rows_accept_exact_bounds_and_reject_max_plus_one_or_invalid_bindings() {
    assert!(SlotRecord::new(binding(1, 2), 0).is_ok());
    assert!(SlotRecord::new(binding(1, 2), MAX_REMAINING_MS).is_ok());
    assert_eq!(
        SlotRecord::new(binding(1, 3), 1).err(),
        Some(StateError::InvalidBinding)
    );
    assert_eq!(
        SlotRecord::new(binding(1, 0), MAX_REMAINING_MS + 1).err(),
        Some(StateError::InvalidRemaining)
    );
    assert_eq!(
        SlotRecord::new(binding(1, 0), u64::MAX).err(),
        Some(StateError::InvalidRemaining)
    );
    for mutation in 0..5 {
        let mut raw = binding(1, 0);
        match mutation {
            0 => raw.key.clear(),
            1 => raw.content_revision = " ".into(),
            2 => raw.category.push('\n'),
            3 => raw.applying_occurrence = [0; 16],
            _ => raw.transaction_id[6] = 0,
        }
        assert_eq!(
            SlotRecord::new(raw, 1).err(),
            Some(StateError::InvalidBinding)
        );
    }
}

#[test]
fn native_premium_is_only_a_subcheck_and_no_result_grants_quest_or_content_access() {
    // Classification data for a pure check, not a current fence fixture for a mutation.
    let open = PremiumGate::from_fence_read(true);
    for tier in [Tier::Basic, Tier::Intricate, Tier::Powerful] {
        assert_eq!(
            outstanding_access(ApplyRoute::ScrollApply, tier, PremiumGate::CLOSED),
            Ok(OutstandingAccess::ItemAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::ScrollCreate, tier, PremiumGate::CLOSED),
            Err(PremiumRefusal::NotCurrent)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::ScrollCreate, tier, open),
            Ok(OutstandingAccess::QuestAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, tier, open),
            Ok(OutstandingAccess::QuestAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, tier, PremiumGate::default()),
            if tier == Tier::Basic {
                Ok(OutstandingAccess::QuestAndContentBindings)
            } else {
                Err(PremiumRefusal::NotCurrent)
            }
        );
    }
    assert_eq!(
        outstanding_access(
            ApplyRoute::Shrine,
            Tier::Powerful,
            PremiumGate::from_fence_read(false)
        ),
        Err(PremiumRefusal::NotCurrent)
    );
}

#[test]
fn missing_item_rules_fail_closed_and_basic_maximum_is_not_coerced_to_two() {
    for (slots, maximum) in [(None, Some(Tier::Basic)), (Some(3), None), (None, None)] {
        assert_eq!(
            check_target_slot(item(1), &[], slots, maximum, 0, "category", Tier::Basic),
            Err(StateError::UnresolvedItemRules)
        );
    }
    assert_eq!(
        check_target_slot(
            item(1),
            &[],
            Some(3),
            Some(Tier::Basic),
            0,
            "category",
            Tier::Basic
        ),
        Ok(())
    );
    assert_eq!(
        check_target_slot(
            item(1),
            &[],
            Some(3),
            Some(Tier::Basic),
            0,
            "category",
            Tier::Intricate
        ),
        Err(StateError::TierNotAllowed)
    );
    assert_eq!(
        check_target_slot(
            item(1),
            &[],
            Some(0),
            Some(Tier::Powerful),
            0,
            "category",
            Tier::Basic
        ),
        Err(StateError::SlotUnavailable)
    );
    assert_eq!(
        check_target_slot(
            item(1),
            &[],
            Some(4),
            Some(Tier::Basic),
            0,
            "category",
            Tier::Basic
        ),
        Err(StateError::InvalidItemRules)
    );
}

#[test]
fn zero_time_rows_still_occupy_their_slot_and_category_until_audited_expiry() {
    let expired = row(binding(1, 0), 0);
    assert_eq!(
        check_target_slot(
            item(1),
            std::slice::from_ref(&expired),
            Some(3),
            Some(Tier::Powerful),
            0,
            "other",
            Tier::Basic
        ),
        Err(StateError::SlotUnavailable)
    );
    assert_eq!(
        check_target_slot(
            item(1),
            std::slice::from_ref(&expired),
            Some(3),
            Some(Tier::Powerful),
            1,
            &expired.binding().category,
            Tier::Basic
        ),
        Err(StateError::CategoryOccupied)
    );
    assert_eq!(
        check_target_slot(
            item(1),
            &[expired],
            Some(3),
            Some(Tier::Powerful),
            1,
            "other",
            Tier::Powerful
        ),
        Ok(())
    );
}

#[test]
fn wrong_item_duplicate_slots_and_duplicate_categories_are_invalid_snapshots() {
    let first = row(binding(1, 0), 100);
    let mut same_category = binding(1, 1);
    same_category.category = first.binding().category.clone();
    let mut same_slot = binding(1, 1);
    same_slot.slot = 0;
    for rows in [
        vec![row(binding(2, 0), 100)],
        vec![first.clone(), row(same_slot, 100)],
        vec![first.clone(), row(same_category, 100)],
    ] {
        assert_eq!(
            check_target_slot(
                item(1),
                &rows,
                Some(3),
                Some(Tier::Powerful),
                2,
                "other",
                Tier::Basic
            ),
            Err(StateError::InvalidSnapshot)
        );
    }
}

#[test]
fn each_immutable_binding_field_is_compared_before_planning_a_checkpoint() {
    let before = row(binding(1, 0), 1_000);
    for mutation in 0..8 {
        let mut changed = before.binding().clone();
        match mutation {
            0 => changed.item = item(2),
            1 => changed.slot = 1,
            2 => changed.key = "fixture:other".into(),
            3 => changed.content_revision = "definition-r2".into(),
            4 => changed.category = "other".into(),
            5 => changed.tier = Tier::Intricate,
            6 => changed.applying_occurrence = uuid(102),
            _ => changed.transaction_id = uuid(103),
        }
        let candidate = SlotCheckpoint {
            before: before.clone(),
            after: row(changed, 999),
        };
        assert_eq!(
            CheckpointBatch::plan(vec![candidate]).err(),
            Some(CheckpointError::BindingChanged)
        );
    }
}

#[test]
fn checkpoints_never_increase_time_and_noop_snapshots_produce_no_mutation() {
    assert_eq!(
        CheckpointBatch::plan(vec![change(1, 0, 100, 101)]).err(),
        Some(CheckpointError::TimeIncreased)
    );
    let empty = CheckpointBatch::plan(vec![change(1, 0, 100, 100)]).expect("no change");
    assert!(empty.items.is_empty());
    assert_eq!(
        empty.usage,
        CheckpointUsage {
            items: 0,
            participants: 0,
            work: 0
        }
    );
    let zero =
        CheckpointBatch::plan(vec![change(1, 0, 1, 0)]).expect("zero stays a row, no delete");
    assert_eq!(zero.items[0].changes[0].after.remaining_ms(), 0);
    assert_eq!(zero.usage.work, 2);
}

#[test]
fn duplicate_checkpoint_slots_or_categories_cannot_mutate_a_row_twice() {
    let first = change(1, 0, 100, 99);
    assert_eq!(
        CheckpointBatch::plan(vec![first.clone(), first.clone()]).err(),
        Some(CheckpointError::DuplicateSlot)
    );
    let mut other_binding = binding(1, 1);
    other_binding.category = first.before.binding().category.clone();
    let second = SlotCheckpoint {
        before: row(other_binding.clone(), 100),
        after: row(other_binding, 99),
    };
    assert_eq!(
        CheckpointBatch::plan(vec![first, second]).err(),
        Some(CheckpointError::DuplicateCategory)
    );
}

#[test]
fn ten_items_thirty_rows_forty_work_are_accepted_in_deterministic_item_slot_order() {
    let changes: Vec<_> = (1..=10)
        .flat_map(|tag| (0..3).map(move |slot| change(tag, slot, 72_000_000, 71_940_000)))
        .collect();
    let expected = CheckpointBatch::plan(changes.clone()).expect("registered maxima");
    let mut reversed = changes;
    reversed.reverse();
    assert_eq!(
        CheckpointBatch::plan(reversed).expect("same batch order"),
        expected
    );
    assert_eq!(
        expected.usage,
        CheckpointUsage {
            items: 10,
            participants: 10,
            work: 40
        }
    );
    assert_eq!(
        expected.items.first().map(|group| group.item),
        Some(item(1))
    );
    assert_eq!(
        expected.items.last().map(|group| group.item),
        Some(item(10))
    );
    assert!(expected.items.iter().all(|group| {
        group
            .changes
            .iter()
            .map(|delta| delta.before.binding().slot)
            .eq([0, 1, 2])
    }));
}

#[test]
fn eleven_items_or_thirty_one_input_rows_are_refused_before_any_output() {
    for count in [11, 31] {
        let changes = (1..=count).map(|tag| change(tag, 0, 100, 99)).collect();
        assert_eq!(
            CheckpointBatch::plan(changes).err(),
            Some(CheckpointError::BudgetExceeded)
        );
    }
}

#[test]
fn each_numeric_ceiling_refuses_maximum_plus_one_and_extreme_counts() {
    assert_eq!(
        CheckpointUsage {
            items: 10,
            participants: 10,
            work: 40
        }
        .check(),
        Ok(())
    );
    for usage in [
        CheckpointUsage {
            items: 11,
            participants: 10,
            work: 40,
        },
        CheckpointUsage {
            items: 10,
            participants: 11,
            work: 40,
        },
        CheckpointUsage {
            items: 10,
            participants: 10,
            work: 41,
        },
        CheckpointUsage {
            items: usize::MAX,
            participants: 10,
            work: 40,
        },
    ] {
        assert_eq!(usage.check(), Err(CheckpointError::BudgetExceeded));
    }
}

#[test]
fn checkpoint_does_not_rebind_old_applications_to_a_global_active_revision_or_restart_time() {
    let first = change(1, 0, 100, 80);
    let mut raw = binding(2, 0);
    raw.content_revision = "older-definition-revision".into();
    raw.applying_occurrence = uuid(104);
    let second = SlotCheckpoint {
        before: row(raw.clone(), 9),
        after: row(raw, 8),
    };
    let batch =
        CheckpointBatch::plan(vec![second, first]).expect("each existing row binds nothing new");
    assert_eq!(batch.items[0].changes[0].after.remaining_ms(), 80);
    assert_eq!(
        batch.items[1].changes[0].after.binding().content_revision,
        "older-definition-revision"
    );
    assert_eq!(batch.items[1].changes[0].after.remaining_ms(), 8);
}
