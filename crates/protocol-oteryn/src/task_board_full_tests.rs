// Assertions name malformed fixtures directly; a fixture encoding failure is a test failure.
#![allow(clippy::expect_used)]
use super::*;

fn reply() -> Reply {
    Reply {
        refusal: 0,
        period: 2965,
        points: 150,
        bounty_points: 20,
        soulseals: 2,
        weekly: Weekly {
            started: true,
            difficulty: 0,
            expanded: false,
            multiplier: 1,
            kill_experience: 1850,
            delivery_experience: 1850,
            pending_points: 25,
            pending_soulseals: 0,
            kills: vec![
                KillTask {
                    race: String::new(),
                    required: 500,
                    current: 51,
                },
                KillTask {
                    race: "oteryn:creature.rat".into(),
                    required: 50,
                    current: 50,
                },
            ],
            deliveries: vec![DeliveryTask {
                item: 3031,
                required: 100,
                current: 0,
                delivered: false,
            }],
        },
        bounty: Bounty {
            state: 2,
            difficulty: 0,
            selected: 1,
            current: 4,
            rerolls: 3,
            daily_reroll_at: 1_800_000_000,
            offers: vec![BountyOffer {
                race: "oteryn:creature.rat".into(),
                required: 25,
                experience: 100,
                points: 3,
                grade: 0,
            }],
            active: Some(BountyOffer {
                race: "oteryn:creature.rat".into(),
                required: 25,
                experience: 100,
                points: 3,
                grade: 0,
            }),
            lists: vec![ListSlot {
                unlocked: true,
                preferred: "oteryn:creature.rat".into(),
                unwanted: String::new(),
            }],
        },
        talisman: [TalismanPath {
            level: 1,
            bonus: 250,
            next_cost: 17,
        }; TALISMAN_PATHS],
        shop: vec![ShopOffer {
            index: 0,
            kind: 2,
            price: 100_000,
            primary: 1907,
            secondary: 0,
            addon: 0,
            bought: false,
        }],
    }
}

#[test]
fn full_task_board_request_roundtrips_every_action_and_rejects_ambiguous_fields() {
    for action in 4..=18 {
        let request = Request {
            action,
            selection: 0,
            race: if action == 13 {
                "oteryn:creature.rat".into()
            } else {
                String::new()
            },
        };
        let encoded = encode_request(&request).expect("every full action has a wire request");
        assert_eq!(decode_request(&encoded), Ok(request));
        for end in 0..encoded.len() {
            assert!(decode_request(&encoded[..end]).is_err());
        }
        let mut duplicate = encoded.clone();
        duplicate.extend_from_slice(&[8, action]);
        assert!(decode_request(&duplicate).is_err());
        let mut unknown = encoded;
        unknown.extend_from_slice(&[32, 0]);
        assert!(decode_request(&unknown).is_err());
    }
    for (action, selection) in [
        (3, 0),
        (19, 0),
        (4, 1),
        (5, 4),
        (6, 9),
        (8, 3),
        (13, 5),
        (15, 1),
        (16, 32),
        (18, 4),
    ] {
        assert!(
            encode_request(&Request {
                action,
                selection,
                race: String::new()
            })
            .is_err()
        );
    }
    assert!(
        encode_request(&Request {
            action: 13,
            selection: 0,
            race: "rat".into()
        })
        .is_err()
    );
    assert!(
        encode_request(&Request {
            action: 4,
            selection: 0,
            race: "oteryn:creature.rat".into()
        })
        .is_err()
    );
    assert!(decode_request(&[8, 4, 16, 0, 26, 0, 26, 0]).is_err());
    assert!(decode_request(&[16, 0, 8, 4, 26, 0]).is_err());
    assert!(decode_request(&[8, 0x84, 0, 16, 0, 26, 0]).is_err());
    assert_eq!(
        decode_request(&[0; MAX_REQUEST_BYTES + 1]),
        Err(Error::LimitExceeded)
    );
    // Full actions deliberately cannot be mistaken for a TASKBOARD-1 request.
    assert!(
        super::super::decode_task_board_request(&encode_request(&Request::query()).expect("query"))
            .is_err()
    );
}

