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
        let bytes = encode_request(&request).unwrap();
        assert_eq!(decode_request(&bytes).unwrap(), request)
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
