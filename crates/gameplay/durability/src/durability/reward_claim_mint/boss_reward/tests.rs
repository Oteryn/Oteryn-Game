use super::delivery::{CapacityLine, capacity_prefix};
use super::*;

fn line(weight: u32, quantity: u32) -> CapacityLine {
    CapacityLine {
        quantity,
        maximum: 100,
        weight: Some(weight),
        container: false,
    }
}

#[test]
fn reward_collection_respects_weight_slots_and_sequential_partial_delivery() {
    let lines = [line(10, 2), line(100, 1), line(1, 1)];
    assert_eq!(capacity_prefix(&lines, 3, 0, 25), Ok(1));
    assert_eq!(
        capacity_prefix(&lines, 0, 0, 1_000),
        Err(BossRewardCollectionStatus::NoRoom)
    );
    assert_eq!(capacity_prefix(&lines, 1, 0, 1_000), Ok(1));
    assert_eq!(
        capacity_prefix(&lines, 3, 100, 119),
        Err(BossRewardCollectionStatus::NoRoom)
    );
    assert_eq!(
        capacity_prefix(&[line(1, 1)], 1, u64::MAX, u64::MAX - 1),
        Err(BossRewardCollectionStatus::NoRoom)
    );
}

#[test]
fn reward_collection_rejects_unqualified_items_but_keeps_an_admitted_prefix() {
    let bad = CapacityLine {
        maximum: 0,
        ..line(1, 1)
    };
    assert_eq!(
        capacity_prefix(&[bad], 1, 0, 100),
        Err(BossRewardCollectionStatus::ContentUnavailable)
    );
    assert_eq!(capacity_prefix(&[line(1, 1), bad], 2, 0, 100), Ok(1));
    assert_eq!(
        capacity_prefix(
            &[CapacityLine {
                weight: None,
                ..line(1, 1)
            }],
            1,
            0,
            100
        ),
        Err(BossRewardCollectionStatus::ContentUnavailable)
    );
    assert_eq!(
        capacity_prefix(
            &[CapacityLine {
                container: true,
                ..line(1, 2)
            }],
            1,
            0,
            100
        ),
        Err(BossRewardCollectionStatus::ContentUnavailable)
    );
    assert_eq!(
        capacity_prefix(
            &[CapacityLine {
                container: true,
                maximum: 1,
                ..line(1, 1)
            }],
            1,
            0,
            100
        ),
        Ok(1)
    );
    assert_eq!(capacity_prefix(&vec![line(1, 1); 21], 100, 0, 100), Ok(20));
}

#[test]
fn reward_selection_uses_only_retained_personal_entitlement_coordinates() {
    assert!(valid_selection(BossRewardSelection::All));
    assert!(valid_selection(BossRewardSelection::Group {
        death: [1; 16]
    }));
    for ordinal in [0, 1_001, u16::MAX] {
        assert!(!valid_selection(BossRewardSelection::Item {
            death: [1; 16],
            ordinal
        }));
    }
    assert!(!valid_selection(BossRewardSelection::Group {
        death: [0; 16]
    }));
    assert!(!valid_selection(BossRewardSelection::Item {
        death: [0; 16],
        ordinal: 1
    }));
    assert!(valid_selection(BossRewardSelection::Item {
        death: [1; 16],
        ordinal: 1_000
    }));
}
