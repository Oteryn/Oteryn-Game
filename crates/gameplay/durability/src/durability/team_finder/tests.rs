use super::*;

fn listing() -> LeaderListing {
    LeaderListing {
        min_level: 8,
        max_level: 120,
        vocation_mask: 15,
        slots: 5,
        free_slots: 4,
        party_members_join: false,
        timestamp: 1_700_000_000,
        target: TeamTarget::Hunt { kind: 2, area: 9 },
    }
}

#[test]
fn team_finder_occupancy_preserves_leader_baseline_without_double_counting() {
    assert_eq!(effective_listing(listing(), 0).1, 1);
    assert_eq!(effective_listing(listing(), 1).1, 1);
    assert_eq!(effective_listing(listing(), 2).1, 2);
    assert_eq!(effective_listing(listing(), 2).0.free_slots, 3);
}

#[test]
fn team_finder_effective_capacity_floors_at_zero() {
    for accepted in [5, 6, u64::MAX] {
        assert_eq!(effective_listing(listing(), accepted).0.free_slots, 0);
    }
    let full = LeaderListing {
        free_slots: 0,
        ..listing()
    };
    assert_eq!(effective_listing(full, 0).0.free_slots, 0);
    assert_eq!(effective_listing(full, 1).0.free_slots, 0);
}

#[test]
fn team_finder_cancelled_acceptance_releases_only_derived_occupancy() {
    let (full, _) = effective_listing(listing(), 5);
    assert_eq!(full.free_slots, 0);
    let (cancelled, _) = effective_listing(listing(), 4);
    assert_eq!(cancelled.free_slots, 1);
    let (empty, _) = effective_listing(listing(), 0);
    assert_eq!(empty.free_slots, listing().free_slots);
}

#[test]
fn team_finder_client_vocations_preserve_promoted_donor_ids() -> Result<()> {
    for (name, expected) in [
        ("none", 0),
        ("knight", 1),
        ("paladin", 2),
        ("sorcerer", 3),
        ("druid", 4),
        ("monk", 5),
        ("elite_knight", 11),
        ("royal_paladin", 12),
        ("master_sorcerer", 13),
        ("elder_druid", 14),
        ("exalted_monk", 15),
    ] {
        assert_eq!(client_vocation(name)?, expected);
    }
    assert!(client_vocation("unknown").is_err());
    Ok(())
}

#[test]
fn team_finder_query_bound_keeps_one_extra_row_for_continuation() -> Result<()> {
    assert_eq!(page_limit(100)?, 100);
    assert_eq!(page_limit(101)?, 101);
    for invalid in [i64::MIN, -1, 0, 102, i64::MAX] {
        assert!(page_limit(invalid).is_err());
    }
    Ok(())
}

#[test]
fn team_finder_status_rejects_unknown_and_negative_database_values() -> Result<()> {
    for (number, expected) in [
        (0, MemberStatus::None),
        (1, MemberStatus::Pending),
        (2, MemberStatus::Invited),
        (3, MemberStatus::Accepted),
        (4, MemberStatus::Denied),
    ] {
        assert_eq!(status(number)?, expected);
    }
    for number in [i16::MIN, -1, 5, 256, i16::MAX] {
        assert!(status(number).is_err());
    }
    Ok(())
}

#[test]
fn team_finder_leader_cannot_decide_without_an_existing_applicant() {
    for previous in [None, Some(MemberStatus::None)] {
        for desired in [
            MemberStatus::Invited,
            MemberStatus::Accepted,
            MemberStatus::Denied,
        ] {
            assert_eq!(
                leader_decision(previous, desired, 4, false),
                LeaderDecision::Refused
            );
        }
    }
}

#[test]
fn team_finder_full_listing_refuses_new_acceptance_but_allows_invite_and_deny() {
    for previous in [
        MemberStatus::Pending,
        MemberStatus::Invited,
        MemberStatus::Denied,
    ] {
        assert_eq!(
            leader_decision(Some(previous), MemberStatus::Accepted, 0, false),
            LeaderDecision::Refused
        );
        for desired in [MemberStatus::Invited, MemberStatus::Denied] {
            assert_eq!(
                leader_decision(Some(previous), desired, 0, false),
                LeaderDecision::StatusOnly
            );
        }
    }
}

#[test]
fn team_finder_repeated_acceptance_never_reserves_twice_or_renews_a_party_invitation() {
    assert_eq!(
        leader_decision(
            Some(MemberStatus::Accepted),
            MemberStatus::Accepted,
            0,
            false
        ),
        LeaderDecision::StatusOnly
    );
    assert_eq!(
        leader_decision(
            Some(MemberStatus::Accepted),
            MemberStatus::Accepted,
            4,
            false
        ),
        LeaderDecision::StatusOnly
    );
}

#[test]
fn team_finder_acceptance_reuses_current_party_membership_after_status_changes() {
    // Accepted -> Invited -> Accepted may happen after ordinary party Accept.
    // Strong existing-owner membership avoids issuing an invite to its member.
    assert_eq!(
        leader_decision(Some(MemberStatus::Invited), MemberStatus::Accepted, 1, true),
        LeaderDecision::StatusOnly
    );
    assert_eq!(
        leader_decision(
            Some(MemberStatus::Invited),
            MemberStatus::Accepted,
            1,
            false
        ),
        LeaderDecision::PartyInvitation
    );
    assert_eq!(
        leader_decision(
            Some(MemberStatus::Pending),
            MemberStatus::Accepted,
            1,
            false
        ),
        LeaderDecision::PartyInvitation
    );
}

#[test]
fn team_finder_leader_cannot_synthesize_a_request_or_erase_status_with_none() {
    for desired in [MemberStatus::None, MemberStatus::Pending] {
        assert_eq!(
            leader_decision(Some(MemberStatus::Pending), desired, 4, false),
            LeaderDecision::Refused
        );
    }
}
