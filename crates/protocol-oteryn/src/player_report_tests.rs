#![allow(clippy::expect_used)]
use super::*;

fn bug() -> PlayerReportRequest {
    PlayerReportRequest {
        kind: KIND_BUG,
        category: BUG_CATEGORY_MAP,
        reason: 0,
        action: 0,
        target: String::new(),
        comment: "Hole in the wall".into(),
        map_x: 32_369,
        map_y: 32_241,
        map_floor: 7,
    }
}

fn violation() -> PlayerReportRequest {
    PlayerReportRequest {
        kind: KIND_RULE_VIOLATION,
        category: 0,
        reason: MAX_REPORT_REASON,
        action: MAX_REPORT_ACTION,
        target: "Alice Bob".into(),
        comment: "macro".into(),
        map_x: 0,
        map_y: 0,
        map_floor: 0,
    }
}

#[test]
fn player_report_request_codec_is_strict_and_bounded() {
    let largest = PlayerReportRequest {
        target: "A".repeat(MAX_TARGET_BYTES),
        comment: "ż".repeat(MAX_COMMENT_BYTES / 2),
        map_x: u16::MAX,
        map_y: u16::MAX,
        map_floor: u8::MAX,
        ..bug()
    };
    for r in [bug(), violation(), largest] {
        let bytes = encode_player_report_request(&r).expect("request");
        assert!(bytes.len() <= MAX_REQUEST_BYTES);
        assert_eq!(decode_player_report_request(&bytes), Ok(r));
        for end in 0..bytes.len() {
            assert!(decode_player_report_request(&bytes[..end]).is_err());
        }
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&[8, 1]);
        assert!(decode_player_report_request(&duplicate).is_err());
        let mut unknown = bytes;
        unknown.extend_from_slice(&[80, 1]);
        assert!(decode_player_report_request(&unknown).is_err());
    }
    let long = PlayerReportRequest {
        comment: "x".repeat(MAX_COMMENT_BYTES + 1),
        ..bug()
    };
    assert_eq!(
        encode_player_report_request(&long),
        Err(Error::LimitExceeded)
    );
    let name = PlayerReportRequest {
        target: "A".repeat(MAX_TARGET_BYTES + 1),
        ..violation()
    };
    assert_eq!(
        encode_player_report_request(&name),
        Err(Error::LimitExceeded)
    );
    assert!(decode_player_report_request(&[0; MAX_REQUEST_BYTES + 1]).is_err());
    let mut bad_utf8 = encode_player_report_request(&bug()).expect("request");
    let at = bad_utf8.iter().position(|b| *b == b'H').expect("comment");
    bad_utf8[at] = 0xff;
    assert_eq!(
        decode_player_report_request(&bad_utf8),
        Err(Error::Malformed)
    );
    // A floor beyond u8 and a reason beyond u8 are malformed, not truncated.
    let mut wide = encode_player_report_request(&bug()).expect("request");
    let floor = wide.len() - 2;
    wide.truncate(floor);
    wide.extend_from_slice(&[72, 0x80, 0x02]);
    assert_eq!(decode_player_report_request(&wide), Err(Error::Malformed));
}

#[test]
fn player_report_semantic_shape_refuses_unknown_and_foreign_fields() {
    assert_eq!(validate_player_report(&bug()), Ok(()));
    assert_eq!(validate_player_report(&violation()), Ok(()));
    for category in 1..=MAX_BUG_CATEGORY {
        let other = PlayerReportRequest {
            category,
            map_x: 0,
            map_y: 0,
            map_floor: 0,
            ..bug()
        };
        assert_eq!(validate_player_report(&other), Ok(()));
        let positioned = PlayerReportRequest { category, ..bug() };
        assert_eq!(
            validate_player_report(&positioned),
            Err(PlayerReportCode::Invalid)
        );
    }
    let invalid = [
        PlayerReportRequest { kind: 0, ..bug() },
        PlayerReportRequest {
            kind: 3,
            ..violation()
        },
        PlayerReportRequest {
            category: MAX_BUG_CATEGORY + 1,
            ..bug()
        },
        PlayerReportRequest {
            target: "Alice".into(),
            ..bug()
        },
        PlayerReportRequest { reason: 1, ..bug() },
        PlayerReportRequest {
            reason: MAX_REPORT_REASON + 1,
            ..violation()
        },
        PlayerReportRequest {
            action: MAX_REPORT_ACTION + 1,
            ..violation()
        },
        PlayerReportRequest {
            map_x: 1,
            ..violation()
        },
        PlayerReportRequest {
            category: 1,
            ..violation()
        },
        PlayerReportRequest {
            target: String::new(),
            ..violation()
        },
        PlayerReportRequest {
            target: "Alice  Bob".into(),
            ..violation()
        },
        PlayerReportRequest {
            target: "Al1ce".into(),
            ..violation()
        },
    ];
    for r in invalid {
        assert_eq!(
            validate_player_report(&r),
            Err(PlayerReportCode::Invalid),
            "{r:?}"
        );
    }
}

#[test]
fn player_report_reply_codec_binds_code_number_and_wait() {
    let accepted = PlayerReportReply {
        code: PlayerReportCode::Accepted,
        report: i64::MAX as u64,
        wait_seconds: 0,
    };
    let limited = PlayerReportReply {
        code: PlayerReportCode::RateLimited,
        report: 0,
        wait_seconds: u32::MAX,
    };
    let mut replies = vec![accepted, limited];
    for code in [
        PlayerReportCode::Invalid,
        PlayerReportCode::SelfReport,
        PlayerReportCode::AlreadyPending,
    ] {
        replies.push(PlayerReportReply {
            code,
            report: 0,
            wait_seconds: 0,
        });
    }
    for r in replies {
        let bytes = encode_player_report_reply(&r).expect("reply");
        assert!(bytes.len() <= MAX_REPLY_BYTES);
        assert_eq!(decode_player_report_reply(&bytes), Ok(r));
        for end in 0..bytes.len() {
            assert!(decode_player_report_reply(&bytes[..end]).is_err());
        }
    }
    for bad in [
        PlayerReportReply {
            report: 0,
            ..accepted
        },
        PlayerReportReply {
            wait_seconds: 1,
            ..accepted
        },
        PlayerReportReply {
            wait_seconds: 0,
            ..limited
        },
        PlayerReportReply {
            report: 1,
            ..limited
        },
    ] {
        assert_eq!(encode_player_report_reply(&bad), Err(Error::Malformed));
    }
    assert!(decode_player_report_reply(&[8, 9, 16, 0, 24, 0]).is_err());
    assert!(decode_player_report_reply(&[8, 2, 16, 0, 24, 0, 24, 0]).is_err());
}
