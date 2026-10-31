use super::*;
use crate::charm_wire::push_message_field;
use crate::team_finder::TeamTarget;

const LISTING: LeaderListing = LeaderListing {
    min_level: 8,
    max_level: 100,
    vocation_mask: 15,
    slots: 5,
    free_slots: 4,
    party_members_join: false,
    timestamp: 7,
    target: TeamTarget::Hunt { kind: 2, area: 9 },
};

fn browse(handle: u64) -> BrowseListing {
    BrowseListing {
        leader_handle: handle,
        name: "Leader".into(),
        listing: LISTING,
        occupied_slots: 2,
        status: MemberStatus::Pending,
    }
}

fn applicant(handle: u64) -> Applicant {
    Applicant {
        handle,
        name: "Applicant".into(),
        level: 20,
        vocation: 4,
        status: MemberStatus::Accepted,
    }
}

#[test]
fn query_and_cursor_round_trip() -> Result<(), TeamFinderWireError> {
    for after in [None, Some(1), Some(u64::MAX)] {
        for query in [
            TeamFinderQuery::Browse { after },
            TeamFinderQuery::Applicants { after },
        ] {
            assert_eq!(decode_query(&encode_query(query)?)?, query);
        }
    }
    assert_eq!(
        encode_query(TeamFinderQuery::Browse { after: None })?,
        [8, 1]
    );
    assert_eq!(
        encode_query(TeamFinderQuery::Applicants { after: Some(9) })?,
        [8, 2, 16, 9]
    );
    Ok(())
}

#[test]
fn member_actions_have_canonical_receipt_bytes() -> Result<(), TeamFinderWireError> {
    for action in [
        MemberAction::Request { leader: 9 },
        MemberAction::Cancel { leader: 9 },
    ] {
        assert_eq!(
            decode_member_action(&encode_member_action(action)?)?,
            action
        );
    }
    assert_eq!(
        encode_member_action(MemberAction::Request { leader: 9 })?,
        [8, 1, 16, 9]
    );
    assert_eq!(
        encode_member_action(MemberAction::Cancel { leader: 9 })?,
        [8, 2, 16, 9]
    );
    Ok(())
}

#[test]
fn leader_decisions_use_donor_status_values() -> Result<(), TeamFinderWireError> {
    for (action, status) in [
        (LeaderAction::Invite { member: 9 }, 2),
        (LeaderAction::Accept { member: 9 }, 3),
        (LeaderAction::Deny { member: 9 }, 4),
    ] {
        assert_eq!(encode_leader_action(action)?, [8, status, 16, 9]);
        assert_eq!(
            decode_leader_action(&encode_leader_action(action)?)?,
            action
        );
    }
    Ok(())
}

#[test]
fn missing_unknown_duplicate_wrong_wire_and_overflow_requests_are_refused() {
    for bytes in [
        vec![],
        vec![8, 0],
        vec![8, 9],
        vec![16, 1],
        vec![8, 1, 16, 0],
        vec![8, 1, 8, 1],
        vec![8, 1, 16, 1, 16, 1],
        vec![8, 1, 24, 1],
        vec![10, 1, 1],
        vec![8, 1, 16, 128],
        vec![8, 1, 16, 255, 255, 255, 255, 255, 255, 255, 255, 255, 2],
    ] {
        assert!(decode_query(&bytes).is_err(), "{bytes:?}");
        assert!(decode_member_action(&bytes).is_err(), "{bytes:?}");
        assert!(decode_leader_action(&bytes).is_err(), "{bytes:?}");
    }
    assert!(decode_member_action(&[8, 1]).is_err());
    assert!(decode_leader_action(&[8, 3]).is_err());
    assert!(decode_member_action(&[8, 3, 16, 1]).is_err());
    assert!(decode_leader_action(&[8, 1, 16, 1]).is_err());
}

