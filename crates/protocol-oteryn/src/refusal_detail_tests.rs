#![allow(clippy::expect_used)]

use super::*;
use crate::item_view::{
    ItemMoveSelection, MAX_ITEM_MOVE_RESULT_BYTES, decode_item_move_result_for,
    encode_item_move_result_for,
};
use crate::world_object::{
    MAX_USE_RESULT_BYTES, decode_use_result_with_gate, encode_use_result_with_gate,
};
use crate::world_spatial::{MAX_STEP_RESULT_BYTES, decode_step_result, encode_step_outcome};

const WITH: &[u32] = &[CAPABILITY_REFUSAL_DETAIL_V1];

/// Canary 47dfd51f `getReturnMessage` (`src/utils/tools.cpp`) and `key_door.lua:32`.
#[test]
fn refusal_detail_reference_messages_match_canary() {
    let expected = [
        (1, "Sorry, not possible."),
        (2, "Too far away."),
        (3, "There is not enough room."),
        (4, "This object is too heavy for you to carry."),
        (5, "You cannot take this object."),
        (6, "First go upstairs."),
        (7, "First go downstairs."),
        (8, "It is locked."),
    ];
    assert_eq!(RefusalDetail::ALL.len(), expected.len());
    for (detail, (wire, text)) in RefusalDetail::ALL.into_iter().zip(expected) {
        assert_eq!(detail as u64, wire);
        assert_eq!(RefusalDetail::from_wire(wire), Some(detail));
        assert_eq!(detail.reference_message(), text);
    }
    assert_eq!(RefusalDetail::from_wire(0), None);
    assert_eq!(RefusalDetail::from_wire(9), None);
}

/// The golden bytes: without capability 66 every result is the exact pre-66 payload; with it
/// field 15 follows.
#[test]
fn refusal_detail_golden_result_bytes() {
    let item = |selected: &[u32], outcome| {
        let mut bytes =
            encode_item_move_result_for(outcome, ItemMoveSelection::default()).expect("encodes");
        append_refusal_detail(&mut bytes, selected, RefusalDetail::of_item_move(outcome));
        bytes
    };
    assert_eq!(item(&[], ItemMoveOutcome::TooFar), [0x08, 0x03]);
    assert_eq!(item(&[61, 12], ItemMoveOutcome::NoRoom), [0x08, 0x05]);
    assert_eq!(
        item(WITH, ItemMoveOutcome::TooFar),
        [0x08, 0x03, 0x78, 0x02]
    );
    assert_eq!(
        item(WITH, ItemMoveOutcome::NoRoom),
        [0x08, 0x05, 0x78, 0x03]
    );
    assert_eq!(item(WITH, ItemMoveOutcome::Moved), [0x08, 0x01]);
    let used = |selected: &[u32], disposition| {
        let mut bytes = encode_use_result_with_gate(disposition, None).expect("encodes");
        append_refusal_detail(&mut bytes, selected, RefusalDetail::of_use(disposition));
        bytes
    };
    assert_eq!(used(&[], UseDisposition::TooFar), [0x08, 0x05]);
    assert_eq!(used(&[11], UseDisposition::Locked), [0x08, 0x0b]);
    assert_eq!(used(WITH, UseDisposition::Locked), [0x08, 0x0b, 0x78, 0x08]);
    assert_eq!(used(WITH, UseDisposition::Committed), [0x08, 0x01]);
    let step = |selected: &[u32], disposition| {
        let mut bytes = encode_step_outcome(selected, disposition, 0);
        append_refusal_detail(&mut bytes, selected, RefusalDetail::of_step(disposition));
        bytes
    };
    assert_eq!(step(&[], StepDisposition::Blocked), [0x08, 0x02]);
    assert_eq!(
        step(WITH, StepDisposition::Blocked),
        [0x08, 0x02, 0x78, 0x01]
    );
    assert_eq!(step(WITH, StepDisposition::Moved), [0x08, 0x01]);
}

