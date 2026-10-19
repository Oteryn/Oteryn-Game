use super::*;

const HOLE_TEXT: &str = "I should not climb down again but rather venture forth.";

#[test]
fn golden_snapshot_hint_and_status_text_vectors() -> WireResult<()> {
    assert_eq!(encode_tutorial_notices_snapshot(), [0x08, 0x01]);
    decode_tutorial_notices_snapshot(&[0x08, 0x01])?;
    assert_eq!(
        encode_tutorial_notice(&TutorialNotice::Hint(2))?,
        [0x08, 0x02]
    );
    assert_eq!(
        encode_tutorial_notice(&TutorialNotice::Hint(u8::MAX))?,
        [0x08, 0xff, 0x01]
    );
    let text = encode_tutorial_notice(&TutorialNotice::StatusText(HOLE_TEXT.to_owned()))?;
    assert_eq!(
        text,
        [&[0x12, 55][..], HOLE_TEXT.as_bytes()].concat(),
        "field 2, length 55, the text bytes"
    );
    for notice in [
        TutorialNotice::Hint(1),
        TutorialNotice::Hint(2),
        TutorialNotice::Hint(u8::MAX),
        TutorialNotice::StatusText(HOLE_TEXT.to_owned()),
        TutorialNotice::StatusText("ä".repeat(127)),
    ] {
        let bytes = encode_tutorial_notice(&notice)?;
        assert!(bytes.len() <= MAX_TUTORIAL_NOTICE_BYTES);
        assert_eq!(decode_tutorial_notice(&bytes)?, notice);
    }
    Ok(())
}

#[test]
fn the_longest_text_fits_and_one_byte_more_is_refused_both_ways() -> WireResult<()> {
    let longest = "a".repeat(MAX_TUTORIAL_STATUS_TEXT_BYTES);
    let bytes = encode_tutorial_notice(&TutorialNotice::StatusText(longest.clone()))?;
    assert_eq!(bytes.len(), 258);
    assert_eq!(&bytes[..3], [0x12, 0xff, 0x01]);
    assert_eq!(
        decode_tutorial_notice(&bytes)?,
        TutorialNotice::StatusText(longest)
    );
    let over = "a".repeat(MAX_TUTORIAL_STATUS_TEXT_BYTES + 1);
    assert_eq!(
        encode_tutorial_notice(&TutorialNotice::StatusText(over.clone())),
        Err(TutorialNoticesWireError::LimitExceeded)
    );
    let wire = [&[0x12, 0x80, 0x02][..], over.as_bytes()].concat();
    assert_eq!(
        decode_tutorial_notice(&wire),
        Err(TutorialNoticesWireError::LimitExceeded)
    );
    let oversized = [&[0x12, 0x82, 0x02][..], &[b'a'; 258][..]].concat();
    assert_eq!(oversized.len(), 261);
    assert_eq!(
        decode_tutorial_notice(&oversized),
        Err(TutorialNoticesWireError::LimitExceeded)
    );
    assert_eq!(
        decode_tutorial_notices_snapshot(&[0; 9]),
        Err(TutorialNoticesWireError::LimitExceeded)
    );
    Ok(())
}

#[test]
fn zero_or_wide_hints_and_bad_text_are_refused_both_ways() {
    assert_eq!(
        encode_tutorial_notice(&TutorialNotice::Hint(0)),
        Err(TutorialNoticesWireError::Malformed)
    );
    for text in ["", "line\nbreak", "tab\there", "\u{7f}", "\u{85}"] {
        assert_eq!(
            encode_tutorial_notice(&TutorialNotice::StatusText(text.to_owned())),
            Err(TutorialNoticesWireError::Malformed),
            "{text:?}"
        );
        let wire = [&[0x12, text.len() as u8][..], text.as_bytes()].concat();
        assert_eq!(
            decode_tutorial_notice(&wire),
            Err(TutorialNoticesWireError::Malformed),
            "{text:?}"
        );
    }
    for wire in [
        vec![0x08, 0x00],
        vec![0x08, 0x80, 0x02],
        [vec![0x08], vec![0xff; 9], vec![0x01]].concat(),
        vec![0x12, 0x02, 0xc3, 0x28],
    ] {
        assert_eq!(
            decode_tutorial_notice(&wire),
            Err(TutorialNoticesWireError::Malformed),
            "{wire:?}"
        );
    }
}

