use super::*;

const OP: &str = "0192a000-0000-7000-8000-000000000001";
const AUTHORITY: &str = "oteryn:character-authority:primary";
/// §4.2 `CreateCharacter` fixture.
const INTENT: &str = r#"{"contract_version":1,"command":"CreateCharacter","variant":"PLATFORM_USER_CREATE","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","issuer_decision_id":"0192a000-0000-7000-8000-0000000000a1","source_revision":"17","operation_id":"0192a000-0000-7000-8000-000000000001","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","target_world_id":"01934f10-7c02-7001-805b-3b1122334401","requested_name":"Aldric","issued_at_source":"1790000000","expires_at_source":"1790000300","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}"#;

fn bytes(value: &str) -> [u8; 16] {
    uuid_bytes(value).unwrap()
}

fn classify(raw: &str) -> Classified {
    classify_intent(raw.as_bytes(), CREATE_COMMAND, OP)
}

#[test]
fn the_contract_create_intent_decodes() {
    assert_eq!(
        classify(INTENT),
        Classified::Create(CreateIntent {
            source_revision: 17,
            account_id: bytes("0190f2a1-3b4c-7d5e-8f60-718293a4b5c6"),
            target_world_id: bytes("01934f10-7c02-7001-805b-3b1122334401"),
            requested_name: "Aldric".into(),
            issued_at_source: 1_790_000_000,
            expires_at_source: 1_790_000_300,
        })
    );
    // The decoder does not judge the name; the decision stores NAME_INVALID.
    let named = INTENT.replace("Aldric", "A1");
    assert!(matches!(classify(&named), Classified::Create(_)));
}

