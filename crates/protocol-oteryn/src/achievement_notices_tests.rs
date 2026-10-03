#![allow(clippy::expect_used)]

use serde_json::Value;

use super::*;

const PROTOCOL_REGISTRY: &str =
    include_str!("../../../docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json");
const SCHEMA_PATH: &str = "docs/contracts/protocol-oteryn/v1/achievement_notices_v1.proto";
const SCHEMA: &str =
    include_str!("../../../docs/contracts/protocol-oteryn/v1/achievement_notices_v1.proto");

const PREFIX: &str = "oteryn:achievement/";

fn earned(key: &str, name: &str, fact_count: u32, total_points: u32) -> AchievementEarned {
    AchievementEarned {
        key: key.to_owned(),
        name: name.to_owned(),
        watermark: AchievementWatermark {
            fact_count,
            total_points,
        },
    }
}

fn longest() -> AchievementEarned {
    earned(
        &format!(
            "{PREFIX}{}",
            "a".repeat(MAX_ACHIEVEMENT_KEY_BYTES - PREFIX.len())
        ),
        &"n".repeat(MAX_ACHIEVEMENT_NAME_BYTES),
        u32::MAX,
        u32::MAX,
    )
}

#[test]
fn the_earned_delta_round_trips_within_its_bound() {
    let notice = earned("oteryn:achievement/allow_cookies", "Allow Cookies?!", 3, 7);
    let bytes = encode_achievement_earned(&notice).expect("encode");
    assert_eq!(decode_achievement_earned(&bytes).expect("decode"), notice);
    assert_eq!(&bytes[..2], [0x0a, 32]);
    assert_eq!(&bytes[bytes.len() - 4..], [0x18, 3, 0x20, 7]);

    let notice = longest();
    let bytes = encode_achievement_earned(&notice).expect("encode");
    assert_eq!(bytes.len(), 241);
    assert!(bytes.len() <= MAX_ACHIEVEMENT_EARNED_BYTES);
    assert_eq!(decode_achievement_earned(&bytes).expect("decode"), notice);

    // `total_points` 0 is omitted and decodes back to 0.
    let zero = earned("oteryn:achievement/a", "A", 1, 0);
    let bytes = encode_achievement_earned(&zero).expect("encode");
    assert_eq!(&bytes[bytes.len() - 2..], [0x18, 1]);
    assert_eq!(decode_achievement_earned(&bytes).expect("decode"), zero);
}

#[test]
fn the_snapshot_round_trips_within_its_bound() {
    for watermark in [
        AchievementWatermark::default(),
        AchievementWatermark {
            fact_count: 1,
            total_points: 0,
        },
        AchievementWatermark {
            fact_count: u32::MAX,
            total_points: u32::MAX,
        },
    ] {
        let bytes = encode_achievement_notices_snapshot(watermark);
        assert!(bytes.len() <= 12);
        assert_eq!(
            decode_achievement_notices_snapshot(&bytes).expect("decode"),
            watermark
        );
    }
    assert!(encode_achievement_notices_snapshot(AchievementWatermark::default()).is_empty());
    assert_eq!(
        encode_achievement_notices_snapshot(AchievementWatermark {
            fact_count: u32::MAX,
            total_points: u32::MAX,
        })
        .len(),
        12
    );
}

#[test]
fn earned_deltas_outside_the_bounds_are_refused_in_both_directions() {
    let malformed = AchievementNoticesWireError::Malformed;
    let limit = AchievementNoticesWireError::LimitExceeded;
    let mut long_key = longest();
    long_key.key.push('a');
    let mut long_name = longest();
    long_name.name.push('n');
    for (notice, error) in [
        (long_key, limit),
        (long_name, limit),
        (earned("oteryn:achievement/a", "", 1, 0), malformed),
        (earned("oteryn:achievement/a", "A", 0, 0), malformed),
        (earned("oteryn:achievement/A", "A", 1, 0), malformed),
        (earned("oteryn:achievement/a__b", "A", 1, 0), malformed),
        (earned("oteryn:item/a", "A", 1, 0), malformed),
        (earned("", "A", 1, 0), malformed),
    ] {
        assert_eq!(encode_achievement_earned(&notice), Err(error), "{notice:?}");
        // The same value built by hand is refused by the decoder.
        let mut bytes = Vec::new();
        crate::charm_wire::push_message_field(&mut bytes, 1, notice.key.as_bytes());
        crate::charm_wire::push_message_field(&mut bytes, 2, notice.name.as_bytes());
        crate::charm_wire::push_nonzero_varint_field(
            &mut bytes,
            3,
            u64::from(notice.watermark.fact_count),
        );
        assert_eq!(decode_achievement_earned(&bytes), Err(error), "{notice:?}");
    }
}

