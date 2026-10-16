#![allow(clippy::expect_used)]
//! Native planning tests only despite the allocated filename. No PostgreSQL/writer result.

pub use oteryn_game_server::{domain, durability};

#[path = "../src/durability/item_timed_repair.rs"]
mod item_timed_repair;

use self::domain::currency::{Coin, CoinStack, FeePlanError, plan_fee_within};
use self::domain::timed_item::TimedValues;
use self::durability::item_fee_burn::FeeChangeFacts;
use self::durability::item_mint::TypedDefinitionRef;
use self::durability::item_mint_audit::Type2EventTuple;
use self::durability::item_timed_state::{StoredTimedState, TimedDefinitionFacts};
use self::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};
use item_timed_repair::*;

fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tag;
    bytes
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "fixture-r1".into(),
    }
}

fn entry(tag: u8, ordinal: u64, definition: TypedDefinitionRef) -> RepairEntry {
    RepairEntry {
        item_instance_id: id(tag),
        parent: Some(id(1)),
        placement_ordinal: ordinal,
        definition,
        quantity: 1,
        live: true,
        has_contents: false,
        timed: None,
    }
}

fn funds(tuple: Type2EventTuple, junior: bool, bank_balance: u64) -> RepairFunds {
    RepairFunds {
        tuple,
        junior,
        bank_balance,
    }
}

struct Fixture {
    from: TypedDefinitionRef,
    target: TimedDefinitionFacts,
    change: FeeChangeFacts,
    entries: Vec<RepairEntry>,
}

impl Fixture {
    fn new() -> Self {
        // Synthetic definition keys and values are not an admitted production offer.
        let from = definition("fixture:worn");
        let target = TimedDefinitionFacts {
            definition: definition("fixture:inactive-target"),
            full: TimedValues::new(Some(5), Some(14_400_000)).expect("native bounded values"),
            lit_continuous: false,
        };
        let change = FeeChangeFacts {
            item_instance_ids: [id(90), id(91)],
            platinum: definition(Coin::Platinum.production_key()),
            gold: definition(Coin::Gold.production_key()),
            crystal: definition(Coin::Crystal.production_key()),
            backpack: ItemDefinitionFacts {
                definition: definition("fixture:backpack"),
                stack: ItemStackClass::NonStackable,
                container_capacity: Some(20),
                container_slot_equip_pattern: true,
            },
        };
        let entries = vec![
            entry(2, 10, from.clone()),
            entry(3, 20, change.crystal.clone()),
        ];
        Self {
            from,
            target,
            change,
            entries,
        }
    }

    fn plan(&self, funds: RepairFunds) -> Result<RepairPlan, RepairPlanError> {
        plan_soft_boots_repair(RepairSnapshot {
            backpack: id(1),
            entries: &self.entries,
            from: &self.from,
            target: &self.target,
            change: &self.change,
            funds,
        })
    }

    fn coin_only(&self) -> Result<RepairPlan, RepairPlanError> {
        self.plan(funds(Type2EventTuple::V1, false, 0))
    }
}

#[test]
fn first_b3_direct_match_wins_independently_of_input_order_or_item_id() {
    let mut f = Fixture::new();
    f.entries.push(entry(8, 30, f.from.clone()));
    let mut nested = entry(9, 100, f.from.clone());
    nested.parent = Some(id(7));
    f.entries.push(nested);
    let mut equipped = entry(10, 200, f.from.clone());
    equipped.parent = None;
    f.entries.push(equipped);
    let mut gold = entry(11, 40, f.change.gold.clone());
    gold.quantity = 50;
    f.entries.push(gold);
    let first = f.coin_only().expect("native fee paid");
    f.entries.reverse();
    let reversed = f.coin_only().expect("same B3 order");
    assert_eq!(first.item_instance_id, id(8));
    assert_eq!(reversed.item_instance_id, id(8));
    assert_eq!(first.placement_ordinal, 30);
    for plan in [&first, &reversed] {
        let burned: Vec<_> = plan
            .payment
            .coins
            .lines
            .iter()
            .map(|line| plan.coin_item_ids[line.input])
            .collect();
        assert_eq!(burned, vec![id(11), id(3)]);
    }
}

#[test]
fn nested_equipped_retired_or_wrong_revision_item_is_missing() {
    let baseline = Fixture::new();
    for mutation in 0..4 {
        let mut f = Fixture::new();
        match mutation {
            0 => f.entries[0].parent = Some(id(7)),
            1 => f.entries[0].parent = None,
            2 => f.entries[0].live = false,
            _ => f.entries[0].definition.revision_ref = "other-r1".into(),
        }
        assert_eq!(f.coin_only().err(), Some(RepairPlanError::ItemMissing));
    }
    assert!(baseline.coin_only().is_ok());
}