/// `REFUSAL-DETAIL-RL-01`: the suffix is two bytes, unique, last and a known code; the rest
/// still decodes with the result's own strict decoder, which refuses field 15.
#[test]
fn refusal_detail_strict_suffix() {
    for detail in RefusalDetail::ALL {
        let mut bytes = vec![0x08, 0x03];
        append_refusal_detail(&mut bytes, WITH, Some(detail));
        assert_eq!(bytes.len(), 2 + MAX_REFUSAL_DETAIL_BYTES);
        assert!(bytes.len() <= MAX_ITEM_MOVE_RESULT_BYTES + MAX_REFUSAL_DETAIL_BYTES);
        let (rest, split) = split_refusal_detail(&bytes).expect("splits");
        assert_eq!(split, Some(detail));
        assert_eq!(
            decode_item_move_result_for(rest, ItemMoveSelection::default()),
            Ok(ItemMoveOutcome::TooFar)
        );
    }
    assert_eq!(
        split_refusal_detail(&[0x08, 0x05]),
        Ok((&[0x08, 0x05][..], None))
    );
    assert_eq!(
        decode_use_result_with_gate(&[0x08, 0x05, 0x78, 0x02]),
        Err(crate::world_object::WorldObjectError::Malformed)
    );
    assert!(decode_step_result(&[0x08, 0x02, 0x78, 0x01]).is_err());
    assert!([0x08, 0x02, 0x78, 0x01].len() <= MAX_STEP_RESULT_BYTES + MAX_REFUSAL_DETAIL_BYTES);
    let largest = [0x08, 0x0d, 0x10, 0x7f, 0x78, 0x08];
    assert_eq!(
        largest.len(),
        MAX_USE_RESULT_BYTES + MAX_REFUSAL_DETAIL_BYTES
    );
    let (rest, detail) = split_refusal_detail(&largest).expect("splits");
    assert_eq!(detail, Some(RefusalDetail::DoorLocked));
    assert!(decode_use_result_with_gate(rest).is_ok());
    for refused in [
        &[0x08, 0x03, 0x78, 0x02, 0x78, 0x02][..],
        &[0x78, 0x02, 0x08, 0x03],
        &[0x08, 0x03, 0x78, 0x00],
        &[0x08, 0x03, 0x78, 0x09],
        &[0x08, 0x03, 0x7a, 0x01, 0x00],
        &[0x08, 0x03, 0x78],
        &[0x08, 0x03, 0x78, 0x82],
    ] {
        assert_eq!(
            split_refusal_detail(refused),
            Err(CyclopediaWireError::Malformed),
            "{refused:02x?}"
        );
    }
}

#[test]
fn refusal_detail_maps_only_refusals() {
    assert_eq!(RefusalDetail::of_item_move(ItemMoveOutcome::Moved), None);
    assert_eq!(
        RefusalDetail::of_item_move(ItemMoveOutcome::NotPickupable),
        Some(RefusalDetail::CannotPickup)
    );
    assert_eq!(
        RefusalDetail::of_item_move(ItemMoveOutcome::Stale),
        Some(RefusalDetail::NotPossible)
    );
    assert_eq!(RefusalDetail::of_use(UseDisposition::Committed), None);
    assert_eq!(RefusalDetail::of_use(UseDisposition::Sealed), None);
    assert_eq!(RefusalDetail::of_step(StepDisposition::TooEarly), None);
    assert_eq!(RefusalDetail::of_step(StepDisposition::Rejected), None);
}

/// Canary `actions.cpp:183`: `playerPos.z > pos.z` (the target is above) is `FIRSTGOUPSTAIRS`.
/// Native floors grow upwards, so an actor below the target has the smaller floor.
#[test]
fn refusal_detail_floor_split_follows_canary_can_use() {
    assert_eq!(
        RefusalDetail::of_floors(-1, 0),
        Some(RefusalDetail::FirstGoUpstairs)
    );
    assert_eq!(
        RefusalDetail::of_floors(0, -1),
        Some(RefusalDetail::FirstGoDownstairs)
    );
    assert_eq!(RefusalDetail::of_floors(-7, -7), None);
}

#[test]
fn refusal_detail_needs_capability_66() {
    let mut bytes = vec![0x08, 0x02];
    append_refusal_detail(
        &mut bytes,
        &[1, 2, 13, 61],
        Some(RefusalDetail::NotPossible),
    );
    assert_eq!(bytes, [0x08, 0x02]);
    append_refusal_detail(&mut bytes, WITH, None);
    assert_eq!(bytes, [0x08, 0x02]);
}
