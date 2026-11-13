use super::*;
const ROOT: &str = "../../../docs/contracts/fixtures/character-services-v2/";
fn fixture(name: &str) -> Vec<u8> {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(ROOT)
            .join(format!("{name}.json")),
    )
    .unwrap_or_default();
    assert!(!bytes.is_empty(), "missing or empty golden {name}");
    bytes
}
fn kind(name: &str) -> Kind {
    match name {
        "schedule-receipt" | "rejected" | "invalid-receipt" => Kind::Receipt,
        "state" => Kind::State,
        "watermark" => Kind::Watermark,
        "invalid-list" => Kind::List,
        _ => Kind::Intent,
    }
}
#[test]
fn character_services_v2_golden_bytes_and_closed_schema() {
    for name in [
        "schedule",
        "schedule-receipt",
        "cancel",
        "rename",
        "sex",
        "world",
        "rejected",
        "state",
        "watermark",
    ] {
        let bytes = fixture(name);
        assert!(!bytes.is_empty(), "missing golden {name}");
        let doc = decode(kind(name), &bytes).unwrap_or_else(|_| Object(Vec::new()));
        assert_eq!(
            serde_json::to_vec(&doc).ok().as_deref(),
            Some(bytes.as_slice()),
            "golden {name}"
        );
        let mut extra = bytes.clone();
        extra.pop();
        extra.extend_from_slice(b",\"extra\":1}");
        assert!(decode(kind(name), &extra).is_err());
        let mut missing: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        if let Some(o) = missing.as_object_mut() {
            o.remove("contract_version");
        }
        assert!(
            decode(
                kind(name),
                &serde_json::to_vec(&missing).unwrap_or_default()
            )
            .is_err()
        );
        let object: Object = serde_json::from_slice(&bytes).unwrap_or_else(|_| Object(Vec::new()));
        let mut reordered = object.clone();
        reordered.0.swap(0, 1);
        assert!(
            decode(
                kind(name),
                &serde_json::to_vec(&reordered).unwrap_or_default()
            )
            .is_err()
        );
        let mut newline = bytes.clone();
        newline.push(b'\n');
        assert!(decode(kind(name), &newline).is_err());
        assert!(decode(kind(name), &vec![b' '; kind(name).bound() + 1]).is_err());
    }
    for name in ["invalid-list", "invalid-receipt", "invalid-intent"] {
        assert!(decode(kind(name), &fixture(name)).is_err(), "{name}");
    }
}
#[test]
fn character_services_v2_types_ttl_unicode_and_bounds() {
    let original = String::from_utf8(fixture("rename")).unwrap_or_default();
    for (from, to) in [
        ("0192a000-0000-7000", "0192a000-0000-4000"),
        ("\"19\"", "19"),
        ("\"19\"", "\"0\""),
        ("\"19\"", "\"01\""),
        ("\"19\"", "\"18446744073709551616\""),
        ("1790000500", "1790000501"),
        ("Aldric Vale", " Aldric"),
        ("Aldric Vale", "Aldric  Vale"),
    ] {
        assert!(
            CharacterCommandIntentV2::decode(original.replace(from, to).as_bytes()).is_err(),
            "{to}"
        );
    }
    let state = String::from_utf8(fixture("state")).unwrap_or_default();
    assert!(
        PublishCharacterServiceStateV2::decode(state.replace("Aldric", "Élan").as_bytes()).is_ok()
    );
    for bad in ["E\u{301}lan", "A\u{200d}B", "A\u{202e}B"] {
        assert!(
            PublishCharacterServiceStateV2::decode(state.replace("Aldric", bad).as_bytes())
                .is_err()
        );
    }
    assert!(
        PublishCharacterServiceStateV2::decode(
            state.replace("1795270402", "1790000002").as_bytes()
        )
        .is_err()
    );
    for bad in ["0", "3", "null"] {
        assert!(
            CharacterCommandIntentV2::decode(
                String::from_utf8(fixture("sex"))
                    .unwrap_or_default()
                    .replace("\"requested_sex\":1", &format!("\"requested_sex\":{bad}"))
                    .as_bytes()
            )
            .is_err()
        );
    }
}
#[test]
fn character_services_v2_pending_and_request_roundtrips() {
    let bytes = br#"{"contract_version":2,"pending":[]}"#;
    assert_eq!(
        PendingCharacterCommandsV2::decode(bytes)
            .and_then(|v| v.encode())
            .ok()
            .as_deref(),
        Some(bytes.as_slice())
    );
    let list=br#"{"contract_version":2,"operation":"ListPendingCharacterCommandsV2","source_authority":"game:authority","after_source_revision":"0","after_operation_id":"00000000-0000-0000-0000-000000000000","max_entries":32}"#;
    assert_eq!(
        ListPendingCharacterCommandsV2::decode(list)
            .and_then(|v| v.encode())
            .ok()
            .as_deref(),
        Some(list.as_slice())
    );
    let read=br#"{"contract_version":2,"operation":"ReadCharacterCommandIntentV2","source_authority":"game:authority","issuer_authority":"OTERYN_PLATFORM_CHARACTER_AUTHORITY","command":"RenameCharacter","operation_id":"0192a000-0000-7000-8000-000000000003"}"#;
    assert_eq!(
        ReadCharacterCommandIntentV2::decode(read)
            .and_then(|v| v.encode())
            .ok()
            .as_deref(),
        Some(read.as_slice())
    );
    assert!(
        ReceiptAcknowledgementV2::decode(br#"{"contract_version":2,"result":"superseded"}"#)
            .is_err()
    );
    assert!(AcknowledgementV2::decode(br#"{"contract_version":2,"result":"superseded"}"#).is_ok());
    let pending=br#"{"contract_version":2,"pending":[{"command":"RenameCharacter","operation_id":"0192a000-0000-7000-8000-000000000003","source_revision":"19"}]}"#;
    assert!(PendingCharacterCommandsV2::decode(pending).is_ok());
    assert!(PendingCharacterCommandsV2::decode(String::from_utf8_lossy(pending).replace("\"command\":\"RenameCharacter\",\"operation_id\":\"0192a000-0000-7000-8000-000000000003\"","\"operation_id\":\"0192a000-0000-7000-8000-000000000003\",\"command\":\"RenameCharacter\"").as_bytes()).is_err());
    let intent = CharacterCommandIntentV2::decode(&fixture("schedule")).ok();
    assert!(
        intent
            .as_ref()
            .is_some_and(|i| i.valid_at(Some(1790000000)))
    );
    assert!(intent.as_ref().is_some_and(|i| !i.valid_at(None)
        && !i.valid_at(Some(1790000300))
        && !i.valid_at(Some(1789999994))));
}

#[test]
fn character_services_v2_duplicates_limits_and_command_specific_codes() {
    let original = String::from_utf8(fixture("schedule")).unwrap_or_default();
    assert!(
        CharacterCommandIntentV2::decode(
            original
                .replace(
                    "\"contract_version\":2",
                    "\"contract_version\":2,\"contract_version\":2"
                )
                .as_bytes()
        )
        .is_err()
    );
    for id in [
        "00000000-0000-0000-0000-000000000000",
        "0192a000-0000-7000-c000-000000000001",
        "0192A000-0000-7000-8000-000000000001",
    ] {
        assert!(
            CharacterCommandIntentV2::decode(
                original
                    .replace("0192a000-0000-7000-8000-000000000001", id)
                    .as_bytes()
            )
            .is_err()
        );
    }
    let rejected = String::from_utf8(fixture("rejected")).unwrap_or_default();
    assert!(
        PublishCharacterCommandReceiptV2::decode(
            rejected
                .replace("RenameCharacter", "ScheduleCharacterDeletion")
                .as_bytes()
        )
        .is_err()
    );
    let state = String::from_utf8(fixture("state")).unwrap_or_default();
    assert!(
        PublishCharacterServiceStateV2::decode(
            state.replace("1795270402", "1790000003").as_bytes()
        )
        .is_ok(),
        "deadline length is Game policy, not wire"
    );
    let pending=br#"{"contract_version":2,"pending":[{"command":"RenameCharacter","operation_id":"0192a000-0000-7000-8000-000000000003","source_revision":"19"},{"command":"RenameCharacter","operation_id":"0192a000-0000-7000-8000-000000000003","source_revision":"19"}]}"#;
    assert!(PendingCharacterCommandsV2::decode(pending).is_err());
    for (kind, bound) in [
        (Kind::List, 512),
        (Kind::Read, 512),
        (Kind::Intent, 4096),
        (Kind::Pending, 8192),
        (Kind::Receipt, 1024),
        (Kind::State, 2048),
        (Kind::Watermark, 512),
        (Kind::Ack, 256),
    ] {
        assert_eq!(kind.bound(), bound);
        assert!(decode(kind, &vec![b' '; bound + 1]).is_err());
    }
}
