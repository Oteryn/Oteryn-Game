#![allow(clippy::expect_used)]

use super::*;
use crate::charm_wire::push_varint;
use serde_json::Value;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const RESOURCE_REGISTRY: &str =
    include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
const NPC_PROTO: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/npc_service_v1.proto");

fn npc(generation: u64) -> EntityRef {
    EntityRef {
        identity: [0xA5; ENTITY_IDENTITY_BYTES],
        generation,
    }
}

/// The wire form of an `EntityRefV1` with the given raw fields.
fn raw_npc(identity: &[u8], generation: Option<u64>) -> Vec<u8> {
    let mut inner = Vec::new();
    push_message_field(&mut inner, 1, identity);
    if let Some(generation) = generation {
        push_varint_field(&mut inner, 2, generation);
    }
    inner
}

fn valid_npc() -> Vec<u8> {
    raw_npc(&[0xA5; ENTITY_IDENTITY_BYTES], Some(7))
}

fn talk(text: &str) -> NpcTalkIntent {
    NpcTalkIntent {
        npc_actor: npc(u64::MAX),
        text: text.to_owned(),
    }
}

fn trade() -> NpcTradeIntent {
    NpcTradeIntent {
        npc_actor: npc(u64::MAX),
        catalogue_revision: u64::MAX,
        offer_index: (MAX_OFFERS - 1) as u32,
        side: NpcTradeSide::Sell,
        quantity: u32::MAX,
        expected_unit_price: u32::MAX,
    }
}

fn offer(item: u32) -> NpcOffer {
    NpcOffer {
        item_definition_ref: item,
        side: NpcTradeSide::Buy,
        count: u32::MAX,
        unit_price: u32::MAX,
    }
}

fn window(offers: usize) -> NpcTradeWindow {
    NpcTradeWindow {
        open: Some((npc(u64::MAX), u64::MAX)),
        offers: (0..offers).map(|_| offer(u32::MAX)).collect(),
    }
}

fn conversation(lines: usize, line_bytes: usize) -> NpcConversation {
    NpcConversation {
        open: Some((npc(u64::MAX), NpcConversationState::Confirming)),
        lines: vec!["a".repeat(line_bytes); lines],
    }
}

#[test]
fn the_intents_round_trip() {
    let intent = talk("hello");
    let encoded = encode_npc_talk_intent(&intent).expect("encode");
    assert_eq!(decode_npc_talk_intent(&encoded), Ok(intent));
    let intent = talk("zażółć gęślą jaźń");
    let encoded = encode_npc_talk_intent(&intent).expect("encode");
    assert_eq!(decode_npc_talk_intent(&encoded), Ok(intent));

    for side in [NpcTradeSide::Buy, NpcTradeSide::Sell] {
        let intent = NpcTradeIntent {
            side,
            catalogue_revision: 0,
            offer_index: 0,
            expected_unit_price: 0,
            quantity: 1,
            ..trade()
        };
        let encoded = encode_npc_trade_intent(&intent).expect("encode");
        assert_eq!(decode_npc_trade_intent(&encoded), Ok(intent));
    }
    let encoded = encode_npc_trade_intent(&trade()).expect("encode");
    assert!(encoded.len() <= MAX_NPC_TRADE_INTENT_BYTES);
    assert_eq!(decode_npc_trade_intent(&encoded), Ok(trade()));
}

#[test]
fn the_results_round_trip_and_refuse_anything_else() {
    for disposition in NpcIntentDisposition::ALL {
        let encoded = encode_npc_intent_result(disposition);
        assert!(encoded.len() <= MAX_NPC_INTENT_RESULT_BYTES);
        assert_eq!(decode_npc_intent_result(&encoded), Ok(disposition));
    }
    for payload in [&[][..], &[0x08, 0x00], &[0x08, 0x07], &[0x10, 0x01]] {
        assert_eq!(
            decode_npc_intent_result(payload),
            Err(NpcServiceWireError::Malformed)
        );
    }
    assert_eq!(
        decode_npc_intent_result(&[0x08, 0x01, 0x08, 0x01]),
        Err(NpcServiceWireError::LimitExceeded)
    );
}