#[test]
fn both_neither_repeated_unknown_and_truncated_fields_are_refused() {
    for wire in [
        vec![],
        vec![0x08, 0x02, 0x12, 0x01, b'a'],
        vec![0x12, 0x01, b'a', 0x08, 0x02],
        vec![0x08, 0x02, 0x08, 0x02],
        vec![0x12, 0x01, b'a', 0x12, 0x01, b'a'],
        vec![0x18, 0x01],
        vec![0x08, 0x02, 0x18, 0x01],
        vec![0x0a, 0x01, 0x02],
        vec![0x10, 0x01],
        vec![0x08],
        vec![0x12, 0x02, b'a'],
    ] {
        assert_eq!(
            decode_tutorial_notice(&wire),
            Err(TutorialNoticesWireError::Malformed),
            "{wire:?}"
        );
    }
    for wire in [
        vec![],
        vec![0x08, 0x00],
        vec![0x08, 0x02],
        vec![0x08, 0x01, 0x08, 0x01],
        vec![0x08, 0x01, 0x10, 0x01],
        vec![0x08],
    ] {
        assert_eq!(
            decode_tutorial_notices_snapshot(&wire),
            Err(TutorialNoticesWireError::Malformed),
            "{wire:?}"
        );
    }
}

#[test]
#[allow(clippy::expect_used)]
fn registry_binds_the_tutorial_notice_wire_ids_and_limits() {
    const REGISTRY: &str = include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
    const SCHEMA: &str =
        include_str!("../../../docs/contracts/protocol-oteryn/v1/tutorial_notices_v1.proto");
    let protocol: serde_json::Value = serde_json::from_str(REGISTRY).expect("registry");
    let only = |section: &str, id: u32| {
        let matches: Vec<&serde_json::Value> = protocol[section]
            .as_array()
            .expect(section)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matches.len(), 1, "{section} {id} registered once");
        matches[0].clone()
    };
    assert!(crate::REGISTERED_CAPABILITY_IDS_V1.contains(&CAPABILITY_TUTORIAL_NOTICES_V1));
    let capability = only("capabilities", CAPABILITY_TUTORIAL_NOTICES_V1);
    assert_eq!(capability["name"], "TUTORIAL_NOTICES_V1");
    assert_eq!(capability["offered"], true);
    assert_eq!(capability["requires"], serde_json::json!([]));
    assert_eq!(capability["command_types"], serde_json::json!([]));
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_TUTORIAL_NOTICES])
    );
    assert_eq!(
        protocol["allocation_ranges"]["capability_id"]["next_unallocated"],
        CAPABILITY_TUTORIAL_NOTICES_V1 + 1
    );
    let domain = only("state_domains", STATE_DOMAIN_TUTORIAL_NOTICES);
    assert_eq!(domain["name"], "TUTORIAL_NOTICES");
    assert_eq!(domain["capability"], CAPABILITY_TUTORIAL_NOTICES_V1);
    for (section, id, bound, message) in [
        (
            "delta_types",
            DELTA_TYPE_TUTORIAL_NOTICE_V1,
            MAX_TUTORIAL_NOTICE_BYTES,
            "TutorialNoticeV1",
        ),
        (
            "snapshot_types",
            SNAPSHOT_TYPE_TUTORIAL_NOTICES_V1,
            MAX_TUTORIAL_NOTICES_SNAPSHOT_BYTES,
            "TutorialNoticesSnapshotV1",
        ),
    ] {
        let entries = domain[section].as_array().expect(section);
        assert_eq!(entries.len(), 1, "{section}");
        assert_eq!(entries[0]["id"], id);
        assert_eq!(entries[0]["max_payload_bytes"], bound);
        assert_eq!(
            entries[0]["payload_schema"],
            format!("docs/contracts/protocol-oteryn/v1/tutorial_notices_v1.proto#{message}")
        );
        assert!(SCHEMA.contains(&format!("message {message} {{")));
    }
}