#[test]
fn malformed_earned_deltas_fail_closed() {
    let good =
        encode_achievement_earned(&earned("oteryn:achievement/a", "A", 1, 2)).expect("encode");
    let malformed = Err(AchievementNoticesWireError::Malformed);
    // Unknown field 5, a repeated key, a key sent as a varint, a truncated tail, and invalid UTF-8.
    for bytes in [
        [good.as_slice(), &[0x28, 1]].concat(),
        [good.as_slice(), &[0x0a, 1, b'a']].concat(),
        [&[0x08, 1], good.as_slice()].concat(),
        good[..good.len() - 1].to_vec(),
        [
            &[0x0a, 0x14],
            PREFIX.as_bytes(),
            &[0xff, 0x12, 1, b'A', 0x18, 1],
        ]
        .concat(),
        [&good[..], &[0x18, 0x80, 0x80, 0x80, 0x80, 0x10]].concat(),
    ] {
        assert_eq!(decode_achievement_earned(&bytes), malformed, "{bytes:?}");
    }
    assert_eq!(decode_achievement_earned(&[]), malformed);
    assert_eq!(
        decode_achievement_earned(&[0; MAX_ACHIEVEMENT_EARNED_BYTES + 1]),
        Err(AchievementNoticesWireError::LimitExceeded)
    );
}

#[test]
fn malformed_snapshots_fail_closed() {
    let malformed = Err(AchievementNoticesWireError::Malformed);
    // Unknown field 3, a repeated field, a length-delimited field, a uint32 overflow, truncation.
    for bytes in [
        &[0x18, 1][..],
        &[0x08, 1, 0x08, 2][..],
        &[0x0a, 1, 0][..],
        &[0x08, 0x80, 0x80, 0x80, 0x80, 0x10][..],
        &[0x08][..],
    ] {
        assert_eq!(
            decode_achievement_notices_snapshot(bytes),
            malformed,
            "{bytes:?}"
        );
    }
    assert_eq!(
        decode_achievement_notices_snapshot(&[0; MAX_ACHIEVEMENT_NOTICES_SNAPSHOT_BYTES + 1]),
        Err(AchievementNoticesWireError::LimitExceeded)
    );
}

#[test]
fn registry_binds_the_notice_wire_ids_and_limits() {
    let protocol: Value = serde_json::from_str(PROTOCOL_REGISTRY).expect("protocol registry");
    let only = |section: &str, id: u32| {
        let matches: Vec<&Value> = protocol[section]
            .as_array()
            .expect(section)
            .iter()
            .filter(|entry| entry["id"] == id)
            .collect();
        assert_eq!(matches.len(), 1, "{section} {id} registered once");
        matches[0].clone()
    };

    let capability = only("capabilities", CAPABILITY_ACHIEVEMENT_NOTICES_V1);
    assert_eq!(capability["name"], "ACHIEVEMENT_NOTICES_V1");
    assert_eq!(capability["offered"], false);
    assert_eq!(capability["command_types"], serde_json::json!([]));
    assert_eq!(
        capability["state_domains"],
        serde_json::json!([STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES])
    );
    assert!(
        protocol["command_types"]
            .as_array()
            .expect("command types")
            .iter()
            .all(|command| command["capability"] != CAPABILITY_ACHIEVEMENT_NOTICES_V1)
    );

    let domain = only("state_domains", STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES);
    assert_eq!(domain["name"], "ACCOUNT_ACHIEVEMENT_NOTICES");
    assert_eq!(domain["capability"], CAPABILITY_ACHIEVEMENT_NOTICES_V1);
    let types = |section: &str| -> Vec<(u64, String, String, u64)> {
        domain[section]
            .as_array()
            .expect(section)
            .iter()
            .map(|entry| {
                (
                    entry["id"].as_u64().expect("id"),
                    entry["name"].as_str().expect("name").to_owned(),
                    entry["payload_schema"].as_str().expect("schema").to_owned(),
                    entry["max_payload_bytes"].as_u64().expect("bound"),
                )
            })
            .collect()
    };
    let schema = |message: &str| format!("{SCHEMA_PATH}#{message}");
    assert_eq!(
        types("delta_types"),
        [(
            u64::from(DELTA_TYPE_ACHIEVEMENT_EARNED_V1),
            "ACHIEVEMENT_EARNED_DELTA_V1".to_owned(),
            schema("AchievementEarnedV1"),
            MAX_ACHIEVEMENT_EARNED_BYTES as u64
        )]
    );
    assert_eq!(
        types("snapshot_types"),
        [(
            u64::from(SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1),
            "ACHIEVEMENT_NOTICES_SNAPSHOT_V1".to_owned(),
            schema("AchievementNoticesSnapshotV1"),
            MAX_ACHIEVEMENT_NOTICES_SNAPSHOT_BYTES as u64
        )]
    );

    for message in ["AchievementNoticesSnapshotV1", "AchievementEarnedV1"] {
        assert!(
            SCHEMA.contains(&format!("message {message} {{")),
            "{message}"
        );
    }
    for bound in [
        MAX_ACHIEVEMENT_EARNED_BYTES,
        MAX_ACHIEVEMENT_NOTICES_SNAPSHOT_BYTES,
    ] {
        assert!(
            SCHEMA.contains(&format!("At most {bound} bytes")),
            "{bound}"
        );
    }
}