#[test]
fn all_zero_handle_and_oversized_requests_are_refused() {
    assert!(encode_query(TeamFinderQuery::Browse { after: Some(0) }).is_err());
    assert!(encode_query(TeamFinderQuery::Applicants { after: Some(0) }).is_err());
    assert!(encode_member_action(MemberAction::Request { leader: 0 }).is_err());
    assert!(encode_member_action(MemberAction::Cancel { leader: 0 }).is_err());
    for action in [
        LeaderAction::Invite { member: 0 },
        LeaderAction::Accept { member: 0 },
        LeaderAction::Deny { member: 0 },
    ] {
        assert!(encode_leader_action(action).is_err());
    }
    let bytes = vec![0; MAX_REQUEST_BYTES + 1];
    assert_eq!(
        decode_query(&bytes),
        Err(TeamFinderWireError::LimitExceeded)
    );
    assert_eq!(
        decode_member_action(&bytes),
        Err(TeamFinderWireError::LimitExceeded)
    );
    assert_eq!(
        decode_leader_action(&bytes),
        Err(TeamFinderWireError::LimitExceeded)
    );
}

#[test]
fn browse_round_trips_every_target_status_and_donor_field() -> Result<(), TeamFinderWireError> {
    for target in [
        TeamTarget::Boss(7),
        TeamTarget::Hunt { kind: 2, area: 8 },
        TeamTarget::Quest(9),
    ] {
        for status in [
            MemberStatus::None,
            MemberStatus::Pending,
            MemberStatus::Invited,
            MemberStatus::Accepted,
            MemberStatus::Denied,
        ] {
            let mut entry = browse(42);
            entry.listing.target = target;
            entry.status = status;
            let reply = TeamFinderResult::Browse {
                listings: vec![entry],
                next: Some(42),
            };
            assert_eq!(decode_result(&encode_result(&reply)?)?, reply);
        }
    }
    Ok(())
}

#[test]
fn applicant_pages_preserve_global_occupancy_and_baseline() -> Result<(), TeamFinderWireError> {
    let reply = TeamFinderResult::Applicants {
        listing: Some(LISTING),
        occupied_slots: 4,
        members: vec![applicant(9)],
        next: Some(9),
    };
    assert_eq!(decode_result(&encode_result(&reply)?)?, reply);
    // Occupancy can exceed an edited baseline or team size; effective free saturates at zero.
    let reply = TeamFinderResult::Applicants {
        listing: Some(LISTING),
        occupied_slots: u16::MAX,
        members: vec![applicant(9)],
        next: Some(9),
    };
    assert_eq!(decode_result(&encode_result(&reply)?)?, reply);
    Ok(())
}

#[test]
fn no_advertisement_and_empty_browse_have_distinct_shapes() -> Result<(), TeamFinderWireError> {
    for reply in [
        TeamFinderResult::Browse {
            listings: vec![],
            next: None,
        },
        TeamFinderResult::Applicants {
            listing: None,
            occupied_slots: 0,
            members: vec![],
            next: None,
        },
        TeamFinderResult::Applicants {
            listing: Some(LISTING),
            occupied_slots: 1,
            members: vec![],
            next: None,
        },
    ] {
        assert_eq!(decode_result(&encode_result(&reply)?)?, reply);
    }
    Ok(())
}

#[test]
fn page_bound_is_a_transport_limit_with_continuation() -> Result<(), TeamFinderWireError> {
    let entries: Vec<_> = (1..=MAX_PAGE_ENTRIES as u64).map(browse).collect();
    let reply = TeamFinderResult::Browse {
        listings: entries,
        next: Some(MAX_PAGE_ENTRIES as u64),
    };
    assert_eq!(decode_result(&encode_result(&reply)?)?, reply);
    let TeamFinderResult::Browse { mut listings, .. } = reply else {
        return Err(TeamFinderWireError::Malformed);
    };
    listings.push(browse(MAX_PAGE_ENTRIES as u64 + 1));
    assert_eq!(
        encode_result(&TeamFinderResult::Browse {
            listings,
            next: None
        }),
        Err(TeamFinderWireError::LimitExceeded)
    );
    let members: Vec<_> = (1..=MAX_PAGE_ENTRIES as u64 + 1).map(applicant).collect();
    assert_eq!(
        encode_result(&TeamFinderResult::Applicants {
            listing: Some(LISTING),
            occupied_slots: 1,
            members,
            next: None
        }),
        Err(TeamFinderWireError::LimitExceeded)
    );
    Ok(())
}