#[test]
fn full_task_board_reply_preserves_progress_rewards_lists_shop_and_talisman() {
    let expected = reply();
    let encoded = encode_reply(&expected).expect("full reply");
    assert_eq!(decode_reply(&encoded), Ok(expected));
    let mut duplicate = encoded.clone();
    duplicate.extend_from_slice(&[8, 0]);
    assert!(decode_reply(&duplicate).is_err());
    let mut unknown = encoded;
    unknown.extend_from_slice(&[80, 0]);
    assert!(decode_reply(&unknown).is_err());
    let mut invalid = reply();
    invalid.weekly.kills[0].current = 501;
    assert!(encode_reply(&invalid).is_err());
    invalid = reply();
    invalid.weekly.deliveries[0].delivered = true;
    assert!(encode_reply(&invalid).is_err());
    invalid = reply();
    invalid.bounty.lists[0].unlocked = false;
    assert!(encode_reply(&invalid).is_err());
    invalid = reply();
    invalid.bounty.current = 26;
    assert!(encode_reply(&invalid).is_err());
    assert_eq!(
        decode_reply(&[0; MAX_REPLY_BYTES + 1]),
        Err(Error::LimitExceeded)
    );
}

#[test]
fn full_task_board_expanded_board_is_bounded_without_truncation() {
    let mut board = reply();
    let longest = format!("oteryn:{}", "a".repeat(128 - 7));
    board.weekly.expanded = true;
    board.weekly.kills = vec![
        KillTask {
            race: longest.clone(),
            required: u32::MAX,
            current: u32::MAX
        };
        MAX_KILL_TASKS
    ];
    board.weekly.deliveries = vec![
        DeliveryTask {
            item: u16::MAX,
            required: u32::MAX,
            current: u32::MAX,
            delivered: true
        };
        MAX_DELIVERY_TASKS
    ];
    board.bounty.offers = vec![
        BountyOffer {
            race: longest.clone(),
            required: u32::MAX,
            experience: u64::MAX,
            points: u32::MAX,
            grade: 2
        };
        MAX_BOUNTY_OFFERS
    ];
    board.bounty.lists = vec![
        ListSlot {
            unlocked: true,
            preferred: longest.clone(),
            unwanted: longest
        };
        MAX_LIST_SLOTS
    ];
    board.shop = (0..MAX_SHOP_OFFERS)
        .map(|index| ShopOffer {
            index: index as u16,
            kind: 0,
            price: u32::MAX,
            primary: u32::MAX,
            secondary: u32::MAX,
            addon: 0,
            bought: false,
        })
        .collect();
    let encoded = encode_reply(&board).expect("maximum legal board");
    assert!(encoded.len() < MAX_REPLY_BYTES);
    assert_eq!(decode_reply(&encoded), Ok(board.clone()));
    board.weekly.kills.push(board.weekly.kills[0].clone());
    assert_eq!(encode_reply(&board), Err(Error::LimitExceeded));
    board.weekly.kills.pop();
    board
        .weekly
        .deliveries
        .push(board.weekly.deliveries[0].clone());
    assert_eq!(encode_reply(&board), Err(Error::LimitExceeded));
}

#[test]
fn bounty_token_balance_is_not_limited_by_the_daily_claim_limit() {
    // Owner decision #2764/6542842489: completion awards keep the existing wide
    // resource balance; ten is only the daily-claim refusal threshold.
    for balance in [10, 11, 255, 256, i64::MAX as u64] {
        let mut value = reply();
        value.bounty.rerolls = balance;
        let bytes = encode_reply(&value).expect("wide token balance encodes");
        assert_eq!(decode_reply(&bytes), Ok(value));
    }
}

// Mutate one canonical field in a valid encoded fixture so decoder rejection is
// checked independently of the public encoder's refusal to construct it.
fn replace_encoded_field(bytes: &[u8], field: u64, replacement: &[u8]) -> Option<Vec<u8>> {
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        let tag = read_varint(bytes, &mut at).ok()?;
        match tag & 7 {
            0 => {
                read_varint(bytes, &mut at).ok()?;
            }
            2 => {
                read_bytes(bytes, &mut at).ok()?;
            }
            _ => return None,
        }
        if tag >> 3 == field {
            let mut changed = bytes[..start].to_vec();
            changed.extend_from_slice(replacement);
            changed.extend_from_slice(&bytes[at..]);
            return Some(changed);
        }
    }
    None
}

