use super::*;

#[test]
fn stash_strict_codecs_and_limits() -> Result<(), Error> {
    for action in [
        StashAction::Browse,
        StashAction::Stow(ItemHandle::new(1).ok_or(Error::Malformed)?),
        StashAction::Withdraw {
            row: [1; 16],
            amount: 1,
        },
        StashAction::Withdraw {
            row: [2; 16],
            amount: u32::MAX,
        },
    ] {
        let intent = StashIntent {
            page: MAX_STASH_PAGE,
            action,
        };
        let wire = encode_stash_intent(intent)?;
        assert_eq!(decode_stash_intent(&wire)?, intent);
        let mut duplicate = wire;
        duplicate.extend_from_slice(&[8, 0]);
        assert!(decode_stash_intent(&duplicate).is_err());
    }
    assert!(decode_stash_intent(&[0; 49]).is_err());
    assert!(decode_stash_intent(&[8, 0, 16, 1, 24, 1]).is_err());
    assert!(
        encode_stash_intent(StashIntent {
            page: 0,
            action: StashAction::Withdraw {
                row: [1; 16],
                amount: 0
            }
        })
        .is_err()
    );
    let row = StashRow {
        handle: [1; 16],
        definition_key: "k".into(),
        definition_revision: "r1".into(),
        quantity: MAX_STASH_UNITS,
        migration_required: true,
    };
    for outcome in [
        StashOutcome::Opened,
        StashOutcome::Committed,
        StashOutcome::NoLocker,
        StashOutcome::Stale,
        StashOutcome::NoRoom,
        StashOutcome::NoBackpack,
        StashOutcome::Overweight,
        StashOutcome::Unsupported,
        StashOutcome::Rejected,
        StashOutcome::PremiumRequired,
        StashOutcome::AmountLimit,
        StashOutcome::Insufficient,
    ] {
        let reply = StashReply {
            outcome,
            stowed_quantity: if outcome == StashOutcome::Committed {
                100
            } else {
                0
            },
            transaction: Some([2; 16]),
            page: 0,
            has_more: false,
            rows: vec![row.clone()],
        };
        assert_eq!(decode_stash_reply(&encode_stash_reply(&reply)?)?, reply);
    }
    let mut reply = StashReply {
        outcome: StashOutcome::Opened,
        stowed_quantity: 0,
        transaction: None,
        page: 0,
        has_more: true,
        rows: (0..32)
            .map(|i| StashRow {
                handle: [i + 1; 16],
                definition_key: format!("{i:03}{}", "k".repeat(509)),
                definition_revision: "r".repeat(512),
                ..row.clone()
            })
            .collect(),
    };
    let wire = encode_stash_reply(&reply)?;
    assert!(wire.len() <= MAX_STASH_REPLY_BYTES);
    assert_eq!(decode_stash_reply(&wire)?, reply);
    reply.rows.push(row);
    assert!(encode_stash_reply(&reply).is_err());
    Ok(())
}

#[test]
fn stash_view_criteria_are_bounded_and_preserve_base_conversion_identity() -> Result<(), Error> {
    let intent = StashIntent {
        page: 0,
        action: StashAction::Withdraw {
            row: [3; 16],
            amount: 2,
        },
    };
    let filter = StashFilter {
        search: "apple".into(),
        category: "rune".into(),
        trader: "Sam".into(),
        stock_order: true,
    };
    let bytes = encode_stash_query(intent, Some(&filter))?;
    assert_eq!(decode_stash_query(&bytes)?, (intent, Some(filter.clone())));
    assert_eq!(
        decode_stash_query(&encode_stash_intent(intent)?)?,
        (intent, None)
    );
    let mut duplicate = bytes.clone();
    duplicate.extend_from_slice(&bytes[encode_stash_intent(intent)?.len()..]);
    assert!(decode_stash_query(&duplicate).is_err());
    let invalid = StashFilter {
        search: "x".repeat(129),
        ..filter
    };
    assert!(encode_stash_query(intent, Some(&invalid)).is_err());
    Ok(())
}

#[test]
fn stash_stock_order_crosses_the_wire_without_reordering_handles() -> Result<(), Error> {
    let mut reply = StashReply {
        outcome: StashOutcome::Opened,
        stowed_quantity: 0,
        transaction: None,
        page: 0,
        has_more: false,
        rows: vec![
            StashRow {
                handle: [2; 16],
                definition_key: "z".into(),
                definition_revision: "r1".into(),
                quantity: 9,
                migration_required: false,
            },
            StashRow {
                handle: [1; 16],
                definition_key: "a".into(),
                definition_revision: "r1".into(),
                quantity: 2,
                migration_required: false,
            },
        ],
    };
    assert_eq!(decode_stash_reply(&encode_stash_reply(&reply)?)?, reply);
    reply.rows[1].handle = reply.rows[0].handle;
    assert!(encode_stash_reply(&reply).is_err());
    Ok(())
}
