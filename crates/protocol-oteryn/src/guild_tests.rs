#![allow(
    clippy::expect_used,
    reason = "Conformance fixtures explicitly fail on codec errors"
)]
use super::*;

fn uuid(seed: u8) -> [u8; 16] {
    let mut value = [seed; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}
fn guild(seed: u8) -> GuildIdentity {
    GuildIdentity::decode(&uuid(seed)).expect("fixture UUIDv7")
}
fn variants() -> Vec<GuildIntent> {
    vec![
        GuildIntent::Found {
            name: "Knights Of Dawn".into(),
        },
        GuildIntent::Invite {
            character: "Aldric".into(),
        },
        GuildIntent::RevokeInvite {
            character: "Aldric".into(),
        },
        GuildIntent::Accept { guild: guild(1) },
        GuildIntent::Leave,
        GuildIntent::Exclude {
            character: "Aldric".into(),
        },
        GuildIntent::SetRank {
            character: "Aldric".into(),
            level: 20,
            expected_revision: u64::MAX,
        },
        GuildIntent::EditRanks {
            names: vec!["Leader".into(), "Vice Leader".into(), "Member".into()],
            expected_revision: 1,
        },
        GuildIntent::SetTitle {
            character: "Aldric".into(),
            title: String::new(),
            expected_revision: 1,
        },
        GuildIntent::SetMessage {
            text: String::new(),
            expected_revision: 1,
        },
        GuildIntent::Resign {
            successor: "Aldric".into(),
        },
        GuildIntent::Disband {
            confirm: "Knights Of Dawn".into(),
        },
        GuildIntent::Deposit { amount: MAX_AMOUNT },
        GuildIntent::Withdraw { amount: 1 },
        GuildIntent::ClaimDisbandPayout { guild: guild(1) },
    ]
}

#[test]
fn accepted_command_set_roundtrips_and_literal_fixtures_match() {
    for intent in variants() {
        let bytes = encode_guild_intent(&intent).expect("encode");
        assert!(bytes.len() <= MAX_INTENT_BYTES);
        assert_eq!(decode_guild_intent(&bytes), Ok(intent));
    }
    assert_eq!(encode_guild_intent(&GuildIntent::Leave), Ok(vec![0x2a, 0]));
    assert_eq!(
        encode_guild_intent(&GuildIntent::Invite {
            character: "Bob".into()
        }),
        Ok(vec![0x12, 5, 0x0a, 3, b'B', b'o', b'b'])
    );
    assert_eq!(
        decode_guild_intent(&[0x6a, 2, 8, 1]),
        Ok(GuildIntent::Deposit { amount: 1 })
    );
    for result in GuildResult::ALL {
        let bytes = encode_guild_result(result);
        assert_eq!(bytes, vec![8, result as u8]);
        assert_eq!(decode_guild_result(&bytes), Ok(result));
    }
}

#[test]
fn malformed_duplicate_unknown_oneofs_and_fields_fail_closed() {
    for bytes in [
        vec![],
        vec![0x2a],
        vec![0x2a, 1],
        vec![0x2a, 0, 0x2a, 0],
        vec![0x2a, 0, 0x22, 0],
        vec![0x82, 1, 0],
        vec![0x28, 0],
        vec![0x2a, 2, 8, 1],
        vec![0x12, 7, 0x0a, 1, b'A', 0x0a, 2, b'B', b'C'],
        vec![0x12, 3, 0x0a, 1, 0xff],
    ] {
        assert!(decode_guild_intent(&bytes).is_err(), "{bytes:?}");
    }
    for intent in variants() {
        let bytes = encode_guild_intent(&intent).expect("encode");
        for n in 0..bytes.len() {
            assert!(decode_guild_intent(&bytes[..n]).is_err(), "prefix{n}");
        }
    }
    assert!(decode_guild_intent(&vec![0; MAX_INTENT_BYTES + 1]).is_err());
    for bytes in [
        vec![],
        vec![8, 0],
        vec![8, 17],
        vec![8, 1, 8, 2],
        vec![0x0a, 1, 1],
    ] {
        assert!(decode_guild_result(&bytes).is_err());
    }
}

#[test]
fn exact_text_rank_revision_amount_and_uuid_boundaries() {
    let max = GuildIntent::EditRanks {
        names: vec!["a".repeat(MAX_RANK_NAME_BYTES); MAX_RANKS],
        expected_revision: u64::MAX,
    };
    let encoded = encode_guild_intent(&max).expect("exact maximum");
    assert_eq!(encoded.len(), MAX_INTENT_BYTES);
    assert_eq!(decode_guild_intent(&encoded), Ok(max));
    for count in [2, 21] {
        assert!(
            encode_guild_intent(&GuildIntent::EditRanks {
                names: vec!["A".into(); count],
                expected_revision: 1
            })
            .is_err()
        );
    }
    let mut body = Vec::new();
    for _ in 0..21 {
        push_message_field(&mut body, 1, b"A");
    }
    push_varint_field(&mut body, 2, 1);
    let mut bytes = Vec::new();
    push_message_field(&mut bytes, 8, &body);
    assert!(matches!(
        decode_guild_intent(&bytes),
        Err(GuildWireError::LimitExceeded)
    ));
    for intent in [
        GuildIntent::Found {
            name: "a".repeat(30),
        },
        GuildIntent::EditRanks {
            names: vec!["a".repeat(MAX_RANK_NAME_BYTES + 1); 3],
            expected_revision: 1,
        },
        GuildIntent::SetTitle {
            character: "Ab".into(),
            title: "😀".repeat(30),
            expected_revision: 1,
        },
        GuildIntent::SetMessage {
            text: "😀".repeat(256),
            expected_revision: 1,
        },
        GuildIntent::SetRank {
            character: "Ab".into(),
            level: 0,
            expected_revision: 1,
        },
        GuildIntent::SetRank {
            character: "Ab".into(),
            level: 21,
            expected_revision: 1,
        },
        GuildIntent::SetMessage {
            text: "x".into(),
            expected_revision: 0,
        },
        GuildIntent::Deposit { amount: 0 },
        GuildIntent::Withdraw {
            amount: MAX_AMOUNT + 1,
        },
    ] {
        assert!(encode_guild_intent(&intent).is_err());
    }
    for intent in [
        GuildIntent::SetTitle {
            character: "Ab".into(),
            title: "😀".repeat(29),
            expected_revision: 1,
        },
        GuildIntent::SetMessage {
            text: "😀".repeat(255),
            expected_revision: 1,
        },
    ] {
        let bytes = encode_guild_intent(&intent).expect("boundary");
        assert_eq!(decode_guild_intent(&bytes), Ok(intent));
    }
    for bytes in [vec![0; 16], vec![1; 15], vec![1; 17], vec![1; 16]] {
        assert!(GuildIdentity::decode(&bytes).is_err());
    }
}

#[test]
fn query_keysets_roundtrip_and_reject_cross_section_and_guild() {
    let keys = [
        GuildCursorKey::Roster {
            level: 20,
            character: uuid(2),
        },
        GuildCursorKey::Invitation { character: uuid(2) },
        GuildCursorKey::DisbandClaim { guild: guild(3) },
        GuildCursorKey::Activity {
            occurred_at_unix_ms: i64::MAX as u64,
            event: uuid(3),
        },
    ];
    for key in keys {
        let claims = matches!(key, GuildCursorKey::DisbandClaim { .. });
        let cursor = GuildCursor {
            guild: (!claims).then(|| guild(1)),
            key,
        };
        let bytes = encode_guild_cursor(&cursor).expect("cursor");
        assert!(bytes.len() <= MAX_CURSOR_BYTES);
        assert_eq!(decode_guild_cursor(&bytes), Ok(cursor.clone()));
        assert!(cursor.check_binding(guild(2), cursor.section()).is_err());
        assert!(
            cursor
                .check_binding(guild(1), GuildSection::Summary)
                .is_err()
        );
        let query = GuildQuery {
            section: cursor.section(),
            public_guild: None,
            cursor: Some(cursor),
        };
        let bytes = encode_guild_query(&query).expect("query");
        assert_eq!(decode_guild_query(&bytes), Ok(query));
    }
    for query in [
        GuildQuery {
            section: GuildSection::Summary,
            public_guild: None,
            cursor: None,
        },
        GuildQuery {
            section: GuildSection::Public,
            public_guild: Some("Knights Of Dawn".into()),
            cursor: None,
        },
        GuildQuery {
            section: GuildSection::Roster,
            public_guild: Some("Knights Of Dawn".into()),
            cursor: None,
        },
    ] {
        assert_eq!(
            decode_guild_query(&encode_guild_query(&query).expect("query")),
            Ok(query)
        );
    }
    for query in [
        GuildQuery {
            section: GuildSection::Public,
            public_guild: None,
            cursor: None,
        },
        GuildQuery {
            section: GuildSection::Activity,
            public_guild: Some("Other Guild".into()),
            cursor: None,
        },
        GuildQuery {
            section: GuildSection::Roster,
            public_guild: None,
            cursor: Some(GuildCursor {
                guild: Some(guild(1)),
                key: GuildCursorKey::Invitation { character: uuid(2) },
            }),
        },
    ] {
        assert!(encode_guild_query(&query).is_err());
    }
    for key in [
        GuildCursorKey::Roster {
            level: 0,
            character: uuid(2),
        },
        GuildCursorKey::Roster {
            level: 21,
            character: uuid(2),
        },
        GuildCursorKey::Roster {
            level: 1,
            character: [0; 16],
        },
        GuildCursorKey::Invitation { character: [0; 16] },
        GuildCursorKey::Activity {
            occurred_at_unix_ms: i64::MAX as u64 + 1,
            event: uuid(3),
        },
    ] {
        assert!(
            encode_guild_cursor(&GuildCursor {
                guild: Some(guild(1)),
                key
            })
            .is_err()
        );
    }
    assert!(
        encode_guild_cursor(&GuildCursor {
            guild: Some(guild(1)),
            key: GuildCursorKey::DisbandClaim { guild: guild(3) }
        })
        .is_err()
    );
    assert!(
        encode_guild_cursor(&GuildCursor {
            guild: None,
            key: GuildCursorKey::Invitation { character: uuid(2) }
        })
        .is_err()
    );
    for bytes in [
        vec![],
        vec![8, 0],
        vec![8, 7],
        vec![8, 1, 8, 2],
        vec![8, 1, 0x22, 0],
        vec![8, 6, 0x12, 1, 0xff],
    ] {
        assert!(decode_guild_query(&bytes).is_err());
    }
    assert!(decode_guild_cursor(&[0; MAX_CURSOR_BYTES + 1]).is_err());
}

#[test]
fn cursor_identity_section_and_key_shape_are_not_interchangeable() {
    let cursor = GuildCursor {
        guild: Some(guild(1)),
        key: GuildCursorKey::Invitation { character: uuid(2) },
    };
    let bytes = encode_guild_cursor(&cursor).expect("cursor");
    //Change ONLY the declared section, retaining well-formed UUID and row key.
    let mut wrong = bytes.clone();
    assert_eq!(&wrong[18..20], &[16, 3]);
    wrong[19] = 2;
    assert!(decode_guild_cursor(&wrong).is_err());
    let mut duplicate = bytes.clone();
    push_message_field(&mut duplicate, 4, &uuid(2));
    assert!(decode_guild_cursor(&duplicate).is_err());
    let mut unknown = bytes.clone();
    unknown.extend_from_slice(&[48, 1]);
    assert!(decode_guild_cursor(&unknown).is_err());
    for n in 0..bytes.len() {
        assert!(decode_guild_cursor(&bytes[..n]).is_err());
    }
}

#[test]
fn emblems_reuse_opaque_native_identities_and_exact_visibility_budget() {
    let entries = (0..MAX_SNAPSHOT_ENTITIES)
        .map(|n| {
            let mut identity = [0; 16];
            identity[14..].copy_from_slice(&(n as u16).to_be_bytes());
            GuildEmblemEntry {
                entity: EntityRef {
                    identity,
                    generation: u64::MAX,
                },
                emblem: match n % 3 {
                    0 => GuildEmblem::None,
                    1 => GuildEmblem::OwnGuild,
                    _ => GuildEmblem::OtherGuild,
                },
            }
        })
        .collect::<Vec<_>>();
    let bytes = encode_guild_emblems(&entries).expect("256");
    assert_eq!(bytes.len(), MAX_EMBLEMS_BYTES);
    assert_eq!(decode_guild_emblems(&bytes), Ok(entries.clone()));
    let mut extra = entries.clone();
    let mut last = entries[255];
    last.entity.identity = [255; 16];
    extra.push(last);
    assert!(encode_guild_emblems(&extra).is_err());
    let mut duplicate = vec![entries[0], entries[0]];
    duplicate[1].entity.generation = 1;
    assert!(encode_guild_emblems(&duplicate).is_err());
    assert!(encode_guild_emblems(&[entries[1], entries[0]]).is_err());
    let mut zero = entries[0];
    zero.entity.generation = 0;
    assert!(encode_guild_emblems(&[zero]).is_err());
    let bytes = encode_guild_emblems(&[entries[0]]).expect("one");
    let mut unknown = bytes.clone();
    *unknown.last_mut().expect("enum") = 4;
    assert!(decode_guild_emblems(&unknown).is_err());
    let mut privacy = bytes;
    privacy.extend_from_slice(&[0x12, 3, b's', b'e', b'c']);
    assert!(decode_guild_emblems(&privacy).is_err());
    assert_eq!(decode_guild_emblems(&[]), Ok(Vec::new()));
}
