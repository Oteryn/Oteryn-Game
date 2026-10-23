use super::*;

const OPERATION_ID: [u8; 16] = [
    0x01, 0x89, 0x0f, 0x4e, 0x7c, 0x00, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, 1,
];

#[path = "tests/character_sex_tests.rs"]
mod character_sex_tests;

fn wire(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value = serde_json::json!({
        "contract_version": 3,
        "variant": VARIANT,
        "issuer_authority": ISSUER_AUTHORITY,
        "issuer_decision_id": "3f0c5b7e-1d2a-4c3b-9a8f-0123456789ab",
        "source_revision": "7",
        "operation_id": "01890f4e-7c00-7000-8000-000000000001",
        "operation": OPERATION,
        "account_id": "01934f10-7c00-7000-8000-000000000001",
        "target_world_id": "01934f10-7c00-7000-8000-0000000000aa",
        "requested_name": "Sir Aldric",
        "requested_sex": "Male",
        "interpretation_context": {
            "profile_revision": "profile-1",
            "ruleset_revision": "ruleset-1",
            "content_revision": "content-1",
            "starter_template_revision": "starter-1"
        },
        "issued_at_source": "1000",
        "expires_at_source": "1120",
        "audience": AUDIENCE
    });
    edit(&mut value);
    serde_json::to_vec(&value).unwrap_or_default()
}

#[test]
fn exact_producer_wire_decodes_and_binds_every_field() -> Result<(), InvalidIntent> {
    let intent = decode_producer_response(&wire(|_| {}), OPERATION_ID)?;
    assert_eq!(intent.source_revision(), 7);
    assert_eq!(intent.operation_id(), OPERATION_ID);
    assert_eq!(intent.expires_at_source() - intent.issued_at_source(), 120);
    assert_eq!(intent.requested_name().as_str(), "Sir Aldric");
    assert_eq!(
        intent.binding().len(),
        1 + 16 * 4 + 8 * 3 + 4 * 11 + 2 + 10 + 1
    );
    let renamed = decode_producer_response(
        &wire(|v| v["requested_name"] = "Aldric".into()),
        OPERATION_ID,
    )?;
    assert_ne!(renamed.binding(), intent.binding());
    let changed = decode_producer_response(
        &wire(|v| v["interpretation_context"]["content_revision"] = "content-2".into()),
        OPERATION_ID,
    )?;
    assert_ne!(changed.binding(), intent.binding());
    assert!(!format!("{intent:?}").contains("01934f10"));
    assert!(!format!("{intent:?}").contains("Aldric"));
    Ok(())
}

#[test]
fn every_non_contract_wire_is_rejected() {
    let rejected: Vec<Vec<u8>> = vec![
        wire(|v| v["variant"] = "PLATFORM_USER_CREATE".into()),
        wire(|v| v["contract_version"] = 1.into()),
        wire(|v| v["contract_version"] = 4.into()),
        wire(|v| v["requested_name"] = "A".into()),
        wire(|v| v["requested_name"] = "Aldric2".into()),
        wire(|v| v["requested_name"] = " Aldric".into()),
        wire(|v| v["requested_name"] = "Al\"dric".into()),
        wire(|v| v["requested_name"] = "a".repeat(30).into()),
        wire(|v| v["requested_name"] = serde_json::Value::Null),
        wire(|v| {
            if let Some(map) = v.as_object_mut() {
                map.remove("requested_name");
            }
        }),
        wire(|v| v["issuer_authority"] = "OTHER".into()),
        wire(|v| v["operation"] = "RENAME".into()),
        wire(|v| v["audience"] = "OTHER".into()),
        wire(|v| v["source_revision"] = "0".into()),
        wire(|v| v["source_revision"] = "07".into()),
        wire(|v| v["source_revision"] = "9223372036854775808".into()),
        wire(|v| v["source_revision"] = 7.into()),
        wire(|v| v["operation_id"] = "01890f4e-7c00-7000-8000-000000000002".into()),
        wire(|v| v["account_id"] = "01934F10-7c00-7000-8000-000000000001".into()),
        wire(|v| v["account_id"] = "01934f10-7c00-4000-8000-000000000001".into()),
        wire(|v| v["target_world_id"] = "not-a-uuid".into()),
        wire(|v| v["interpretation_context"]["profile_revision"] = "-leading".into()),
        wire(|v| v["interpretation_context"]["profile_revision"] = "x".repeat(129).into()),
        wire(|v| v["expires_at_source"] = "1000".into()),
        wire(|v| v["expires_at_source"] = "1301".into()),
        wire(|v| v["extra"] = "x".into()),
        wire(|v| v["interpretation_context"]["extra"] = "x".into()),
        wire(|v| {
            if let Some(map) = v.as_object_mut() {
                map.remove("audience");
            }
        }),
        br#"{"contract_version":1,"contract_version":1}"#.to_vec(),
        vec![b' '; MAX_RESPONSE_BYTES + 1],
        Vec::new(),
    ];
    for raw in rejected {
        assert_eq!(
            decode_producer_response(&raw, OPERATION_ID),
            Err(InvalidIntent)
        );
    }
    let mut duplicated = wire(|_| {});
    duplicated.pop();
    duplicated.extend_from_slice(br#","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}"#);
    assert_eq!(
        decode_producer_response(&duplicated, OPERATION_ID),
        Err(InvalidIntent)
    );
}

#[test]
fn read_request_is_the_exact_bounded_producer_request() {
    let request = encode_read_request(OPERATION_ID);
    assert_eq!(
        request,
        r#"{"contract_version":3,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}"#
    );
    assert!(request.len() <= 256);
}
