#![allow(clippy::unwrap_used, clippy::panic)] // Test assertions only.

use super::*;

fn id(seed: u16) -> CharacterId {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[14..].copy_from_slice(&seed.to_be_bytes());
    CharacterId::decode(&bytes).unwrap()
}
fn position() -> ActorPosition {
    ActorPosition {
        x: 65535,
        y: 65535,
        floor: -15,
    }
}
fn oneof(tag: u64, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    push_message_field(&mut out, tag, body);
    out
}

#[test]
fn every_variant_round_trips_without_reordering_lists() {
    let cases = [
        HouseIntent::Bid {
            house_key: "oteryn:content.house.thais_1".into(),
            max_gold: u64::MAX,
        },
        HouseIntent::Bid {
            house_key: "oteryn:content.house.thais_1".into(),
            max_gold: 0,
        },
        HouseIntent::MoveOut {
            date_unix_ms: i64::MAX,
        },
        HouseIntent::CancelMoveOut,
        HouseIntent::AclSet {
            role: HouseAclRole::Subowner,
            character_ids: vec![id(2), id(1)],
            expected_revision: u64::MAX,
        },
        HouseIntent::AclSet {
            role: HouseAclRole::Guest,
            character_ids: vec![],
            expected_revision: 0,
        },
        HouseIntent::DoorSet {
            position: position(),
            character_ids: vec![id(3)],
            expected_revision: 0,
        },
        HouseIntent::Kick {
            character_id: id(4),
        },
        HouseIntent::Leave,
    ];
    for value in cases {
        let bytes = encode_house_intent(&value).unwrap();
        assert_eq!(decode_house_intent(&bytes), Ok(value));
    }
}
#[test]
fn fixed_proto3_fixtures_include_empty_variants_and_zero_max() {
    let underground = HouseIntent::DoorSet {
        position: ActorPosition {
            x: 1,
            y: 2,
            floor: -1,
        },
        character_ids: vec![],
        expected_revision: 0,
    };
    // Native signed floor is copied directly: -1 zigzags to1, never to positive floor1.
    let fixture = [42, 8, 10, 6, 8, 2, 16, 4, 24, 1];
    assert_eq!(encode_house_intent(&underground).unwrap(), fixture);
    assert_eq!(decode_house_intent(&fixture), Ok(underground));

    assert_eq!(
        encode_house_intent(&HouseIntent::CancelMoveOut).unwrap(),
        [26, 0]
    );
    assert_eq!(encode_house_intent(&HouseIntent::Leave).unwrap(), [58, 0]);
    assert_eq!(
        encode_house_intent(&HouseIntent::MoveOut { date_unix_ms: 150 }).unwrap(),
        [18, 3, 8, 150, 1]
    );
    assert_eq!(
        decode_house_intent(&[10, 5, 10, 1, b'h', 16, 0]),
        Ok(HouseIntent::Bid {
            house_key: "h".into(),
            max_gold: 0
        })
    );
    assert_eq!(
        encode_house_intent(&HouseIntent::Bid {
            house_key: "h".into(),
            max_gold: 0
        })
        .unwrap(),
        [10, 3, 10, 1, b'h']
    );
}
#[test]
fn every_truncation_of_nonempty_nested_intent_refuses() {
    let body = encode_house_intent(&HouseIntent::DoorSet {
        position: position(),
        character_ids: vec![id(1), id(2)],
        expected_revision: u64::MAX,
    })
    .unwrap();
    for end in 0..body.len() {
        assert!(decode_house_intent(&body[..end]).is_err(), "prefix{end}");
    }
}
#[test]
fn empty_multiple_unknown_oneofs_and_wrong_wire_types_refuse() {
    for bytes in [
        vec![],
        vec![26, 0, 58, 0],
        vec![26, 0, 26, 0],
        vec![66, 0],
        vec![24, 0],
        vec![26, 1, 0],
        vec![58, 2, 8, 1],
    ] {
        assert_eq!(decode_house_intent(&bytes), Err(HouseWireError::Malformed));
    }
}
#[test]
fn singular_duplicates_unknown_and_packed_fields_refuse() {
    for bytes in [
        oneof(1, &[10, 1, b'h', 10, 1, b'h']),
        oneof(1, &[10, 1, b'h', 16, 1, 16, 1]),
        oneof(1, &[10, 1, b'h', 24, 1]),
        oneof(2, &[8, 1, 8, 1]),
        oneof(4, &[8, 1, 8, 1]),
        oneof(4, &[8, 1, 24, 1, 24, 1]),
        oneof(4, &[8, 1, 16, 1]),
        oneof(5, &[10, 0, 10, 0]),
    ] {
        assert_eq!(decode_house_intent(&bytes), Err(HouseWireError::Malformed));
    }
}
#[test]
fn malformed_lengths_varints_utf8_and_missing_required_fields_refuse() {
    for bytes in [
        vec![10, 255],
        vec![10, 20, 10, 1, b'h'],
        oneof(1, &[10, 1, 255]),
        oneof(1, &[]),
        oneof(2, &[]),
        oneof(2, &[8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 2]),
        oneof(4, &[]),
        oneof(4, &[8, 0]),
        oneof(4, &[8, 3]),
        oneof(5, &[]),
        oneof(6, &[10, 0]),
    ] {
        assert_eq!(decode_house_intent(&bytes), Err(HouseWireError::Malformed));
    }
}
#[test]
fn native_uuid_version_and_variant_contract_is_enforced() {
    for bad in [
        vec![0; 16],
        {
            let mut v = id(1).as_bytes().to_vec();
            v[6] = 0x40;
            v
        },
        {
            let mut v = id(1).as_bytes().to_vec();
            v[8] = 0;
            v
        },
        vec![1; 15],
        vec![1; 17],
    ] {
        let mut body = vec![8, 1];
        push_message_field(&mut body, 2, &bad);
        assert_eq!(
            decode_house_intent(&oneof(4, &body)),
            Err(HouseWireError::Malformed)
        );
        let mut kick = Vec::new();
        push_message_field(&mut kick, 1, &bad);
        assert_eq!(
            decode_house_intent(&oneof(6, &kick)),
            Err(HouseWireError::Malformed)
        );
    }
}
#[test]
fn acl_200_and_201_bound_and_duplicates_on_both_directions() {
    let ids: Vec<_> = (0..200).map(id).collect();
    let mut value = HouseIntent::DoorSet {
        position: position(),
        character_ids: ids.clone(),
        expected_revision: u64::MAX,
    };
    let bytes = encode_house_intent(&value).unwrap();
    assert!(bytes.len() <= MAX_HOUSE_INTENT_BYTES);
    assert_eq!(decode_house_intent(&bytes), Ok(value.clone()));
    if let HouseIntent::DoorSet { character_ids, .. } = &mut value {
        character_ids.push(id(200));
    }
    assert_eq!(
        encode_house_intent(&value),
        Err(HouseWireError::LimitExceeded)
    );
    let mut body = vec![8, 1];
    for n in 0..201 {
        push_message_field(&mut body, 2, id(n).as_bytes());
    }
    assert_eq!(
        decode_house_intent(&oneof(4, &body)),
        Err(HouseWireError::LimitExceeded)
    );
    let duplicate = HouseIntent::AclSet {
        role: HouseAclRole::Guest,
        character_ids: vec![id(1), id(1)],
        expected_revision: 0,
    };
    assert_eq!(
        encode_house_intent(&duplicate),
        Err(HouseWireError::Malformed)
    );
    let mut body = vec![8, 2];
    for _ in 0..2 {
        push_message_field(&mut body, 2, id(1).as_bytes());
    }
    assert_eq!(
        decode_house_intent(&oneof(4, &body)),
        Err(HouseWireError::Malformed)
    );
}
#[test]
fn key_and_total_byte_limits_are_symmetric() {
    let mut value = HouseIntent::Bid {
        house_key: "h".repeat(512),
        max_gold: 0,
    };
    assert_eq!(
        decode_house_intent(&encode_house_intent(&value).unwrap()),
        Ok(value.clone())
    );
    if let HouseIntent::Bid { house_key, .. } = &mut value {
        house_key.push('h');
    }
    assert_eq!(
        encode_house_intent(&value),
        Err(HouseWireError::LimitExceeded)
    );
    let mut nested = Vec::new();
    push_message_field(&mut nested, 1, &[b'h'; 513]);
    assert_eq!(
        decode_house_intent(&oneof(1, &nested)),
        Err(HouseWireError::LimitExceeded)
    );
    assert_eq!(
        decode_house_intent(&[0; MAX_HOUSE_INTENT_BYTES + 1]),
        Err(HouseWireError::LimitExceeded)
    );
}
#[test]
fn date_is_positive_signed_milliseconds_and_floor_uses_native_range() {
    for n in [0, -1] {
        assert_eq!(
            encode_house_intent(&HouseIntent::MoveOut { date_unix_ms: n }),
            Err(HouseWireError::Malformed)
        );
    }
    assert_eq!(
        decode_house_intent(&oneof(2, &[8, 0])),
        Err(HouseWireError::Malformed)
    );
    let mut body = Vec::new();
    push_varint_field(&mut body, 1, (i64::MAX as u64) + 1);
    assert_eq!(
        decode_house_intent(&oneof(2, &body)),
        Err(HouseWireError::Malformed)
    );
    for floor in [-16, 1] {
        let mut wire = Vec::new();
        push_message_field(
            &mut wire,
            1,
            &encode_position(&ActorPosition { x: 0, y: 0, floor }),
        );
        assert_eq!(
            decode_house_intent(&oneof(5, &wire)),
            Err(HouseWireError::Malformed)
        );
        let mut pos = position();
        pos.floor = floor;
        let value = HouseIntent::DoorSet {
            position: pos,
            character_ids: vec![],
            expected_revision: 0,
        };
        assert_eq!(encode_house_intent(&value), Err(HouseWireError::Malformed));
    }
}

