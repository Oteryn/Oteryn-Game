#![allow(clippy::expect_used, reason = "bounded protocol test fixtures")]
use super::*;

#[test]
fn boss_reward_requests_are_exact_bounded_and_round_trip() {
    for after_ordinal in [0, 1, 1_000] {
        let value = BrowseRequest {
            death: [9; 16],
            after_ordinal,
        };
        let bytes = encode_browse_request(&value).expect("valid browse");
        assert_eq!(decode_browse_request(&bytes), Ok(value));
        for len in 0..bytes.len() {
            assert!(decode_browse_request(&bytes[..len]).is_err());
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(decode_browse_request(&extra).is_err());
        let mut wrong_version = bytes;
        wrong_version[0] = 2;
        assert!(decode_browse_request(&wrong_version).is_err());
    }
    for ordinal in [1, 1_000] {
        let value = TakeRequest {
            death: [7; 16],
            ordinal,
        };
        assert_eq!(
            decode_take_request(&encode_take_request(&value).expect("valid take")),
            Ok(value)
        );
    }
    for ordinal in [0, 1_001, u16::MAX] {
        assert!(
            encode_take_request(&TakeRequest {
                death: [7; 16],
                ordinal
            })
            .is_err()
        );
        let mut bytes = vec![1];
        bytes.extend([7; 16]);
        bytes.extend(ordinal.to_le_bytes());
        assert!(decode_take_request(&bytes).is_err());
    }
    assert!(
        encode_browse_request(&BrowseRequest {
            death: [0; 16],
            after_ordinal: 0
        })
        .is_err()
    );
    assert!(decode_browse_request(&[1; 18]).is_err());
    for death in [None, Some([8; 16])] {
        let value = CollectAllRequest { death };
        let bytes = encode_collect_all_request(&value).expect("valid collect");
        assert_eq!(decode_collect_all_request(&bytes), Ok(value));
        for len in 0..bytes.len() {
            assert!(decode_collect_all_request(&bytes[..len]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(decode_collect_all_request(&extra).is_err());
    }
    assert!(
        encode_collect_all_request(&CollectAllRequest {
            death: Some([0; 16])
        })
        .is_err()
    );
}

#[test]
fn boss_reward_rows_are_ordered_and_bounded_and_refusals_are_closed() {
    let rows: Vec<_> = (1..=64)
        .map(|ordinal| RewardRow {
            ordinal,
            definition_reference: 100,
            quantity: 100,
        })
        .collect();
    let value = BrowseReply {
        status: BossRewardStatus::Ok,
        death: [4; 16],
        expires_at_unix_ms: 900,
        rows,
        more: true,
    };
    let bytes = encode_browse_reply(&value).expect("valid page");
    assert!(bytes.len() <= MAX_RESULT_BYTES);
    assert_eq!(decode_browse_reply(&bytes), Ok(value.clone()));
    for len in 0..bytes.len() {
        assert!(decode_browse_reply(&bytes[..len]).is_err());
    }
    let mut bad = value.clone();
    bad.rows.push(RewardRow {
        ordinal: 65,
        definition_reference: 100,
        quantity: 1,
    });
    assert!(encode_browse_reply(&bad).is_err());
    for row in [
        RewardRow {
            ordinal: 0,
            definition_reference: 1,
            quantity: 1,
        },
        RewardRow {
            ordinal: 1_001,
            definition_reference: 1,
            quantity: 1,
        },
        RewardRow {
            ordinal: 1,
            definition_reference: 0,
            quantity: 1,
        },
        RewardRow {
            ordinal: 1,
            definition_reference: 1,
            quantity: 0,
        },
    ] {
        bad.rows = vec![row];
        assert!(encode_browse_reply(&bad).is_err());
    }
    bad.rows = vec![value.rows[0], value.rows[0]];
    assert!(encode_browse_reply(&bad).is_err());
    let mut invalid_more = bytes.clone();
    *invalid_more.last_mut().expect("has more") = 2;
    assert!(decode_browse_reply(&invalid_more).is_err());
    let mut unknown_status = bytes;
    unknown_status[1] = 9;
    assert!(decode_browse_reply(&unknown_status).is_err());
    for raw in 0..=8 {
        let status = BossRewardStatus::try_from(raw).expect("registered status");
        let reply = CollectReply {
            status,
            moved: 3,
            remaining: 7,
        };
        assert_eq!(
            decode_collect_reply(&encode_collect_reply(&reply).expect("reply")),
            Ok(reply)
        );
    }
    for raw in 9..=u8::MAX {
        assert!(BossRewardStatus::try_from(raw).is_err());
    }
    assert!(decode_collect_reply(&[1, 0, 0, 0, 0, 0, 0]).is_err());
    assert!(decode_collect_reply(&[1, 0, 0, 0, 0, 0, 0, 0, 0]).is_err());
}
