use super::*;
use crate::charm_wire::push_varint;
use crate::npc_service::MAX_OFFERS;

fn intent(action: NpcShopAction) -> NpcShopIntent {
    NpcShopIntent {
        npc_actor: EntityRef {
            identity: [7; 16],
            generation: u64::MAX,
        },
        catalogue_revision: u64::MAX,
        offer_index: u32::try_from(MAX_OFFERS - 1).expect("bound"),
        side: NpcTradeSide::Sell,
        action,
        expected_unit_price: u32::MAX,
    }
}

#[test]
fn npc_shop_numbers_are_the_reserved_ones() {
    assert_eq!(CAPABILITY_NPC_SHOP_V1, 81);
    assert_eq!(COMMAND_TYPE_NPC_SHOP, 97);
}

#[test]
fn npc_shop_intent_round_trips_at_its_bound() {
    for action in [NpcShopAction::Look, NpcShopAction::SellAll] {
        let request = intent(action);
        let bytes = encode_npc_shop_intent(&request).expect("encode");
        assert_eq!(bytes.len(), MAX_NPC_SHOP_INTENT_BYTES);
        assert_eq!(decode_npc_shop_intent(&bytes).expect("decode"), request);
    }
}

#[test]
fn npc_shop_intent_refuses_unknown_repeated_and_out_of_range_fields() {
    let small = NpcShopIntent {
        catalogue_revision: 1,
        ..intent(NpcShopAction::Look)
    };
    let valid = encode_npc_shop_intent(&small).expect("encode");
    let with = |extra: &[u8]| [valid.as_slice(), extra].concat();
    // An unknown field, a repeated action, an unknown action and a zero action.
    for extra in [vec![7 << 3, 1], vec![5 << 3, 1]] {
        assert_eq!(
            decode_npc_shop_intent(&with(&extra)),
            Err(NpcShopWireError::Malformed)
        );
    }
    let mut no_action = intent(NpcShopAction::Look);
    no_action.side = NpcTradeSide::Buy;
    let mut bytes = encode_npc_shop_intent(&no_action).expect("encode");
    let at = bytes
        .windows(2)
        .position(|pair| pair == [5 << 3, 1])
        .expect("action");
    for wrong in [0, 3] {
        bytes[at + 1] = wrong;
        assert_eq!(
            decode_npc_shop_intent(&bytes),
            Err(NpcShopWireError::Malformed)
        );
    }
    bytes.remove(at);
    bytes.remove(at);
    assert_eq!(
        decode_npc_shop_intent(&bytes),
        Err(NpcShopWireError::Malformed)
    );
    let mut index = intent(NpcShopAction::Look);
    index.offer_index = u32::try_from(MAX_OFFERS).expect("bound");
    assert_eq!(
        encode_npc_shop_intent(&index),
        Err(NpcShopWireError::LimitExceeded)
    );
    let mut generation = intent(NpcShopAction::Look);
    generation.npc_actor.generation = 0;
    assert_eq!(
        encode_npc_shop_intent(&generation),
        Err(NpcShopWireError::Malformed)
    );
    let mut oversized = valid.clone();
    oversized.resize(MAX_NPC_SHOP_INTENT_BYTES + 1, 0);
    assert_eq!(
        decode_npc_shop_intent(&oversized),
        Err(NpcShopWireError::LimitExceeded)
    );
}

#[test]
fn npc_shop_result_carries_text_or_sale_only_with_ok() {
    let look = NpcShopResult {
        disposition: NpcIntentDisposition::Ok,
        look_text: "x".repeat(MAX_ITEM_LOOK_TEXT_BYTES),
        sold_units: 0,
        total_gold_units: 0,
    };
    let sold = NpcShopResult {
        disposition: NpcIntentDisposition::Ok,
        look_text: String::new(),
        sold_units: MAX_SELL_ALL_UNITS,
        total_gold_units: MAX_SELL_ALL_GOLD_UNITS,
    };
    for result in [look.clone(), sold.clone()] {
        let bytes = encode_npc_shop_result(&result).expect("encode");
        assert!(bytes.len() <= MAX_NPC_SHOP_RESULT_BYTES);
        assert_eq!(decode_npc_shop_result(&bytes).expect("decode"), result);
    }
    for disposition in NpcIntentDisposition::ALL {
        if disposition == NpcIntentDisposition::Ok {
            continue;
        }
        let refused = NpcShopResult::refused(disposition);
        let bytes = encode_npc_shop_result(&refused).expect("encode");
        assert_eq!(bytes.len(), 2);
        assert_eq!(decode_npc_shop_result(&bytes).expect("decode"), refused);
        for wrong in [
            NpcShopResult {
                disposition,
                ..look.clone()
            },
            NpcShopResult {
                disposition,
                ..sold.clone()
            },
        ] {
            assert_eq!(
                encode_npc_shop_result(&wrong),
                Err(NpcShopWireError::Malformed)
            );
        }
    }
    for wrong in [
        NpcShopResult::refused(NpcIntentDisposition::Ok),
        NpcShopResult {
            look_text: "x".into(),
            ..sold.clone()
        },
        NpcShopResult {
            total_gold_units: 0,
            ..sold.clone()
        },
        NpcShopResult {
            total_gold_units: 1,
            ..look.clone()
        },
    ] {
        assert_eq!(
            encode_npc_shop_result(&wrong),
            Err(NpcShopWireError::Malformed)
        );
    }
    for over in [
        NpcShopResult {
            look_text: "x".repeat(MAX_ITEM_LOOK_TEXT_BYTES + 1),
            ..look
        },
        NpcShopResult {
            sold_units: MAX_SELL_ALL_UNITS + 1,
            ..sold.clone()
        },
        NpcShopResult {
            total_gold_units: MAX_SELL_ALL_GOLD_UNITS + 1,
            ..sold
        },
    ] {
        assert_eq!(
            encode_npc_shop_result(&over),
            Err(NpcShopWireError::LimitExceeded)
        );
    }
}

#[test]
fn npc_shop_result_refuses_a_second_encoding() {
    // An explicit zero units field and an empty text field are not the canonical bytes.
    let mut zero = vec![1 << 3];
    push_varint(&mut zero, NpcIntentDisposition::Rejected as u64);
    for extra in [vec![3 << 3, 0], vec![2 << 3 | 2, 0]] {
        assert_eq!(
            decode_npc_shop_result(&[zero.as_slice(), &extra].concat()),
            Err(NpcShopWireError::Malformed)
        );
    }
    assert_eq!(
        decode_npc_shop_result(&[8, 0]),
        Err(NpcShopWireError::Malformed)
    );
}