#[test]
fn intents_are_classified_conflict_unsupported_then_invalid() {
    use Classified::Rejected;
    let transfer = INTENT.replace(
        r#""command":"CreateCharacter""#,
        r#""command":"TransferCharacterOwnership""#,
    );
    assert_eq!(classify(&transfer), Rejected(OPERATION_CONFLICT));
    assert_eq!(
        classify_intent(INTENT.as_bytes(), "RenameCharacter", OP),
        Rejected(OPERATION_CONFLICT)
    );
    let v2 = INTENT.replace(r#""contract_version":1"#, r#""contract_version":2"#);
    assert_eq!(classify(&v2), Rejected(UNSUPPORTED));
    let variant = INTENT.replace("PLATFORM_USER_CREATE", "PLATFORM_ADMIN_CREATE");
    assert_eq!(classify(&variant), Rejected(UNSUPPORTED));
    for invalid in [
        String::new(),
        "not json".into(),
        INTENT.replace(r#""command":"CreateCharacter","#, ""),
        INTENT.replace(r#","audience""#, r#", "audience""#),
        INTENT.replace("Aldric", "Ald\\u0072ic"),
        INTENT.replace(r#""source_revision":"17""#, r#""source_revision":"017""#),
        INTENT.replace(r#""source_revision":"17""#, r#""source_revision":"0""#),
        INTENT.replace(r#""source_revision":"17""#, r#""source_revision":17"#),
        INTENT.replace("1790000300", "1790000301"),
        INTENT.replace("1790000300", "1790000000"),
        INTENT
            .replace(
                r#""issued_at_source":"1790000000""#,
                r#""issued_at_source":"0""#,
            )
            .replace("1790000300", "1"),
        INTENT.replace(
            "0192a000-0000-7000-8000-0000000000a1",
            "0192a000-0000-4000-8000-0000000000a1",
        ),
        INTENT.replace("0190f2a1-3b4c-7d5e", "0190F2A1-3b4c-7d5e"),
        INTENT.replace("0190f2a1-3b4c-7d5e", "0190f2a1-3b4c-4d5e"),
        INTENT.replace("OTERYN_PLATFORM_CHARACTER_AUTHORITY", "OTERYN_PLATFORM"),
        INTENT.replace(
            r#""audience":"OTERYN_GAME_CHARACTER_AUTHORITY""#,
            r#""audience":"X""#,
        ),
        INTENT.replace("}", r#","extra":1}"#),
        // Member order is part of the exact wire.
        INTENT.replace(
            r#""issued_at_source":"1790000000","expires_at_source":"1790000300""#,
            r#""expires_at_source":"1790000300","issued_at_source":"1790000000""#,
        ),
    ] {
        assert_eq!(classify(&invalid), Rejected(INTENT_INVALID), "{invalid}");
    }
    // Another operation's intent under this operation identity.
    assert_eq!(
        classify_intent(
            INTENT.as_bytes(),
            CREATE_COMMAND,
            "0192a000-0000-7000-8000-000000000002"
        ),
        Rejected(INTENT_INVALID)
    );
}

#[test]
fn receipts_match_the_contract_fixtures() {
    assert_eq!(
        String::from_utf8(committed_create_receipt(
            AUTHORITY,
            OP,
            Committed {
                character_id: bytes("01934f10-7c04-7001-805b-3b1122334401"),
                account_id: bytes("0190f2a1-3b4c-7d5e-8f60-718293a4b5c6"),
                world_id: bytes("01934f10-7c02-7001-805b-3b1122334401"),
                name: "Aldric",
                projection_epoch: 1,
                projection_revision: 43,
            },
            3,
            1_790_000_002,
        ))
        .unwrap(),
        r#"{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"CreateCharacter","operation_id":"0192a000-0000-7000-8000-000000000001","outcome":"COMMITTED","result_code":"CHAR_CMD_OK","character_id":"01934f10-7c04-7001-805b-3b1122334401","account_id":"0190f2a1-3b4c-7d5e-8f60-718293a4b5c6","world_id":"01934f10-7c02-7001-805b-3b1122334401","name":"Aldric","character_revision":"1","projection_epoch":"1","projection_revision":"43","recovery_generation":"3","decided_at":"1790000002"}"#
    );
    assert_eq!(
        String::from_utf8(rejected_receipt(
            AUTHORITY,
            CREATE_COMMAND,
            OP,
            NAME_UNAVAILABLE,
            3,
            1_790_000_002
        ))
        .unwrap(),
        r#"{"contract_version":1,"operation":"PublishCharacterCommandReceiptV1","source_authority":"oteryn:character-authority:primary","command":"CreateCharacter","operation_id":"0192a000-0000-7000-8000-000000000001","outcome":"REJECTED","result_code":"CHAR_CMD_NAME_UNAVAILABLE","recovery_generation":"3","decided_at":"1790000002"}"#
    );
}

#[test]
fn worst_case_receipts_fit_the_bound() {
    let authority = "a".repeat(128);
    let committed = committed_create_receipt(
        &authority,
        OP,
        Committed {
            character_id: bytes(OP),
            account_id: bytes(OP),
            world_id: bytes(OP),
            name: &"A".repeat(29),
            projection_epoch: u64::MAX,
            projection_revision: u64::MAX,
        },
        u64::MAX,
        u64::MAX,
    );
    assert!(committed.len() <= MAX_RECEIPT_BYTES);
    let rejected = rejected_receipt(
        &authority,
        &"C".repeat(64),
        OP,
        "CHAR_CMD_ACCOUNT_CHARACTER_LIMIT",
        u64::MAX,
        u64::MAX,
    );
    assert!(rejected.len() <= MAX_RECEIPT_BYTES);
}

#[test]
fn bindings_separate_what_was_decided() {
    let intent = intent_binding(INTENT.as_bytes());
    assert_eq!(intent.len(), 33);
    assert_eq!(intent[0], 0x03);
    assert_ne!(
        intent,
        intent_binding(INTENT.replace("Aldric", "Aldrik").as_bytes())
    );
    assert_ne!(
        read_conflict_binding(CREATE_COMMAND),
        unsupported_binding(CREATE_COMMAND)
    );
    assert_eq!(unsupported_binding("X"), [0x05, b'X']);
}