#[test]
fn the_talk_text_is_bounded_at_255_bytes() {
    let at_maximum = talk(&"a".repeat(MAX_TALK_TEXT_BYTES));
    let encoded = encode_npc_talk_intent(&at_maximum).expect("max");
    assert_eq!(encoded.len(), MAX_NPC_TALK_INTENT_BYTES);
    assert_eq!(decode_npc_talk_intent(&encoded), Ok(at_maximum));

    let over = talk(&"a".repeat(MAX_TALK_TEXT_BYTES + 1));
    assert_eq!(
        encode_npc_talk_intent(&over),
        Err(NpcServiceWireError::LimitExceeded)
    );
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_message_field(&mut payload, 2, over.text.as_bytes());
    assert_eq!(
        decode_npc_talk_intent(&payload),
        Err(NpcServiceWireError::LimitExceeded)
    );
    // A multi-byte text counts bytes, not characters.
    let wide = talk(&"ż".repeat(MAX_TALK_TEXT_BYTES / 2 + 1));
    assert_eq!(
        encode_npc_talk_intent(&wide),
        Err(NpcServiceWireError::LimitExceeded)
    );
}

#[test]
fn the_talk_text_refuses_empty_control_and_invalid_utf8() {
    for text in ["", "a\nb", "a\tb", "a\u{0}b", "a\u{7f}b", "a\u{85}b", "\r"] {
        assert_eq!(
            encode_npc_talk_intent(&talk(text)),
            Err(NpcServiceWireError::Malformed),
            "{text:?}"
        );
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &valid_npc());
        push_message_field(&mut payload, 2, text.as_bytes());
        assert_eq!(
            decode_npc_talk_intent(&payload),
            Err(NpcServiceWireError::Malformed),
            "{text:?}"
        );
    }
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_message_field(&mut payload, 2, &[0xff, 0xfe]);
    assert_eq!(
        decode_npc_talk_intent(&payload),
        Err(NpcServiceWireError::Malformed)
    );
}

#[test]
fn an_npc_reference_needs_a_16_byte_identity_and_a_non_zero_generation() {
    assert_eq!(
        encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc(0),
            ..talk("hi")
        }),
        Err(NpcServiceWireError::Malformed)
    );
    for inner in [
        raw_npc(&[0xA5; 15], Some(7)),
        raw_npc(&[0xA5; 17], Some(7)),
        raw_npc(&[0xA5; 16], Some(0)),
        raw_npc(&[0xA5; 16], None),
        Vec::new(),
    ] {
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &inner);
        push_message_field(&mut payload, 2, b"hi");
        assert_eq!(
            decode_npc_talk_intent(&payload),
            Err(NpcServiceWireError::Malformed),
            "{inner:?}"
        );
    }
    // The reference is required.
    let mut payload = Vec::new();
    push_message_field(&mut payload, 2, b"hi");
    assert_eq!(
        decode_npc_talk_intent(&payload),
        Err(NpcServiceWireError::Malformed)
    );
}

#[test]
fn the_offer_index_is_bounded_at_1024_offers() {
    let at_maximum = NpcTradeIntent {
        offer_index: (MAX_OFFERS - 1) as u32,
        ..trade()
    };
    let encoded = encode_npc_trade_intent(&at_maximum).expect("max");
    assert_eq!(decode_npc_trade_intent(&encoded), Ok(at_maximum));
    let over = NpcTradeIntent {
        offer_index: MAX_OFFERS as u32,
        ..trade()
    };
    assert_eq!(
        encode_npc_trade_intent(&over),
        Err(NpcServiceWireError::LimitExceeded)
    );
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_varint_field(&mut payload, 3, MAX_OFFERS as u64);
    push_varint_field(&mut payload, 4, 1);
    push_varint_field(&mut payload, 5, 1);
    assert_eq!(
        decode_npc_trade_intent(&payload),
        Err(NpcServiceWireError::LimitExceeded)
    );
}

