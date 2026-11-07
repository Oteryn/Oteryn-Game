fn caps(item_use: bool, world_interaction: bool) -> UseCapabilities {
    UseCapabilities {
        item_use,
        world_interaction,
        rune_use: false,
    }
}

#[test]
fn use_dispositions_7_to_10_need_capability_15() {
    use UseDisposition as D;
    let neg = |i, w, d| negotiated_use_disposition(caps(i, w), d, None).0;
    for d in [D::RequirementNotMet, D::Exhausted, D::Full, D::NoTarget] {
        assert_eq!(neg(false, false, d), D::Rejected);
        assert_eq!(neg(true, false, d), d);
    }
    for d in [
        D::Locked,
        D::KeyMismatch,
        D::PzBlocked,
        D::NotMovable,
        D::NoRoom,
        D::NotPossible,
    ] {
        assert_eq!(neg(true, false, d), D::Rejected);
        assert_eq!(neg(true, true, d), d);
    }
    for d in [
        D::Committed,
        D::NothingToUse,
        D::Occupied,
        D::StaleState,
        D::TooFar,
    ] {
        assert_eq!(neg(false, false, d), d);
    }
}

/// RUNE-WIRE-1: TARGET_ILLEGAL and PROTECTION_ZONE reach only a session that selected 22;
/// capability 11 alone does not admit them, and 22 alone does not admit 11 to 17.
#[test]
fn use_dispositions_18_and_19_need_capability_21() {
    use UseDisposition as D;
    let all = UseCapabilities {
        item_use: true,
        world_interaction: true,
        rune_use: true,
    };
    let runes_only = UseCapabilities {
        world_interaction: false,
        ..all
    };
    for d in [D::TargetIllegal, D::ProtectionZone] {
        assert_eq!(
            negotiated_use_disposition(caps(true, true), d, None).0,
            D::Rejected
        );
        assert_eq!(negotiated_use_disposition(all, d, None), (d, None));
        assert_eq!(negotiated_use_disposition(runes_only, d, None), (d, None));
        assert_eq!(
            negotiated_use_disposition(all, d, Some(9)),
            (D::Rejected, None)
        );
    }
    assert_eq!(
        negotiated_use_disposition(runes_only, D::Locked, None).0,
        D::Rejected
    );
    for d in [D::RequirementNotMet, D::Exhausted, D::NoTarget, D::TooFar] {
        assert_eq!(negotiated_use_disposition(runes_only, d, None).0, d);
    }
}

#[test]
fn sealed_carries_its_gate_message_id_only_under_capability_11() {
    use UseDisposition as D;
    assert_eq!(
        negotiated_use_disposition(caps(true, true), D::Sealed, Some(9)),
        (D::Sealed, Some(9))
    );
    assert_eq!(
        negotiated_use_disposition(caps(true, false), D::Sealed, Some(9)),
        (D::Rejected, None)
    );
    for bad in [None, Some(0), Some(128)] {
        assert_eq!(
            negotiated_use_disposition(caps(true, true), D::Sealed, bad),
            (D::Rejected, None)
        );
    }
    assert_eq!(
        negotiated_use_disposition(caps(true, true), D::Locked, Some(9)),
        (D::Rejected, None)
    );
    let payload = encode_use_result_with_gate(D::Sealed, Some(9)).expect("sealed");
    assert_eq!(
        decode_use_result_with_gate(&payload).expect("decode"),
        (D::Sealed, Some(9))
    );
}
