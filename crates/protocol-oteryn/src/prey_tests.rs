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
    };
    assert!(
        encode_reply(&Reply {
            status: 0,
            wildcard_balance: 52,
            slots: vec![slot.clone()]
        })
        .is_ok()
    );
    assert!(
        encode_reply(&Reply {
            status: 0,
            wildcard_balance: 53,
            slots: vec![slot]
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
        for status in 0..=8 {
            let reply = Reply {
                status,
                wildcard_balance: 52,
                slots: slots.clone(),
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
    assert_eq!(bytes.len(), MAX_REQUEST_BYTES);
    assert_eq!(decode_request(&bytes).expect("valid PREY fixture"), request);
}

#[test]
fn reply_rejects_duplicate_unknown_and_noncanonical_fields() {
    let slot = encode_slot(&sample_slot(1)).expect("valid PREY fixture");
    for tag in [8, 16, 40, 48, 56, 64, 72, 80] {
        let mut duplicate = slot.clone();
        duplicate.extend([tag, 0]);
        let mut bytes = vec![8, 0, 16, 52];
        push_message_field(&mut bytes, 3, &duplicate);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    for extra in [vec![34, 1, b'x'], vec![88, 0], vec![10, 0]] {
        let mut body = slot.clone();
        body.extend(extra);
        let mut bytes = vec![8, 0, 16, 52];
        push_message_field(&mut bytes, 3, &body);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    for bytes in [
        vec![8, 0, 8, 0, 16, 0],
        vec![8, 0, 16, 0, 16, 0],
        vec![8, 0, 16, 0, 32, 0],
        vec![16, 0, 8, 0],
        vec![8, 128, 0, 16, 0],
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
    assert_eq!(decode_reply(&bytes), Err(Error::LimitExceeded));
    let mut slot = sample_slot(1);
    slot.free_reroll_at = i64::MAX as u64 + 1;
    assert_eq!(encode_slot(&slot), Err(Error::Malformed));
    let mut bytes = raw_reply(&[sample_slot(1)]);
    bytes.pop(); // truncate the timestamp varint
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
        ] {
            field(
                &mut body,
                number,
                if number << 3 == tag { value } else { original },
            );
        }
        let mut bytes = vec![8, 0, 16, 52];
        push_message_field(&mut bytes, 3, &body);
        assert_eq!(decode_reply(&bytes), Err(Error::Malformed));
    }
    assert_eq!(decode_reply(&[8, 9, 16, 0]), Err(Error::Malformed));
    assert_eq!(decode_reply(&[8, 0, 16, 53]), Err(Error::Malformed));
}