#[test]
fn a_trade_intent_needs_a_side_and_a_quantity_and_refuses_bad_enums() {
    assert_eq!(
        encode_npc_trade_intent(&NpcTradeIntent {
            quantity: 0,
            ..trade()
        }),
        Err(NpcServiceWireError::Malformed)
    );
    let build = |side: Option<u64>, quantity: Option<u64>| {
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &valid_npc());
        if let Some(side) = side {
            push_varint_field(&mut payload, 4, side);
        }
        if let Some(quantity) = quantity {
            push_varint_field(&mut payload, 5, quantity);
        }
        payload
    };
    assert!(decode_npc_trade_intent(&build(Some(1), Some(1))).is_ok());
    for payload in [
        build(None, Some(1)),
        build(Some(0), Some(1)),
        build(Some(3), Some(1)),
        build(Some(1), None),
        build(Some(1), Some(0)),
        build(Some(1), Some(u64::from(u32::MAX) + 1)),
    ] {
        assert_eq!(
            decode_npc_trade_intent(&payload),
            Err(NpcServiceWireError::Malformed)
        );
    }
}

#[test]
fn a_decoder_refuses_unknown_repeated_packed_and_wrongly_typed_fields() {
    let mut base = Vec::new();
    push_message_field(&mut base, 1, &valid_npc());
    push_varint_field(&mut base, 4, 1);
    push_varint_field(&mut base, 5, 1);
    assert!(decode_npc_trade_intent(&base).is_ok());
    let with = |extra: &dyn Fn(&mut Vec<u8>)| {
        let mut payload = base.clone();
        extra(&mut payload);
        payload
    };
    let cases = [
        // Unknown field and unknown wire type.
        with(&|p| push_varint_field(p, 7, 1)),
        with(&|p| push_message_field(p, 9, b"x")),
        // Repeated singular fields.
        with(&|p| push_message_field(p, 1, &valid_npc())),
        with(&|p| push_varint_field(p, 4, 2)),
        with(&|p| push_varint_field(p, 5, 2)),
        // A packed scalar (length-delimited) where a varint is declared.
        with(&|p| push_message_field(p, 3, &[0x01])),
        with(&|p| push_message_field(p, 6, &[0x01])),
        // A varint where a message is declared.
        with(&|p| push_varint_field(p, 1, 1)),
        // Truncated tail.
        with(&|p| p.push(0x28)),
    ];
    for payload in cases {
        assert_eq!(
            decode_npc_trade_intent(&payload),
            Err(NpcServiceWireError::Malformed),
            "{payload:?}"
        );
    }
    // Talk: a repeated text and a varint text.
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_message_field(&mut payload, 2, b"a");
    push_message_field(&mut payload, 2, b"b");
    assert_eq!(
        decode_npc_talk_intent(&payload),
        Err(NpcServiceWireError::Malformed)
    );
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_varint_field(&mut payload, 2, 1);
    assert_eq!(
        decode_npc_talk_intent(&payload),
        Err(NpcServiceWireError::Malformed)
    );
}

#[test]
fn an_intent_over_its_byte_bound_is_refused_before_it_is_read() {
    assert_eq!(
        decode_npc_talk_intent(&vec![0; MAX_NPC_TALK_INTENT_BYTES + 1]),
        Err(NpcServiceWireError::LimitExceeded)
    );
    assert_eq!(
        decode_npc_trade_intent(&[0; MAX_NPC_TRADE_INTENT_BYTES + 1]),
        Err(NpcServiceWireError::LimitExceeded)
    );
}

#[test]
fn a_conversation_round_trips() {
    for conversation in [
        NpcConversation::default(),
        NpcConversation {
            open: Some((npc(3), NpcConversationState::Open)),
            lines: Vec::new(),
        },
        NpcConversation {
            open: Some((npc(3), NpcConversationState::Confirming)),
            lines: vec!["Hello.".to_owned(), "Do you want to travel?".to_owned()],
        },
        // A closing payload carries the farewell once, with no conversation.
        NpcConversation {
            open: None,
            lines: vec!["Farewell.".to_owned()],
        },
    ] {
        let encoded = encode_npc_conversation(&conversation).expect("encode");
        assert_eq!(decode_npc_conversation(&encoded), Ok(conversation));
    }
    assert!(
        encode_npc_conversation(&NpcConversation::default())
            .expect("empty")
            .is_empty()
    );
}

