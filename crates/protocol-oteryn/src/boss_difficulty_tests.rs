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
    for difficulty in [26, u16::MAX] {
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
                Request::Start { difficulty: 25, .. } | Request::Preview { difficulty: 25, .. }
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
        content_revision: "fixture-v1".into(),
        selected_difficulty: 25,
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
            content_revision: "fixture-v1".into(),
            selected_difficulty: 26,
        })
        .is_err()
    );
    Ok(())
}