#[test]
fn absent_and_spent_rows_reset_to_full_native_target_values_and_keep_instance() {
    for before in [
        None,
        Some(StoredTimedState {
            revision: 9,
            values: None,
            deadline_at: None,
        }),
    ] {
        let mut f = Fixture::new();
        f.entries[0].timed = before;
        let plan = f.coin_only().expect("repair plan");
        assert_eq!(plan.item_instance_id, id(2));
        assert_eq!(plan.placement_ordinal, 10);
        assert_eq!(plan.before_definition, f.from);
        assert_eq!(plan.after_definition, f.target.definition);
        assert_eq!(plan.timed_before, before);
        assert_eq!(
            plan.timed_after.revision,
            before.map_or(1, |row| row.revision + 1)
        );
        assert_eq!(plan.timed_after.values, Some(f.target.full));
        assert_eq!(plan.timed_after.deadline_at, None);
        assert_eq!(plan.coin_item_ids, vec![id(3)]);
        assert_eq!(f.entries[0].timed, before); // Planning does not mutate the snapshot.
    }
}

#[test]
fn nonspent_or_zero_revision_rows_are_invalid_and_revision_exhaustion_is_refused() {
    let full = Fixture::new().target.full;
    for row in [
        StoredTimedState {
            revision: 0,
            values: None,
            deadline_at: None,
        },
        StoredTimedState {
            revision: 1,
            values: Some(full),
            deadline_at: None,
        },
        StoredTimedState {
            revision: 1,
            values: None,
            deadline_at: Some(100),
        },
    ] {
        let mut f = Fixture::new();
        f.entries[0].timed = Some(row);
        assert_eq!(
            f.coin_only().err(),
            Some(RepairPlanError::InvalidTimedState)
        );
    }
    let mut f = Fixture::new();
    f.entries[0].timed = Some(StoredTimedState {
        revision: u64::MAX,
        values: None,
        deadline_at: None,
    });
    assert_eq!(
        f.coin_only().err(),
        Some(RepairPlanError::RevisionExhausted)
    );
}

#[test]
fn nineteen_native_coin_inputs_pay_but_twenty_consumed_inputs_are_refused() {
    let mut nineteen: Vec<_> = (0..18)
        .map(|index| CoinStack {
            coin: Coin::Gold,
            quantity: 100,
            placement_ordinal: index + 1,
        })
        .collect();
    nineteen.push(CoinStack {
        coin: Coin::Platinum,
        quantity: 82,
        placement_ordinal: 19,
    });
    let plan = plan_repair_payment(&nineteen, 20, 20, funds(Type2EventTuple::V1, false, 0))
        .expect("19 inputs plus repaired item");
    assert_eq!(plan.coins.lines.len(), 19);
    assert_eq!(
        plan.coins,
        plan_fee_within(10_000, &nineteen, 20, 20).expect("same native plan")
    );
    let twenty: Vec<_> = (0..20)
        .map(|index| CoinStack {
            coin: Coin::Platinum,
            quantity: 5,
            placement_ordinal: index + 1,
        })
        .collect();
    assert_eq!(
        plan_repair_payment(&twenty, 20, 20, funds(Type2EventTuple::V2, false, 10_000)).err(),
        Some(RepairPlanError::BudgetExceeded)
    );
}

#[test]
fn native_change_and_repaired_item_reach_twenty_two_touched_participants() {
    let mut f = Fixture::new();
    f.entries.truncate(1);
    for index in 0..18 {
        let mut gold = entry(index + 10, u64::from(index) + 100, f.change.gold.clone());
        gold.quantity = 99;
        f.entries.push(gold);
    }
    f.entries.push(entry(40, 200, f.change.crystal.clone()));
    let plan = f.coin_only().expect("19 inputs and two change stacks");
    assert_eq!(plan.payment.coins.lines.len(), 19);
    assert_eq!(plan.payment.coins.change_platinum, 17);
    assert_eq!(plan.payment.coins.change_gold, 82);
    assert_eq!(
        plan.usage,
        RepairUsage {
            touched: 22,
            participants: 22,
            work: 65
        }
    );
    assert_eq!(
        RepairUsage {
            touched: 22,
            participants: 22,
            work: 68
        }
        .check(),
        Ok(())
    );
    for usage in [
        RepairUsage {
            touched: 23,
            participants: 22,
            work: 68,
        },
        RepairUsage {
            touched: 22,
            participants: 23,
            work: 68,
        },
        RepairUsage {
            touched: 22,
            participants: 22,
            work: 69,
        },
    ] {
        assert_eq!(usage.check(), Err(RepairPlanError::BudgetExceeded));
    }
}

