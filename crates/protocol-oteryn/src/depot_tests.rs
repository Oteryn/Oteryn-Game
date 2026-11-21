use super::*;

#[test]
fn depot_requests_round_trip_town_zero_and_whole_item_actions() -> Result<(), Error> {
    for action in [
        DepotAction::Browse,
        DepotAction::Deposit(ItemHandle::new(u64::MAX).ok_or(Error::Malformed)?),
        DepotAction::Withdraw(u64::MAX),
    ] {
        let intent = DepotIntent {
            town: Some(0),
            box_number: 17,
            page: MAX_DEPOT_PAGE,
            action,
        };
        assert_eq!(decode_depot_intent(&encode_depot_intent(intent)?)?, intent);
    }
    let intent = DepotIntent {
        town: None,
        box_number: 1,
        page: 0,
        action: DepotAction::Browse,
    };
    assert_eq!(decode_depot_intent(&encode_depot_intent(intent)?)?, intent);
    assert!(
        encode_depot_intent(DepotIntent {
            action: DepotAction::Withdraw(1),
            ..intent
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn depot_replies_round_trip_pages_and_reject_invalid_bounds() -> Result<(), Error> {
    let reply = DepotReply {
        outcome: DepotOutcome::Moved,
        town: Some(u16::MAX),
        box_number: 17,
        page: 0,
        total: 32,
        entries: (1..=32)
            .rev()
            .map(|ordinal| DepotEntry {
                ordinal,
                definition: u32::MAX,
                count: 100,
            })
            .collect(),
    };
    let bytes = encode_depot_reply(&reply)?;
    assert!(bytes.len() <= 1024);
    assert_eq!(decode_depot_reply(&bytes)?, reply);
    let mut invalid = reply.clone();
    invalid.entries.push(DepotEntry {
        ordinal: 33,
        definition: 1,
        count: 1,
    });
    assert!(encode_depot_reply(&invalid).is_err());
    invalid = reply.clone();
    invalid.entries[0].count = 101;
    assert!(encode_depot_reply(&invalid).is_err());
    invalid = reply.clone();
    invalid.entries.swap(0, 1);
    assert!(encode_depot_reply(&invalid).is_err());
    invalid = reply;
    invalid.total = 15_001;
    assert!(encode_depot_reply(&invalid).is_err());
    Ok(())
}

#[test]
fn depot_decoders_reject_truncation_unknown_duplicate_and_oversized_fields() -> Result<(), Error> {
    let bytes = encode_depot_intent(DepotIntent {
        town: Some(1),
        box_number: 1,
        page: 0,
        action: DepotAction::Withdraw(7),
    })?;
    let mut duplicate = bytes.clone();
    duplicate.extend([8, 3]);
    assert!(decode_depot_intent(&duplicate).is_err());
    let mut unknown = bytes;
    unknown.extend([48, 0]);
    assert!(decode_depot_intent(&unknown).is_err());
    for malformed in [
        &[][..],
        &[8, 0],
        &[8, 4],
        &[8, 1, 16, 18],
        &[8, 3, 16, 1, 40, 0],
        &[8, 1, 16, 1, 24, 128],
    ] {
        assert!(decode_depot_intent(malformed).is_err());
    }
    assert!(decode_depot_intent(&[0; 49]).is_err());
    assert!(decode_depot_reply(&[0; 1025]).is_err());
    assert!(decode_depot_reply(&[8, 10, 16, 1]).is_err());
    Ok(())
}

/// Capability 510 (Manual Sort Mode) lets a deposit name the depot box slot it takes.
#[test]
fn a_deposit_slot_is_negotiated_by_capability_510_and_round_trips() -> Result<(), Error> {
    let deposit = DepotIntent {
        town: Some(3),
        box_number: 2,
        page: 1,
        action: DepotAction::Deposit(ItemHandle::new(9).ok_or(Error::Malformed)?),
    };
    let slotted = encode_depot_intent_at(deposit, Some(33))?;
    assert_eq!(decode_depot_intent_at(&slotted, true)?, (deposit, Some(33)));
    assert!(decode_depot_intent_at(&slotted, false).is_err());
    assert!(decode_depot_intent(&slotted).is_err());
    // An unslotted deposit keeps the earlier bytes with or without 510.
    let plain = encode_depot_intent_at(deposit, None)?;
    assert_eq!(plain, encode_depot_intent(deposit)?);
    assert_eq!(decode_depot_intent_at(&plain, true)?, (deposit, None));
    // Only a deposit has a slot; a repeated slot fails closed.
    let withdraw = DepotIntent {
        action: DepotAction::Withdraw(4),
        ..deposit
    };
    assert!(encode_depot_intent_at(withdraw, Some(0)).is_err());
    let mut repeated = slotted.clone();
    repeated.extend([48, 1]);
    assert!(decode_depot_intent_at(&repeated, true).is_err());
    let mut withdraw_slot = encode_depot_intent(withdraw)?;
    withdraw_slot.extend([48, 0]);
    assert!(decode_depot_intent_at(&withdraw_slot, true).is_err());
    let largest = encode_depot_intent_at(
        DepotIntent {
            town: Some(u16::MAX),
            box_number: 17,
            page: MAX_DEPOT_PAGE,
            action: DepotAction::Deposit(ItemHandle::new(u64::MAX).ok_or(Error::Malformed)?),
        },
        Some(u32::MAX),
    )?;
    assert!(largest.len() <= 48);
    Ok(())
}