#[test]
fn duplicate_zero_handles_and_impossible_continuations_are_refused() {
    for (listings, next) in [
        (vec![browse(0)], None),
        (vec![browse(1), browse(1)], None),
        (vec![browse(1)], Some(0)),
        (vec![browse(1)], Some(2)),
        (vec![], Some(1)),
    ] {
        assert!(encode_result(&TeamFinderResult::Browse { listings, next }).is_err());
    }
    assert!(
        encode_result(&TeamFinderResult::Applicants {
            listing: Some(LISTING),
            occupied_slots: 1,
            members: vec![applicant(1), applicant(1)],
            next: None
        })
        .is_err()
    );
}

#[test]
fn invalid_names_missing_listing_and_nonapplicant_status_are_refused() {
    for name in [String::new(), "x".repeat(MAX_NAME_BYTES + 1)] {
        let mut entry = browse(1);
        entry.name = name.clone();
        assert!(
            encode_result(&TeamFinderResult::Browse {
                listings: vec![entry],
                next: None
            })
            .is_err()
        );
        let mut member = applicant(1);
        member.name = name;
        assert!(
            encode_result(&TeamFinderResult::Applicants {
                listing: Some(LISTING),
                occupied_slots: 1,
                members: vec![member],
                next: None
            })
            .is_err()
        );
    }
    let mut member = applicant(1);
    member.status = MemberStatus::None;
    assert!(
        encode_result(&TeamFinderResult::Applicants {
            listing: Some(LISTING),
            occupied_slots: 1,
            members: vec![member],
            next: None
        })
        .is_err()
    );
    assert!(
        encode_result(&TeamFinderResult::Applicants {
            listing: None,
            occupied_slots: 0,
            members: vec![applicant(1)],
            next: None
        })
        .is_err()
    );
    assert!(
        encode_result(&TeamFinderResult::Applicants {
            listing: None,
            occupied_slots: 1,
            members: vec![],
            next: None
        })
        .is_err()
    );
}

#[test]
fn malformed_and_cross_kind_reply_fields_are_refused() -> Result<(), TeamFinderWireError> {
    for bytes in [
        vec![],
        vec![8, 0],
        vec![8, 3],
        vec![8, 1, 8, 1],
        vec![8, 1, 32, 0],
        vec![8, 2],
        vec![8, 2, 32, 0, 32, 0],
        vec![8, 1, 56, 0],
    ] {
        assert!(decode_result(&bytes).is_err(), "{bytes:?}");
    }
    let reply = TeamFinderResult::Browse {
        listings: vec![browse(1)],
        next: None,
    };
    let mut bytes = encode_result(&reply)?;
    push_message_field(&mut bytes, 3, &crate::team_finder::encode_upsert(LISTING)?);
    assert!(decode_result(&bytes).is_err());
    let mut bytes = vec![8, 2, 32, 0];
    push_message_field(&mut bytes, 2, &[8, 1]);
    assert!(decode_result(&bytes).is_err());
    assert_eq!(
        decode_result(&vec![0; MAX_RESULT_BYTES + 1]),
        Err(TeamFinderWireError::LimitExceeded)
    );
    Ok(())
}

#[test]
fn nested_reply_decoder_rejects_duplicates_unknown_status_and_bad_utf8()
-> Result<(), TeamFinderWireError> {
    for suffix in [vec![8, 1], vec![48, 1], vec![40, 5]] {
        let mut inner = Vec::new();
        push_varint_field(&mut inner, 1, 1);
        push_message_field(&mut inner, 2, b"Leader");
        push_message_field(&mut inner, 3, &crate::team_finder::encode_upsert(LISTING)?);
        push_varint_field(&mut inner, 4, 1);
        if suffix != [40, 5] {
            push_varint_field(&mut inner, 5, 0);
        }
        inner.extend_from_slice(&suffix);
        let mut bytes = vec![8, 1];
        push_message_field(&mut bytes, 2, &inner);
        assert!(decode_result(&bytes).is_err());
    }
    let mut inner = vec![8, 1];
    push_message_field(&mut inner, 2, &[255]);
    let mut bytes = vec![8, 1];
    push_message_field(&mut bytes, 2, &inner);
    assert!(decode_result(&bytes).is_err());
    for status in 0..=4 {
        assert_eq!(MemberStatus::try_from(status)? as u8, status);
    }
    assert!(MemberStatus::try_from(5).is_err());
    Ok(())
}
