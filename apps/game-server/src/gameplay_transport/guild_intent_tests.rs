#![allow(
    clippy::expect_used,
    reason = "Value-parser fixtures explicitly fail on codec errors"
)]
use super::*;
use oteryn_protocol_oteryn::guild::{
    GuildCursor, GuildCursorKey, GuildSection, encode_guild_query,
};
fn decode(intent: GuildIntent) -> Result<GuildRequestData, GuildInputError> {
    decode_guild_request(&encode_guild_intent(&intent).expect("wire fixture"))
}
fn guild(seed: u8) -> GuildIdentity {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    GuildIdentity::decode(&bytes).expect("UUIDv7 fixture")
}

#[test]
fn names_use_actual_character_policy_not_a_second_wire_grammar() {
    for value in ["A", "Aldric1", "Al  Dric", " Aldric", "Élan"] {
        assert!(matches!(
            decode(GuildIntent::Found { name: value.into() }),
            Err(GuildInputError::NameInvalid)
        ));
        assert!(matches!(
            decode(GuildIntent::Invite {
                character: value.into()
            }),
            Err(GuildInputError::NameInvalid)
        ));
    }
    let valid = "Sir Aldric Of Thais";
    assert_eq!(
        decode(GuildIntent::Found { name: valid.into() })
            .expect("native name")
            .intent,
        GuildIntent::Found { name: valid.into() }
    );
}

#[test]
fn native_chat_parser_bounds_controls_trimming_and_explicit_empty_metadata() {
    for value in ["a\nb", "a\0b", "\ttext"] {
        assert!(matches!(
            decode(GuildIntent::SetMessage {
                text: value.into(),
                expected_revision: 1
            }),
            Err(GuildInputError::TextInvalid)
        ));
    }
    let title = "é".repeat(30);
    assert!(matches!(
        decode(GuildIntent::SetTitle {
            character: "Aldric".into(),
            title,
            expected_revision: 1
        }),
        Err(GuildInputError::TextInvalid)
    ));
    assert_eq!(
        decode(GuildIntent::SetMessage {
            text: "  Hello  ".into(),
            expected_revision: 9
        })
        .expect("native trim")
        .intent,
        GuildIntent::SetMessage {
            text: "Hello".into(),
            expected_revision: 9
        }
    );
    for intent in [
        GuildIntent::SetMessage {
            text: String::new(),
            expected_revision: 1,
        },
        GuildIntent::SetTitle {
            character: "Aldric".into(),
            title: String::new(),
            expected_revision: 1,
        },
        GuildIntent::SetTitle {
            character: "Aldric".into(),
            title: "é".repeat(29),
            expected_revision: 1,
        },
        GuildIntent::SetMessage {
            text: "😀".repeat(255),
            expected_revision: 1,
        },
    ] {
        assert_eq!(decode(intent.clone()).expect("boundary").intent, intent);
    }
    assert!(matches!(
        decode(GuildIntent::SetMessage {
            text: "   ".into(),
            expected_revision: 1
        }),
        Err(GuildInputError::TextInvalid)
    ));
}

#[test]
fn canonical_binding_preserves_original_request_before_normalization() {
    let a = decode(GuildIntent::SetMessage {
        text: " hello ".into(),
        expected_revision: 1,
    })
    .expect("a");
    let b = decode(GuildIntent::SetMessage {
        text: "hello".into(),
        expected_revision: 1,
    })
    .expect("b");
    assert_eq!(a.intent, b.intent);
    assert_ne!(a.body_digest, b.body_digest);
    let same = decode(GuildIntent::SetMessage {
        text: "hello".into(),
        expected_revision: 1,
    })
    .expect("same");
    assert_eq!(b.body_digest, same.body_digest);
    let different = decode(GuildIntent::SetMessage {
        text: "hello".into(),
        expected_revision: 2,
    })
    .expect("different revision");
    assert_ne!(b.body_digest, different.body_digest);
    //Same field values in a different legal protobuf order have one canonical request digest.
    let reordered = [0x52, 9, 0x10, 1, 0x0a, 5, b'h', b'e', b'l', b'l', b'o'];
    assert_eq!(
        decode_guild_request(&reordered)
            .expect("reordered")
            .body_digest,
        b.body_digest
    );
    assert_eq!(format!("{b:?}"), "GuildRequestData { .. }");
}

#[test]
fn query_binding_is_structural_data_not_a_reader_or_membership_grant() {
    let query = GuildQuery {
        section: GuildSection::Roster,
        public_guild: Some("Other Guild".into()),
        cursor: Some(GuildCursor {
            guild: Some(guild(1)),
            key: GuildCursorKey::Roster {
                level: 1,
                character: *guild(3).as_bytes(),
            },
        }),
    };
    let bytes = encode_guild_query(&query).expect("query");
    let decoded = decode_guild_query_data(&bytes).expect("native name");
    assert_eq!(decoded, query);
    assert_eq!(check_resolved_cursor(&decoded, guild(1)), Ok(()));
    assert_eq!(
        check_resolved_cursor(&decoded, guild(2)),
        Err(GuildInputError::CursorBinding)
    );
    let mut bad = query;
    bad.public_guild = Some("Other  Guild".into());
    assert_eq!(
        decode_guild_query_data(&encode_guild_query(&bad).expect("bounded wire")),
        Err(GuildInputError::NameInvalid)
    );
    assert!(decode_guild_request(&[0x2a, 0, 0x2a, 0]).is_err());
    let claims = GuildQuery {
        section: GuildSection::DisbandClaims,
        public_guild: None,
        cursor: Some(GuildCursor {
            guild: None,
            key: GuildCursorKey::DisbandClaim { guild: guild(3) },
        }),
    };
    let data = decode_guild_query_data(&encode_guild_query(&claims).expect("real UUID key data"))
        .expect("decode only");
    assert_eq!(
        check_resolved_cursor(&data, guild(1)),
        Err(GuildInputError::AccountWorldClaimsContextRequired)
    );
}