#[test]
fn reply_lines_are_bounded_at_32_lines_and_2048_bytes_each() {
    let at_maximum = conversation(MAX_REPLY_LINES, MAX_REPLY_LINE_BYTES);
    let encoded = encode_npc_conversation(&at_maximum).expect("max");
    assert_eq!(encoded.len(), MAX_NPC_CONVERSATION_BYTES);
    assert_eq!(decode_npc_conversation(&encoded), Ok(at_maximum));

    // One line more.
    assert_eq!(
        encode_npc_conversation(&conversation(MAX_REPLY_LINES + 1, 1)),
        Err(NpcServiceWireError::LimitExceeded)
    );
    let mut payload = Vec::new();
    for _ in 0..=MAX_REPLY_LINES {
        push_message_field(&mut payload, 3, b"a");
    }
    assert_eq!(
        decode_npc_conversation(&payload),
        Err(NpcServiceWireError::LimitExceeded)
    );
    // One byte more in a line.
    assert_eq!(
        encode_npc_conversation(&conversation(1, MAX_REPLY_LINE_BYTES + 1)),
        Err(NpcServiceWireError::LimitExceeded)
    );
    let mut payload = Vec::new();
    push_message_field(
        &mut payload,
        3,
        "a".repeat(MAX_REPLY_LINE_BYTES + 1).as_bytes(),
    );
    assert_eq!(
        decode_npc_conversation(&payload),
        Err(NpcServiceWireError::LimitExceeded)
    );
    assert_eq!(
        decode_npc_conversation(&vec![0; MAX_NPC_CONVERSATION_BYTES + 1]),
        Err(NpcServiceWireError::LimitExceeded)
    );
}

#[test]
fn a_reply_line_refuses_empty_control_characters_and_a_half_open_conversation() {
    for line in ["", "one\ntwo", "a\u{0}", "a\u{1b}[0m"] {
        let conversation = NpcConversation {
            open: None,
            lines: vec![line.to_owned()],
        };
        assert_eq!(
            encode_npc_conversation(&conversation),
            Err(NpcServiceWireError::Malformed),
            "{line:?}"
        );
        let mut payload = Vec::new();
        push_message_field(&mut payload, 3, line.as_bytes());
        assert_eq!(
            decode_npc_conversation(&payload),
            Err(NpcServiceWireError::Malformed),
            "{line:?}"
        );
    }
    // The NPC without a state, and a state without the NPC.
    let mut only_npc = Vec::new();
    push_message_field(&mut only_npc, 1, &valid_npc());
    let mut only_state = Vec::new();
    push_varint_field(&mut only_state, 2, 1);
    for payload in [only_npc, only_state] {
        assert_eq!(
            decode_npc_conversation(&payload),
            Err(NpcServiceWireError::Malformed)
        );
    }
    // A zero, unknown or repeated state, an unknown field and a packed state.
    for state in [0, 3] {
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &valid_npc());
        push_varint_field(&mut payload, 2, state);
        assert_eq!(
            decode_npc_conversation(&payload),
            Err(NpcServiceWireError::Malformed)
        );
    }
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_varint_field(&mut payload, 2, 1);
    for extra in [
        (2, 0, vec![1]),
        (4, 0, vec![1]),
        (2, 2, vec![1, 1]),
        (1, 0, vec![1]),
    ] {
        let mut broken = payload.clone();
        push_varint(&mut broken, (extra.0 << 3) | extra.1);
        if extra.1 == 2 {
            push_varint(&mut broken, extra.2.len() as u64);
        }
        broken.extend_from_slice(&extra.2);
        assert_eq!(
            decode_npc_conversation(&broken),
            Err(NpcServiceWireError::Malformed),
            "{extra:?}"
        );
    }
    // A generation of zero is a server fault.
    assert_eq!(
        encode_npc_conversation(&NpcConversation {
            open: Some((npc(0), NpcConversationState::Open)),
            lines: Vec::new(),
        }),
        Err(NpcServiceWireError::Malformed)
    );
}

