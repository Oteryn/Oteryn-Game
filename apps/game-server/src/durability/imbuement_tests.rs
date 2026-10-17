#![allow(clippy::expect_used)]

use crate::domain::imbuement::{
    ApplyRoute, MAX_REMAINING_MS, OutstandingAccess, SlotRecord, StateError, Tier,
    check_target_slot, outstanding_access,
};
use crate::domain::imbuement_checkpoint::*;
use crate::domain::premium::{PremiumGate, PremiumRefusal};

#[path = "imbuement_test_fixtures.rs"]
mod fixtures;
use fixtures::*;

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
    // The public native constructor delegates to the same domain UUID validator.
    // Cover each occurrence field independently, including nil and version/variant edges.
    for field in 0..2 {
        let mut nil = binding(1, 0);
        if field == 0 {
            nil.applying_occurrence = [0; 16];
        } else {
            nil.transaction_id = [0; 16];
        }
        assert_eq!(
            SlotRecord::new(nil, 1).err(),
            Some(StateError::InvalidBinding)
        );
        for (version, valid_version) in [
            (0x00_u8, false),
            (0x6f, false),
            (0x70, true),
            (0x7f, true),
            (0x80, false),
            (0xff, false),
        ] {
            for (variant, valid_variant) in [
                (0x00_u8, false),
                (0x7f, false),
                (0x80, true),
                (0xbf, true),
                (0xc0, false),
                (0xff, false),
            ] {
                let mut raw = binding(1, 0);
                let occurrence = if field == 0 {
                    &mut raw.applying_occurrence
                } else {
                    &mut raw.transaction_id
                };
                occurrence[6] = version;
                occurrence[8] = variant;
                assert_eq!(
                    SlotRecord::new(raw, 1).is_ok(),
                    valid_version && valid_variant
                );
            }
        }
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
fn missing_or_invalid_slot_rules_fail_closed() {
    assert_eq!(
        check_target_slot(item(1), &[], None, 0, "category"),
        Err(StateError::UnresolvedItemRules)
    );
    assert_eq!(
        check_target_slot(item(1), &[], Some(0), 0, "category"),
        Err(StateError::SlotUnavailable)
    );
    assert_eq!(
        check_target_slot(item(1), &[], Some(4), 0, "category"),
        Err(StateError::InvalidItemRules)
    );
    for category in ["", "   ", "category\n"] {
        assert_eq!(
            check_target_slot(item(1), &[], Some(3), 0, category),
            Err(StateError::InvalidItemRules)
        );
    }
}

#[test]
fn retained_rows_outside_declared_slots_invalidate_the_entire_snapshot() {
    // Slot 0 and its new category are available on the nonzero declarations. A retained
    // out-of-range row must nevertheless refuse the whole snapshot, including at expiry.
    for (declared_slots, retained_slot) in [(1, 1), (1, 2), (2, 2), (0, 0), (0, 1), (0, 2)] {
        for remaining_ms in [0, 1, MAX_REMAINING_MS] {
            let retained = row(binding(1, retained_slot), remaining_ms);
            assert_eq!(
                check_target_slot(item(1), &[retained], Some(declared_slots), 0, "other"),
                Err(StateError::InvalidSnapshot),
                "declared={declared_slots}, retained={retained_slot}, remaining={remaining_ms}",
            );
        }
    }
    // The highest valid retained slot still permits another available slot. Declared
    // counts are counts, not inclusive slot indices; retained zero-time rows stay present.
    for (declared_slots, retained_slot) in [(2, 1), (3, 2)] {
        for remaining_ms in [0, MAX_REMAINING_MS] {
            let retained = row(binding(1, retained_slot), remaining_ms);
            assert_eq!(
                check_target_slot(item(1), &[retained], Some(declared_slots), 0, "other"),
                Ok(()),
            );
        }
    }
}

#[test]
fn every_tier_fits_a_slot_bearing_item_while_shrine_premium_stays_independent() {
    // Native pure flow helpers, not current session/quest/content authority for a mutation.
    let premium = PremiumGate::from_fence_read(true);
    let free = PremiumGate::from_fence_read(false);
    for tier in [Tier::Basic, Tier::Intricate, Tier::Powerful] {
        let mut proposed = binding(1, 0);
        proposed.tier = tier;
        let candidate = row(proposed, 1_000);
        let proposed = candidate.binding();
        assert_eq!(
            check_target_slot(
                proposed.item,
                &[],
                Some(1),
                proposed.slot,
                &proposed.category
            ),
            Ok(())
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, proposed.tier, premium),
            Ok(OutstandingAccess::QuestAndContentBindings)
        );
        assert_eq!(
            outstanding_access(ApplyRoute::Shrine, proposed.tier, free),
            if tier == Tier::Basic {
                Ok(OutstandingAccess::QuestAndContentBindings)
            } else {
                Err(PremiumRefusal::NotCurrent)
            }
        );
    }
}

#[test]
fn zero_time_rows_still_occupy_their_slot_and_category_until_audited_expiry() {
    let expired = row(binding(1, 0), 0);
    assert_eq!(
        check_target_slot(item(1), std::slice::from_ref(&expired), Some(3), 0, "other"),
        Err(StateError::SlotUnavailable)
    );
    assert_eq!(
        check_target_slot(
            item(1),
            std::slice::from_ref(&expired),
            Some(3),
            1,
            &expired.binding().category,
        ),
        Err(StateError::CategoryOccupied)
    );
    assert_eq!(
        check_target_slot(item(1), &[expired], Some(3), 1, "other"),
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
            check_target_slot(item(1), &rows, Some(3), 2, "other"),
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
