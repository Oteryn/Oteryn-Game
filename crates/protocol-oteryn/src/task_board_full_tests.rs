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
        (15, 5),
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