#[test]
fn a_trade_window_round_trips() {
    for window in [
        NpcTradeWindow::default(),
        NpcTradeWindow {
            open: Some((npc(3), 0)),
            offers: Vec::new(),
        },
        NpcTradeWindow {
            open: Some((npc(3), 12)),
            offers: vec![
                NpcOffer {
                    item_definition_ref: 3031,
                    side: NpcTradeSide::Buy,
                    count: 1,
                    unit_price: 10_000_000,
                },
                NpcOffer {
                    item_definition_ref: 3035,
                    side: NpcTradeSide::Sell,
                    count: 100,
                    unit_price: 0,
                },
            ],
        },
    ] {
        let encoded = encode_npc_trade_window(&window).expect("encode");
        assert_eq!(decode_npc_trade_window(&encoded), Ok(window));
    }
}

#[test]
fn offers_are_bounded_at_1024() {
    let at_maximum = window(MAX_OFFERS);
    let encoded = encode_npc_trade_window(&at_maximum).expect("max");
    assert_eq!(encoded.len(), MAX_NPC_TRADE_WINDOW_BYTES);
    assert_eq!(decode_npc_trade_window(&encoded), Ok(at_maximum));

    assert_eq!(
        encode_npc_trade_window(&window(MAX_OFFERS + 1)),
        Err(NpcServiceWireError::LimitExceeded)
    );
    let mut payload = Vec::new();
    for _ in 0..=MAX_OFFERS {
        push_message_field(&mut payload, 3, &encode_offer(&offer(1)).expect("offer"));
    }
    let truncated = decode_npc_trade_window(&payload[..payload.len() - 1]);
    assert!(truncated.is_err());
    assert_eq!(
        decode_npc_trade_window(&payload),
        Err(NpcServiceWireError::LimitExceeded)
    );
}

#[test]
fn a_757_offer_snapshot_encodes_within_its_registered_byte_bound() {
    let measured = window(757);
    let encoded = encode_npc_trade_window(&measured).expect("757 offers");
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let domain = protocol["state_domains"]
        .as_array()
        .expect("domains")
        .iter()
        .find(|entry| entry["id"] == STATE_DOMAIN_NPC_TRADE_WINDOW)
        .expect("domain 8");
    for key in ["delta_types", "snapshot_types"] {
        let registered = domain[key][0]["max_payload_bytes"].as_u64().expect("bound");
        assert!(encoded.len() as u64 <= registered);
        assert_eq!(registered, MAX_NPC_TRADE_WINDOW_BYTES as u64);
    }
    const { assert!(MAX_OFFERS >= 757) };
    const { assert!(MAX_NPC_TRADE_WINDOW_BYTES <= crate::MAX_SNAPSHOT_CHUNK_BYTES) };
    const { assert!(MAX_NPC_CONVERSATION_BYTES <= crate::MAX_SNAPSHOT_CHUNK_BYTES) };
    assert_eq!(decode_npc_trade_window(&encoded), Ok(measured));
}

