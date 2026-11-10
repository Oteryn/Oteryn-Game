use super::*;

#[test]
fn window_actions_round_trip_and_donor_range_is_clamped() -> Result<(), Error> {
    for request in [
        Request::Open {
            boss_key: "oteryn:boss.fixture".into(),
        },
        Request::Cancel { context: [1; 16] },
        Request::Preview {
            context: [1; 16],
            difficulty: 0,
        },
        Request::Start {
            context: [1; 16],
            difficulty: 25,
        },
    ] {
        assert_eq!(decode_request(&encode_request(&request)?)?, request);
    }
    for difficulty in [29, u16::MAX] {
        for request in [
            Request::Start {
                context: [1; 16],
                difficulty,
            },
            Request::Preview {
                context: [1; 16],
                difficulty,
            },
        ] {
            let decoded = decode_request(&encode_request(&request)?)?;
            assert!(matches!(
                decoded,
                Request::Start { difficulty: 28, .. } | Request::Preview { difficulty: 28, .. }
            ));
        }
    }
    Ok(())
}

#[test]
fn malformed_and_action_incompatible_requests_fail_closed() -> Result<(), Error> {
    let start = encode_request(&Request::Start {
        context: [1; 16],
        difficulty: 3,
    })?;
    for end in 0..start.len() {
        assert!(decode_request(&start[..end]).is_err(), "prefix {end}");
    }
    for suffix in [&[8, 0][..], &[32, 4], &[40, 0], &[18, 0]] {
        let mut duplicate = start.clone();
        duplicate.extend_from_slice(suffix);
        assert!(decode_request(&duplicate).is_err());
    }
    for payload in [
        vec![],
        vec![8, 9],
        vec![8, 3],
        vec![8, 1, 32, 0],
        vec![0; MAX_REQUEST_BYTES + 1],
    ] {
        assert!(decode_request(&payload).is_err());
    }
    assert!(
        encode_request(&Request::Open {
            boss_key: "invalid".into()
        })
        .is_err()
    );
    assert!(encode_request(&Request::Cancel { context: [0; 16] }).is_err());
    let mut overflow = encode_request(&Request::Cancel { context: [1; 16] })?;
    overflow[1] = 0;
    push_varint_field(&mut overflow, 4, u64::from(u16::MAX) + 1);
    assert!(decode_request(&overflow).is_err());
    Ok(())
}

#[test]
fn window_close_and_source_absent_refusal_are_distinct() -> Result<(), Error> {
    let window = Reply::Window {
        context: [1; 16],
        boss_key: "oteryn:boss.fixture".into(),
        boss_creature_key: "oteryn:creature.fixture".into(),
        content_revision: "fixture-v1".into(),
        selected_difficulty: 25,
        personal_highest: 0,
        unlocked_max: 1,
        maximum_difficulty: 25,
    };
    assert!(encode_reply(&window)?.starts_with(&[8, 0]));
    assert_eq!(
        encode_reply(&Reply::Closed { context: [1; 16] })?[..2],
        [8, 1]
    );
    assert_eq!(
        encode_reply(&Reply::Refused {
            context: None,
            reason: Refusal::NoAcceptedProfile,
        })?,
        vec![8, 2, 56, 1]
    );
    assert!(
        encode_reply(&Reply::Window {
            context: [1; 16],
            boss_key: "oteryn:boss.fixture".into(),
            boss_creature_key: "oteryn:creature.fixture".into(),
            content_revision: "fixture-v1".into(),
            selected_difficulty: 26,
            personal_highest: 0,
            unlocked_max: 1,
            maximum_difficulty: 25,
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn individual_unlock_window_roundtrips_and_rejects_malformed_progress() -> Result<(), Error> {
    let window = Reply::Window {
        context: [8; 16],
        boss_key: "oteryn:boss.phosphorus".into(),
        boss_creature_key: "oteryn:creature.phosphorus".into(),
        content_revision: "r1".into(),
        selected_difficulty: 28,
        personal_highest: 27,
        unlocked_max: 28,
        maximum_difficulty: 28,
    };
    let bytes = encode_reply(&window)?;
    assert_eq!(decode_reply(&bytes)?, window);
    let mut legacy = window.clone();
    if let Reply::Window {
        personal_highest,
        unlocked_max,
        ..
    } = &mut legacy
    {
        *personal_highest = 0;
        *unlocked_max = 0;
    }
    let legacy_length = encode_reply(&legacy)?.len();
    for end in 0..bytes.len() {
        if end == legacy_length {
            assert!(matches!(
                decode_reply(&bytes[..end])?,
                Reply::Window {
                    personal_highest: 0,
                    unlocked_max: 0,
                    ..
                }
            ));
        } else {
            assert!(
                decode_reply(&bytes[..end]).is_err(),
                "truncated field at {end}"
            );
        }
    }
    let mut duplicate = bytes;
    duplicate.extend_from_slice(&[64, 27]);
    assert!(decode_reply(&duplicate).is_err());
    assert_eq!(boss_maximum("oteryn:boss.maior_domus"), Some(15));
    assert_eq!(boss_maximum("oteryn:boss.mimar_haffar"), Some(25));
    assert_eq!(boss_maximum("oteryn:boss.phosphorus"), Some(28));
    Ok(())
}

#[test]
fn legacy_window_bytes_and_open_action_remain_compatible() -> Result<(), Error> {
    let window = Reply::Window {
        context: [1; 16],
        boss_key: "oteryn:boss.old".into(),
        boss_creature_key: "oteryn:creature.old".into(),
        content_revision: "r1".into(),
        selected_difficulty: 0,
        personal_highest: 0,
        unlocked_max: 0,
        maximum_difficulty: 25,
    };
    let bytes = encode_reply(&window)?;
    let mut expected = Vec::new();
    push_varint_field(&mut expected, 1, 0);
    push_message_field(&mut expected, 2, &[1; 16]);
    push_message_field(&mut expected, 3, b"oteryn:boss.old");
    push_message_field(&mut expected, 4, b"r1");
    push_varint_field(&mut expected, 5, 0);
    push_varint_field(&mut expected, 6, 25);
    assert_eq!(bytes, expected);
    assert!(matches!(
        decode_reply(&bytes)?,
        Reply::Window {
            personal_highest: 0,
            unlocked_max: 0,
            ..
        }
    ));
    let request = Request::OpenWithProgress {
        boss_key: "oteryn:boss.phosphorus".into(),
    };
    assert_eq!(decode_request(&encode_request(&request)?)?, request);
    Ok(())
}