#[test]
fn bank_shortfall_requires_v2_nonjunior_and_exact_funds_but_never_changes_coin_priority() {
    let stacks = [CoinStack {
        coin: Coin::Gold,
        quantity: 50,
        placement_ordinal: 1,
    }];
    for funds in [
        funds(Type2EventTuple::V1, false, 10_000),
        funds(Type2EventTuple::V2, true, 10_000),
        funds(Type2EventTuple::V2, false, 9_949),
    ] {
        assert_eq!(
            plan_repair_payment(&stacks, 2, 20, funds).err(),
            Some(RepairPlanError::InsufficientFunds)
        );
    }
    let plan = plan_repair_payment(&stacks, 2, 20, funds(Type2EventTuple::V2, false, 9_950))
        .expect("coins first and exact shortfall");
    assert_eq!(plan.bank_debit, 9_950);
    assert_eq!(plan.coins.lines[0].burned, 50);
    assert_eq!(plan.coins.change, 0);
    let f = Fixture::new();
    let coin_paid = f
        .plan(funds(Type2EventTuple::V2, true, 0))
        .expect("junior coins suffice");
    assert_eq!(coin_paid.payment.bank_debit, 0);
}

#[test]
fn no_coin_input_can_pay_wholly_from_the_bank_only_after_activation_for_nonjunior() {
    let mut f = Fixture::new();
    f.entries.truncate(1);
    let plan = f
        .plan(funds(Type2EventTuple::V2, false, 10_000))
        .expect("native bank policy");
    assert!(plan.payment.coins.lines.is_empty());
    assert!(plan.coin_item_ids.is_empty());
    assert_eq!(plan.payment.bank_debit, 10_000);
    assert_eq!(
        plan.usage,
        RepairUsage {
            touched: 1,
            participants: 2,
            work: 6
        }
    );
    assert_eq!(
        f.coin_only().err(),
        Some(RepairPlanError::InsufficientFunds)
    );
}

#[test]
fn incompatible_nondefault_and_out_of_range_coins_are_not_silently_skipped() {
    for mutation in 0..5 {
        let mut f = Fixture::new();
        match mutation {
            0 => f.entries[1].definition.revision_ref = "incompatible".into(),
            1 => f.entries[1].has_contents = true,
            2 => {
                f.entries[1].timed = Some(StoredTimedState {
                    revision: 1,
                    values: None,
                    deadline_at: None,
                })
            }
            3 => f.entries[1].quantity = 0,
            _ => f.entries[1].quantity = 101,
        }
        assert!(f.plan(funds(Type2EventTuple::V2, false, 10_000)).is_err());
    }
}

#[test]
fn ambiguous_entries_bad_capacity_and_invalid_repair_items_are_refused() {
    for mutation in 0..7 {
        let mut f = Fixture::new();
        match mutation {
            0 => f.entries[1].placement_ordinal = f.entries[0].placement_ordinal,
            1 => f.entries[1].item_instance_id = f.entries[0].item_instance_id,
            2 => f.change.backpack.container_capacity = Some(1),
            3 => f.change.backpack.container_capacity = Some(0),
            4 => f.entries[0].quantity = 2,
            5 => f.entries[0].has_contents = true,
            _ => f.entries[0].placement_ordinal = 0,
        }
        assert_eq!(f.coin_only().err(), Some(RepairPlanError::InvalidSnapshot));
    }
    let mut f = Fixture::new();
    for tag in 10..29 {
        f.entries.push(entry(
            tag,
            u64::from(tag) + 100,
            definition("fixture:other"),
        ));
    }
    assert_eq!(f.coin_only().err(), Some(RepairPlanError::InvalidSnapshot));
    assert_eq!(
        plan_repair_payment(
            &[],
            usize::MAX,
            20,
            funds(Type2EventTuple::V2, false, u64::MAX)
        )
        .err(),
        Some(RepairPlanError::InvalidSnapshot)
    );
}

#[test]
fn only_direct_backpack_entries_are_bounded_not_total_snapshot_records() {
    let mut f = Fixture::new();
    for tag in 10..29 {
        let mut nested = entry(tag, u64::from(tag) + 100, definition("fixture:other"));
        nested.parent = Some(id(7));
        f.entries.push(nested);
    }
    assert!(f.entries.len() > 20);
    assert!(f.coin_only().is_ok());
}

#[test]
fn continuous_exhausted_and_same_definition_targets_are_not_repair_plans() {
    for mutation in 0..3 {
        let mut f = Fixture::new();
        match mutation {
            0 => f.target.lit_continuous = true,
            1 => f.target.full = TimedValues::new(None, Some(0)).expect("native stored bound"),
            _ => f.target.definition = f.from.clone(),
        }
        assert_eq!(f.coin_only().err(), Some(RepairPlanError::InvalidSnapshot));
    }
}

#[test]
fn change_ordinal_overflow_is_refused_but_exact_payment_needs_no_new_entry() {
    let mut f = Fixture::new();
    f.entries[0].placement_ordinal = u64::MAX;
    assert!(f.coin_only().is_ok());
    let mut gold = entry(4, 30, f.change.gold.clone());
    gold.quantity = 50;
    f.entries.push(gold);
    assert_eq!(
        f.coin_only().err(),
        Some(RepairPlanError::CoinPlan(FeePlanError::ChangeDoesNotFit))
    );
}