#[test]
fn a_trade_window_refuses_bad_offers_and_inconsistent_state() {
    let bad_offers = [
        NpcOffer {
            item_definition_ref: 0,
            ..offer(1)
        },
        NpcOffer {
            count: 0,
            ..offer(1)
        },
    ];
    for bad in bad_offers {
        assert_eq!(
            encode_npc_trade_window(&NpcTradeWindow {
                open: Some((npc(1), 1)),
                offers: vec![bad],
            }),
            Err(NpcServiceWireError::Malformed)
        );
    }
    // Offers without an NPC.
    assert_eq!(
        encode_npc_trade_window(&NpcTradeWindow {
            open: None,
            offers: vec![offer(1)],
        }),
        Err(NpcServiceWireError::Malformed)
    );
    let raw_offer = |fields: &dyn Fn(&mut Vec<u8>)| {
        let mut inner = Vec::new();
        fields(&mut inner);
        let mut payload = Vec::new();
        push_message_field(&mut payload, 1, &valid_npc());
        push_message_field(&mut payload, 3, &inner);
        payload
    };
    let good = |p: &mut Vec<u8>| {
        push_varint_field(p, 1, 5);
        push_varint_field(p, 2, 1);
        push_varint_field(p, 3, 1);
    };
    assert!(decode_npc_trade_window(&raw_offer(&good)).is_ok());
    let cases = [
        raw_offer(&|p| {
            good(p);
            push_varint_field(p, 5, 1);
        }),
        raw_offer(&|p| {
            good(p);
            push_varint_field(p, 3, 1);
        }),
        raw_offer(&|p| {
            good(p);
            push_message_field(p, 4, &[0x01]);
        }),
        raw_offer(&|p| {
            push_varint_field(p, 1, 0);
            push_varint_field(p, 2, 1);
            push_varint_field(p, 3, 1);
        }),
        raw_offer(&|p| {
            push_varint_field(p, 1, 5);
            push_varint_field(p, 2, 3);
            push_varint_field(p, 3, 1);
        }),
        raw_offer(&|p| {
            push_varint_field(p, 1, 5);
            push_varint_field(p, 3, 1);
        }),
        raw_offer(&|p| {
            push_varint_field(p, 1, 5);
            push_varint_field(p, 2, 1);
        }),
        raw_offer(&|p| {
            push_varint_field(p, 1, u64::from(u32::MAX) + 1);
            push_varint_field(p, 2, 1);
            push_varint_field(p, 3, 1);
        }),
    ];
    for payload in cases {
        assert_eq!(
            decode_npc_trade_window(&payload),
            Err(NpcServiceWireError::Malformed),
            "{payload:?}"
        );
    }
    // A revision without an NPC, a repeated NPC and a repeated revision.
    let mut payload = Vec::new();
    push_varint_field(&mut payload, 2, 1);
    assert_eq!(
        decode_npc_trade_window(&payload),
        Err(NpcServiceWireError::Malformed)
    );
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_message_field(&mut payload, 1, &valid_npc());
    assert_eq!(
        decode_npc_trade_window(&payload),
        Err(NpcServiceWireError::Malformed)
    );
    let mut payload = Vec::new();
    push_message_field(&mut payload, 1, &valid_npc());
    push_varint_field(&mut payload, 2, 1);
    push_varint_field(&mut payload, 2, 2);
    assert_eq!(
        decode_npc_trade_window(&payload),
        Err(NpcServiceWireError::Malformed)
    );
    assert_eq!(
        decode_npc_trade_window(&vec![0; MAX_NPC_TRADE_WINDOW_BYTES + 1]),
        Err(NpcServiceWireError::LimitExceeded)
    );
}

