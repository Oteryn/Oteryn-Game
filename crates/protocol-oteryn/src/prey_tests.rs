#![allow(clippy::expect_used)]

use super::*;
#[test]
fn every_intent_round_trips_canonically() {
    let requests = [
        Request::Query,
        Request::ListReroll { slot: 1 },
        Request::Select { slot: 2, index: 9 },
        Request::SelectAny {
            slot: 3,
            creature: "oteryn:creature.rat".into(),
        },
        Request::BonusReroll { slot: 1 },
        Request::SetOption { slot: 2, option: 2 },
        Request::SetTier { slot: 2, tier: 5 },
        Request::OfferedToday {
            slot: 1,
            after: String::new(),
        },
        Request::OfferedToday {
            slot: 1,
            after: "oteryn:creature.rat".into(),
        },
        Request::PickAgain {
            slot: 3,
            creature: "oteryn:creature.dragon".into(),
            expected_price_gold: 40000,
        },
        Request::Select { slot: 2, index: 12 },
    ];
    for request in requests {
        let bytes = encode_request(&request).expect("valid PREY fixture");
        assert_eq!(decode_request(&bytes).expect("valid PREY fixture"), request)
    }
}
#[test]
fn request_rejects_invalid_slots_and_noncanonical_input() {
    assert!(encode_request(&Request::ListReroll { slot: 0 }).is_err());
    assert!(decode_request(&[8, 0, 16, 0]).is_err());
    assert!(
        encode_request(&Request::SelectAny {
            slot: 1,
            creature: "rat".into()
        })
        .is_err()
    )
}
#[test]
fn reply_enforces_resource_and_slot_bounds() {
    let slot = Slot {
        slot: 1,
        state: 2,
        creatures: vec!["oteryn:creature.rat".into()],
        selected: "oteryn:creature.rat".into(),
        bonus: 3,
        grade: 10,
        percent: 40,
        hunting_seconds: 7200,
        option: 0,
        free_reroll_at: 72_000,
        chosen_tier: 1,
        allowed_tier: 2,
        selection_epoch: 1,
    };
    assert!(
        encode_reply(&Reply {
            status: 0,
            wildcard_balance: 52,
            slots: vec![slot.clone()],
            list_size: 9,
            expansion_owned: false,
            offered_today: Vec::new(),
            offered_next: String::new(),
        })
        .is_ok()
    );
    assert!(
        encode_reply(&Reply {
            status: 0,
            wildcard_balance: 53,
            slots: vec![slot],
            list_size: 9,
            expansion_owned: false,
            offered_today: Vec::new(),
            offered_next: String::new(),
        })
        .is_err()
    )
}

fn sample_slot(slot: u8) -> Slot {
    Slot {
        slot,
        state: 2,
        creatures: vec!["oteryn:creature.rat".into()],
        selected: "oteryn:creature.rat".into(),
        bonus: 4,
        grade: 10,
        percent: 40,
        hunting_seconds: 7200,
        option: 2,
        free_reroll_at: i64::MAX as u64,
        chosen_tier: 5,
        allowed_tier: 4, // PREY-DIFF: retain the chosen tier after level loss.
        selection_epoch: i64::MAX as u64,
    }
}

fn raw_reply(slots: &[Slot]) -> Vec<u8> {
    let mut bytes = vec![8, 0, 16, 52];
    for slot in slots {
        push_message_field(
            &mut bytes,
            3,
            &encode_slot(slot).expect("valid PREY fixture"),
        );
    }
    bytes.extend([32, 9, 40, 0]);
    bytes
}

#[test]
fn replies_round_trip_including_maximum_projection() {
    let key = format!("oteryn:{}", "x".repeat(249));
    let mut slots: Vec<_> = (1..=3).map(sample_slot).collect();
    for slot in &mut slots {
        slot.creatures = vec![key.clone(); MAX_CREATURES];
        slot.selected = key.clone();
    }
    for slots in [vec![], slots] {
        for status in 0..=STATUS_ALREADY_OWNED {
            let reply = Reply {
                status,
                wildcard_balance: 52,
                slots: slots.clone(),
                list_size: MAX_CREATURES as u8,
                expansion_owned: true,
                offered_today: Vec::new(),
                offered_next: String::new(),
            };
            let bytes = encode_reply(&reply).expect("valid PREY fixture");
            assert_eq!(decode_reply(&bytes).expect("valid PREY fixture"), reply);
        }
    }
    let request = Request::SelectAny {
        slot: 3,
        creature: key,
    };
    let bytes = encode_request(&request).expect("valid PREY fixture");
    assert_eq!(bytes.len(), 263); // Original SelectAny bound remains unchanged.
    let pick = Request::PickAgain {
        slot: 3,
        creature: format!("oteryn:{}", "x".repeat(249)),
        expected_price_gold: i64::MAX as u64,
    };
    let pick_bytes = encode_request(&pick).expect("maximum confirmed price");
    assert_eq!(pick_bytes.len(), MAX_REQUEST_BYTES);
    assert_eq!(decode_request(&pick_bytes).expect("maximum pick"), pick);
    assert_eq!(decode_request(&bytes).expect("valid PREY fixture"), request);
}