// Coordinate correction: native world_map::tile_pos, not the broader wire position shape.
fn door_body_at(position: ActorPosition) -> Vec<u8> {
    let mut body = Vec::new();
    push_message_field(&mut body, 1, &encode_position(&position));
    oneof(5, &body)
}
#[test]
fn native_house_door_grid_accepts_underground_boundaries() {
    for position in [
        ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        ActorPosition {
            x: 1,
            y: 2,
            floor: -1,
        },
        ActorPosition {
            x: 65535,
            y: 65535,
            floor: -15,
        },
    ] {
        let value = HouseIntent::DoorSet {
            position,
            character_ids: vec![],
            expected_revision: 0,
        };
        assert_eq!(
            decode_house_intent(&door_body_at(position)),
            Ok(value.clone())
        );
        assert_eq!(
            decode_house_intent(&encode_house_intent(&value).unwrap()),
            Ok(value)
        );
    }
}
#[test]
fn native_house_door_grid_rejects_outside_floors() {
    for floor in [1, 15, -16, i16::MIN, i16::MAX] {
        let position = ActorPosition { x: 0, y: 0, floor };
        let value = HouseIntent::DoorSet {
            position,
            character_ids: vec![],
            expected_revision: 0,
        };
        assert_eq!(encode_house_intent(&value), Err(HouseWireError::Malformed));
        assert_eq!(
            decode_house_intent(&door_body_at(position)),
            Err(HouseWireError::Malformed)
        );
    }
}
#[test]
fn native_house_door_grid_rejects_outside_axes() {
    for position in [
        ActorPosition {
            x: -1,
            y: 0,
            floor: 0,
        },
        ActorPosition {
            x: 0,
            y: -1,
            floor: 0,
        },
        ActorPosition {
            x: 65536,
            y: 0,
            floor: 0,
        },
        ActorPosition {
            x: 0,
            y: 65536,
            floor: 0,
        },
        ActorPosition {
            x: i32::MIN,
            y: 0,
            floor: 0,
        },
        ActorPosition {
            x: 0,
            y: i32::MAX,
            floor: 0,
        },
    ] {
        let value = HouseIntent::DoorSet {
            position,
            character_ids: vec![],
            expected_revision: 0,
        };
        assert_eq!(encode_house_intent(&value), Err(HouseWireError::Malformed));
        assert_eq!(
            decode_house_intent(&door_body_at(position)),
            Err(HouseWireError::Malformed)
        );
    }
}