#[test]
fn the_registries_bind_the_npc_service_constants() {
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let find = |list: &str, id: u32| {
        let matching: Vec<&Value> = protocol[list]
            .as_array()
            .expect(list)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matching.len(), 1, "{list} {id}");
        matching[0].clone()
    };
    let capability = find("capabilities", CAPABILITY_NPC_SERVICE_V1);
    assert_eq!(capability["name"], "NPC_SERVICE_V1");
    assert_eq!(capability["offered"], false);
    assert!(capability.get("requires").is_none());
    assert!(
        capability["offer_gate"]
            .as_str()
            .expect("offer gate")
            .starts_with("NPC-TALK-1:")
    );
    assert_eq!(
        capability["command_types"],
        serde_json::json!([COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT])
    );
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW])
    );

    let schema = "docs/contracts/protocol-oteryn/v1/npc_service_v1.proto#";
    for (id, name, payload, maximum) in [
        (
            COMMAND_TYPE_NPC_TALK_INTENT,
            "NPC_TALK_INTENT",
            "NpcTalkIntentV1",
            MAX_NPC_TALK_INTENT_BYTES,
        ),
        (
            COMMAND_TYPE_NPC_TRADE_INTENT,
            "NPC_TRADE_INTENT",
            "NpcTradeIntentV1",
            MAX_NPC_TRADE_INTENT_BYTES,
        ),
    ] {
        let command = find("command_types", id);
        assert_eq!(command["name"], name);
        assert_eq!(command["capability"], CAPABILITY_NPC_SERVICE_V1);
        assert_eq!(command["payload_schema"], format!("{schema}{payload}"));
        assert_eq!(
            command["result_schema"],
            format!("{schema}NpcIntentResultV1")
        );
        assert_eq!(command["max_payload_bytes"].as_u64(), Some(maximum as u64));
        assert_eq!(
            command["max_result_payload_bytes"].as_u64(),
            Some(MAX_NPC_INTENT_RESULT_BYTES as u64)
        );
        assert!(NPC_PROTO.contains(&format!("message {payload} {{")));
    }
    assert!(NPC_PROTO.contains("message NpcIntentResultV1 {"));

    for (id, name, message, maximum) in [
        (
            STATE_DOMAIN_NPC_CONVERSATION,
            "NPC_CONVERSATION",
            "NpcConversationV1",
            MAX_NPC_CONVERSATION_BYTES,
        ),
        (
            STATE_DOMAIN_NPC_TRADE_WINDOW,
            "NPC_TRADE_WINDOW",
            "NpcTradeWindowV1",
            MAX_NPC_TRADE_WINDOW_BYTES,
        ),
    ] {
        let domain = find("state_domains", id);
        assert_eq!(domain["name"], name);
        assert_eq!(domain["capability"], CAPABILITY_NPC_SERVICE_V1);
        for (key, type_id) in [("delta_types", 1), ("snapshot_types", 1)] {
            let types = domain[key].as_array().expect(key);
            assert_eq!(types.len(), 1);
            assert_eq!(types[0]["id"], type_id);
            assert_eq!(types[0]["max_payload_bytes"].as_u64(), Some(maximum as u64));
            assert_eq!(types[0]["payload_schema"], format!("{schema}{message}"));
        }
        assert!(NPC_PROTO.contains(&format!("message {message} {{")));
    }
    assert_eq!(SNAPSHOT_TYPE_NPC_CONVERSATION_V1, 1);
    assert_eq!(DELTA_TYPE_NPC_CONVERSATION_V1, 1);
    assert_eq!(SNAPSHOT_TYPE_NPC_TRADE_WINDOW_V1, 1);
    assert_eq!(DELTA_TYPE_NPC_TRADE_WINDOW_V1, 1);
    assert_eq!(MAX_NPC_TALK_INTENT_BYTES, 289);
    assert_eq!(MAX_NPC_TRADE_INTENT_BYTES, 62);
    assert_eq!(MAX_NPC_CONVERSATION_BYTES, 65_665);
    assert_eq!(MAX_NPC_TRADE_WINDOW_BYTES, 22_570);

    let resources: Value = serde_json::from_str(RESOURCE_REGISTRY).expect("resource registry");
    let limit = |row: &str| {
        resources["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .find(|entry| entry["id"] == row)
            .and_then(|entry| entry["hard_maximum"].as_u64())
    };
    for (row, value) in [
        ("NPC0-RL-01", MAX_TALK_TEXT_BYTES as u64),
        ("NPC0-RL-02", MAX_REPLY_LINES as u64),
        ("NPC0-RL-02-LINE-BYTES", MAX_REPLY_LINE_BYTES as u64),
        ("NPC0-RL-02-BYTES", MAX_NPC_CONVERSATION_BYTES as u64),
        ("NPC0-RL-03", MAX_OFFERS as u64),
        ("NPC0-RL-03-BYTES", MAX_NPC_TRADE_WINDOW_BYTES as u64),
        ("NPC0-RL-04", u64::from(MAX_TALK_INTENTS_PER_SECOND)),
        ("NPC0-RL-05", u64::from(MAX_OPEN_CONVERSATIONS_PER_NPC)),
        ("NPC0-RL-06", u64::from(TRAVEL_CONFIRMATION_TIMEOUT_SECONDS)),
        ("NPC0-RL-07", u64::from(TALK_RANGE_TILES)),
    ] {
        assert_eq!(limit(row), Some(value), "{row}");
    }
    assert_eq!(TALK_RANGE_TILES, 4);
}