#[test]
fn reply_rejects_duplicate_unknown_and_noncanonical_fields() {
    let slot = encode_slot(&sample_slot(1)).expect("valid PREY fixture");
    for tag in [8, 16, 40, 48, 56, 64, 72, 80, 88, 96, 104] {
        let mut duplicate = slot.clone();
        duplicate.extend([tag, 0]);
        let mut bytes = vec![8, 0, 16, 52];
        push_message_field(&mut bytes, 3, &duplicate);
        bytes.extend([32, 9, 40, 0]);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    for extra in [vec![34, 1, b'x'], vec![112, 0], vec![10, 0]] {
        let mut body = slot.clone();
        body.extend(extra);
        let mut bytes = vec![8, 0, 16, 52];
        push_message_field(&mut bytes, 3, &body);
        bytes.extend([32, 9, 40, 0]);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    for bytes in [
        vec![8, 0, 8, 0, 16, 0, 32, 9, 40, 0],
        vec![8, 0, 16, 0, 16, 0, 32, 9, 40, 0],
        vec![8, 0, 16, 0, 32, 9, 40, 0, 64, 0],
        vec![16, 0, 8, 0, 32, 9, 40, 0],
        vec![8, 128, 0, 16, 0, 32, 9, 40, 0],
        vec![8, 0],
        vec![],
    ] {
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    // Singular selected repeated, scalar order changed, and invalid UTF-8.
    for body in [
        {
            let mut v = slot.clone();
            push_message_field(&mut v, 4, b"oteryn:rat");
            v
        },
        {
            let mut v = slot.clone();
            v.swap(0, 2);
            v
        },
        {
            let mut v = slot.clone();
            push_message_field(&mut v, 3, &[255]);
            v
        },
    ] {
        let mut bytes = vec![8, 0, 16, 52];
        push_message_field(&mut bytes, 3, &body);
        bytes.extend([32, 9, 40, 0]);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
}

#[test]
fn reply_rejects_oversize_counts_order_and_timestamp_overflow() {
    assert_eq!(
        decode_reply(&vec![0; MAX_REPLY_BYTES + 1]),
        Err(Error::LimitExceeded)
    );
    for slots in [
        vec![sample_slot(2), sample_slot(1)],
        vec![sample_slot(1), sample_slot(1)],
    ] {
        assert_eq!(decode_reply(&raw_reply(&slots)), Err(Error::Malformed));
    }
    assert_eq!(
        decode_reply(&raw_reply(
            &(1..=4).map(|_| sample_slot(1)).collect::<Vec<_>>()
        )),
        Err(Error::LimitExceeded)
    );
    let mut body = encode_slot(&sample_slot(1)).expect("valid PREY fixture");
    for _ in 0..MAX_CREATURES {
        push_message_field(&mut body, 3, b"oteryn:rat");
    }
    let mut bytes = vec![8, 0, 16, 0];
    push_message_field(&mut bytes, 3, &body);
    bytes.extend([32, 9, 40, 0]);
    assert_eq!(decode_reply(&bytes), Err(Error::LimitExceeded));
    let mut slot = sample_slot(1);
    slot.free_reroll_at = i64::MAX as u64 + 1;
    assert_eq!(encode_slot(&slot), Err(Error::Malformed));
    let mut bytes = raw_reply(&[sample_slot(1)]);
    bytes.pop(); // truncate the required expansion flag varint
    assert!(decode_reply(&bytes).is_err());
}

#[test]
fn reply_rejects_every_scalar_above_its_bound() {
    for (tag, value) in [
        (8, 4),
        (16, 3),
        (40, 5),
        (48, 11),
        (56, 41),
        (64, 7201),
        (72, 3),
        (80, i64::MAX as u64 + 1),
        (88, 6),
        (96, 6),
        (104, i64::MAX as u64 + 1),
    ] {
        let mut body = Vec::new();
        for (number, original) in [
            (1, 1),
            (2, 2),
            (5, 4),
            (6, 10),
            (7, 40),
            (8, 7200),
            (9, 2),
            (10, 0),
            (11, 1),
            (12, 1),
            (13, 1),
        ] {
            field(
                &mut body,
                number,
                if number << 3 == tag { value } else { original },
            );
        }
        let mut bytes = vec![8, 0, 16, 52];
        push_message_field(&mut bytes, 3, &body);
        bytes.extend([32, 9, 40, 0]);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    assert_eq!(
        decode_reply(&[8, 14, 16, 0, 32, 9, 40, 0]),
        Err(Error::Malformed)
    );
    assert_eq!(
        decode_reply(&[8, 0, 16, 53, 32, 9, 40, 0]),
        Err(Error::Malformed)
    );
}

#[test]
fn difficulty_and_pick_again_requests_reject_invalid_and_internal_actions() {
    for tier in [0, 6, u8::MAX] {
        assert_eq!(
            encode_request(&Request::SetTier { slot: 1, tier }),
            Err(Error::Malformed)
        );
        assert!(decode_request(&[8, 6, 16, 1, 24, tier]).is_err());
    }
    for slot in [0, 4, u8::MAX] {
        assert!(encode_request(&Request::SetTier { slot, tier: 1 }).is_err());
        assert!(
            encode_request(&Request::OfferedToday {
                slot,
                after: String::new()
            })
            .is_err()
        );
        assert!(
            encode_request(&Request::PickAgain {
                slot,
                creature: "oteryn:creature.rat".into(),
                expected_price_gold: 400
            })
            .is_err()
        );
    }
    assert!(encode_request(&Request::Select { slot: 1, index: 65 }).is_err());
    // Store entitlement is never accepted from the client, even with a valid slot.
    assert_eq!(decode_request(&[8, 9, 16, 1]), Err(Error::Malformed));
    assert!(decode_request(&[8, 7, 16, 1, 34, 0]).is_err());
    assert!(decode_request(&[8, 6, 16, 1, 24, 1, 24, 2]).is_err());
}

#[test]
fn offered_page_prices_and_expansion_round_trip_and_enforce_bounds() {
    let mut reply = Reply {
        status: 0,
        wildcard_balance: 52,
        slots: vec![sample_slot(1)],
        list_size: 12,
        expansion_owned: true,
        offered_today: (0..MAX_OFFERED_PAGE)
            .map(|index| OfferedCreature {
                creature: format!("oteryn:creature.example_{index:02}"),
                price_gold: i64::MAX as u64,
                occupied_slot: (index % 4) as u8,
            })
            .collect(),
        offered_next: "oteryn:creature.example_15".into(),
    };
    let bytes = encode_reply(&reply).expect("maximum offered page");
    assert_eq!(decode_reply(&bytes).expect("page round-trip"), reply);
    for size in [0, 65, u8::MAX] {
        reply.list_size = size;
        assert!(encode_reply(&reply).is_err());
    }
    reply.list_size = 12;
    reply.offered_today[0].price_gold = i64::MAX as u64 + 1;
    assert!(encode_reply(&reply).is_err());
    reply.offered_today[0].price_gold = 1;
    reply.offered_today[0].occupied_slot = 4;
    assert!(encode_reply(&reply).is_err());
    reply.offered_today[0].occupied_slot = 0;
    reply.offered_today.swap(0, 1);
    assert!(encode_reply(&reply).is_err());
    reply.offered_today.swap(0, 1);
    reply.offered_today.push(OfferedCreature {
        creature: "oteryn:creature.zzz".into(),
        price_gold: 1,
        occupied_slot: 0,
    });
    assert!(encode_reply(&reply).is_err());
}

#[test]
fn offered_page_rejects_duplicate_unknown_noncanonical_and_oversize_rows() {
    let row = |extra: &[u8]| {
        let mut row = Vec::new();
        push_message_field(&mut row, 1, b"oteryn:creature.rat");
        field(&mut row, 2, 100);
        field(&mut row, 3, 0);
        row.extend_from_slice(extra);
        row
    };
    for extra in [&[16, 1][..], &[24, 1], &[32, 0], &[10, 0]] {
        let mut bytes = vec![8, 0, 16, 0, 32, 12, 40, 1];
        push_message_field(&mut bytes, 6, &row(extra));
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    let mut bytes = vec![8, 0, 16, 0, 32, 12, 40, 1];
    for _ in 0..=MAX_OFFERED_PAGE {
        push_message_field(&mut bytes, 6, &row(&[]));
    }
    assert_eq!(decode_reply(&bytes), Err(Error::LimitExceeded));
    assert!(decode_reply(&[8, 0, 16, 0, 32, 9, 40, 2]).is_err());
    assert!(decode_reply(&[8, 0, 16, 0, 32, 9, 32, 12, 40, 1]).is_err());
}

#[test]
fn historical_receipts_have_an_explicit_strict_upgrade_reader() {
    let slot = sample_slot(1);
    let mut legacy_slot = Vec::new();
    field(&mut legacy_slot, 1, 1);
    field(&mut legacy_slot, 2, 2);
    push_message_field(&mut legacy_slot, 3, b"oteryn:creature.rat");
    push_message_field(&mut legacy_slot, 4, b"oteryn:creature.rat");
    for (number, value) in [
        (5, 4),
        (6, 10),
        (7, 40),
        (8, 7200),
        (9, 2),
        (10, i64::MAX as u64),
    ] {
        field(&mut legacy_slot, number, value);
    }
    let mut bytes = vec![8, 0, 16, 52];
    push_message_field(&mut bytes, 3, &legacy_slot);
    assert!(decode_reply(&bytes).is_err());
    let upgraded = decode_legacy_reply(&bytes).expect("0337 receipt");
    assert_eq!(upgraded.slots[0].selected, slot.selected);
    assert_eq!(upgraded.slots[0].hunting_seconds, slot.hunting_seconds);
    assert_eq!(upgraded.slots[0].selection_epoch, 0);
    assert_eq!(upgraded.list_size, 9);
    assert_eq!(
        decode_reply(&encode_reply(&upgraded).expect("upgrade")),
        Ok(upgraded)
    );
    bytes.extend([32, 9]);
    assert!(decode_legacy_reply(&bytes).is_err());
}