#[test]
fn full_task_board_reply_closes_refusals_and_owner_balance_ranges() {
    let mut empty_balances = reply();
    empty_balances.points = 0;
    empty_balances.bounty_points = 0;
    empty_balances.soulseals = 0;
    empty_balances.bounty.rerolls = 0;
    let bytes = encode_reply(&empty_balances)
        .expect("zero balances are present values, not missing fields");
    assert_eq!(decode_reply(&bytes), Ok(empty_balances));

    let mut boundary = reply();
    boundary.points = i64::MAX.unsigned_abs();
    boundary.bounty_points = u64::from(u32::MAX);
    boundary.soulseals = u64::from(u32::MAX);
    boundary.bounty.rerolls = i64::MAX.unsigned_abs();
    for refusal in 0..=17 {
        boundary.refusal = refusal;
        let bytes =
            encode_reply(&boundary).expect("all closed refusals and exact owner maxima encode");
        assert_eq!(decode_reply(&bytes), Ok(boundary.clone()));
    }
    let valid_bytes = encode_reply(&boundary).expect("boundary fixture");
    for (field, value) in [
        (1, 18),
        (1, 255),
        (3, i64::MAX.unsigned_abs() + 1),
        (4, u64::from(u32::MAX) + 1),
        (5, u64::from(u32::MAX) + 1),
    ] {
        let mut invalid = boundary.clone();
        match field {
            1 => {
                invalid.refusal = u8::try_from(value).expect("refusal fixture fits its wire field")
            }
            3 => invalid.points = value,
            4 => invalid.bounty_points = value,
            _ => invalid.soulseals = value,
        }
        assert_eq!(
            encode_reply(&invalid),
            Err(Error::Malformed),
            "field {field}: {value}"
        );
        let mut replacement = Vec::new();
        push_varint_field(&mut replacement, field, value);
        let malformed = replace_encoded_field(&valid_bytes, field, &replacement)
            .expect("existing scalar field");
        assert_eq!(
            decode_reply(&malformed),
            Err(Error::Malformed),
            "decoded field {field}: {value}"
        );
    }

    let mut invalid = boundary.clone();
    invalid.bounty.rerolls = i64::MAX.unsigned_abs() + 1;
    assert_eq!(encode_reply(&invalid), Err(Error::Malformed));
    let valid_bounty = encode_bounty(&boundary.bounty).expect("valid nested bounty fixture");
    let mut replacement = Vec::new();
    push_varint_field(&mut replacement, 5, invalid.bounty.rerolls);
    let malformed_bounty =
        replace_encoded_field(&valid_bounty, 5, &replacement).expect("existing reroll field");
    let mut nested = Vec::new();
    push_message_field(&mut nested, 7, &malformed_bounty);
    let malformed =
        replace_encoded_field(&valid_bytes, 7, &nested).expect("existing bounty message");
    assert_eq!(decode_reply(&malformed), Err(Error::Malformed));
}

#[test]
fn full_task_board_delivery_availability_can_exceed_the_requirement() {
    let mut value = reply();
    let required = value.weekly.deliveries[0].required;
    for available in [0, required - 1, required, required + 1, u32::MAX] {
        value.weekly.deliveries[0].current = available;
        let bytes =
            encode_reply(&value).expect("undelivered availability is not capped by required");
        assert_eq!(decode_reply(&bytes), Ok(value.clone()));
    }
    value.weekly.deliveries[0].delivered = true;
    value.weekly.deliveries[0].current = required;
    let bytes = encode_reply(&value).expect("completed delivery reports its required quantity");
    assert_eq!(decode_reply(&bytes), Ok(value.clone()));
    for invalid in [required - 1, required + 1, u32::MAX] {
        value.weekly.deliveries[0].current = invalid;
        assert_eq!(encode_reply(&value), Err(Error::Malformed));
    }
}
