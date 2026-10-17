use super::view::{PartyViews, ViewError};
use crate::foundation::{PartyMemberSnapshot, PartyPresence, PartySnapshot};
use std::time::Duration;

fn at(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}
fn id(value: u8) -> [u8; 16] {
    [value; 16]
}

// Presentation DATA fixtures only; they do not construct an actor, fence or authority.
fn party(character: u8, revision: u64) -> PartySnapshot {
    PartySnapshot {
        world: id(1),
        character: id(character),
        party_id: Some(id(2)),
        revision: Some(revision),
        leader: Some(id(3)),
        shared_xp_enabled: true,
        members: [3, 4]
            .into_iter()
            .enumerate()
            .map(|(index, member)| PartyMemberSnapshot {
                character: id(member),
                invitation_order: index as u64 + 1,
                presence_revision: 1,
                presence: PartyPresence::Channel,
                channel: Some(id(5)),
            })
            .collect(),
        observed_database_micros: 100,
    }
}
fn solo(character: u8) -> PartySnapshot {
    PartySnapshot {
        party_id: None,
        revision: None,
        leader: None,
        shared_xp_enabled: false,
        members: vec![],
        ..party(character, 1)
    }
}
fn ready() -> Result<PartyViews, ViewError> {
    let mut views = PartyViews::new(id(1));
    let ticket = views.begin_refresh()?;
    views.finish_refresh(ticket, vec![party(3, 1)], at(0))?;
    Ok(views)
}

#[test]
fn restart_requires_full_refresh_and_unobserved_is_not_solo() -> Result<(), ViewError> {
    let mut views = PartyViews::new(id(1));
    assert_eq!(views.membership(id(3), at(0)), Err(ViewError::Unavailable));
    let ticket = views.begin_refresh()?;
    views.finish_refresh(ticket, vec![solo(3)], at(0))?;
    assert_eq!(views.membership(id(3), at(0)), Ok(None));
    assert_eq!(views.membership(id(4), at(0)), Err(ViewError::Unavailable));
    Ok(())
}

#[test]
fn revision_poll_is_due_at_five_seconds_and_unconfirmed_view_stale_at_ten() -> Result<(), ViewError>
{
    let views = ready()?;
    assert!(!views.revision_check_due(at(4)));
    assert!(views.revision_check_due(at(5)));
    assert!(views.ready(at(9)));
    assert!(!views.ready(at(10)));
    assert_eq!(views.membership(id(3), at(10)), Err(ViewError::Unavailable));
    Ok(())
}

#[test]
fn lost_hint_is_caught_by_revision_and_presence_batch() -> Result<(), ViewError> {
    let mut views = ready()?;
    let mut changed = party(3, 2);
    changed.shared_xp_enabled = false;
    changed.members[1].presence_revision = 2;
    changed.members[1].presence = PartyPresence::Hidden;
    changed.members[1].channel = None;
    let ticket = views.begin_refresh()?;
    assert!(!views.ready(at(5)));
    views.finish_refresh(ticket, vec![changed], at(5))?;
    let membership = views
        .membership(id(3), at(5))?
        .ok_or(ViewError::Unavailable)?;
    assert_eq!(membership.revision, 2);
    assert!(!membership.shared_xp_enabled);
    assert!(views.ready(at(14)));
    assert!(!views.ready(at(15)));
    Ok(())
}

#[test]
fn hint_invalidation_never_applies_hinted_data() -> Result<(), ViewError> {
    let mut views = ready()?;
    assert!(!views.observe_hint(id(99), 100)?);
    // Same-revision hint (presence-only change) still forces a durable reread.
    assert!(views.observe_hint(id(2), 1)?);
    assert_eq!(views.membership(id(3), at(1)), Err(ViewError::Unavailable));
    let mut views = ready()?;
    assert!(views.observe_hint(id(2), 2)?);
    assert_eq!(views.membership(id(3), at(1)), Err(ViewError::Unavailable));
    Ok(())
}

#[test]
fn relay_reconnect_supersedes_an_inflight_read() -> Result<(), ViewError> {
    let mut views = ready()?;
    let old = views.begin_refresh()?;
    let current = views.begin_refresh()?;
    assert_eq!(
        views.finish_refresh(old, vec![party(3, 2)], at(1)),
        Err(ViewError::Superseded)
    );
    assert!(!views.ready(at(1)));
    views.finish_refresh(current, vec![party(3, 2)], at(1))?;
    assert!(views.ready(at(1)));
    Ok(())
}

#[test]
fn completed_ticket_cannot_be_replayed_to_extend_freshness() -> Result<(), ViewError> {
    let mut views = ready()?;
    let ticket = views.begin_refresh()?;
    views.finish_refresh(ticket, vec![party(3, 1)], at(5))?;
    assert_eq!(
        views.finish_refresh(ticket, vec![party(3, 1)], at(14)),
        Err(ViewError::Superseded)
    );
    assert!(!views.ready(at(15)));
    Ok(())
}

#[test]
fn failed_or_rolled_back_admission_cannot_keep_old_membership_ready() -> Result<(), ViewError> {
    let mut views = ready()?;
    views.begin_admission(id(3))?;
    assert_eq!(views.membership(id(3), at(1)), Err(ViewError::Unavailable));
    // No finish occurs if the native transaction fails or rolls back.
    let retry = views.begin_admission(id(3))?;
    views.finish_admission(retry, solo(3))?;
    assert_eq!(views.membership(id(3), at(1)), Ok(None));
    Ok(())
}

#[test]
fn admission_does_not_replace_full_refresh_or_confirm_other_parties() -> Result<(), ViewError> {
    let mut startup = PartyViews::new(id(1));
    let admission = startup.begin_admission(id(3))?;
    startup.finish_admission(admission, party(3, 1))?;
    assert!(!startup.ready(at(0)));
    let mut views = ready()?;
    let admission = views.begin_admission(id(4))?;
    views.finish_admission(admission, party(4, 1))?;
    assert!(!views.ready(at(10)));
    Ok(())
}

#[test]
fn admission_ticket_is_bound_to_the_character_and_supersedes_pending_full_read()
-> Result<(), ViewError> {
    let mut views = ready()?;
    let full = views.begin_refresh()?;
    let admission = views.begin_admission(id(4))?;
    assert_eq!(
        views.finish_admission(admission, party(3, 1)),
        Err(ViewError::Superseded)
    );
    assert_eq!(
        views.finish_refresh(full, vec![party(3, 1)], at(1)),
        Err(ViewError::Superseded)
    );
    assert!(!views.ready(at(1)));
    Ok(())
}

#[test]
fn last_local_exit_drops_view_but_other_local_member_keeps_it() -> Result<(), ViewError> {
    let mut views = ready()?;
    let ticket = views.begin_refresh()?;
    views.finish_refresh(ticket, vec![party(3, 1), party(4, 1)], at(1))?;
    views.remove_local(id(3))?;
    assert!(views.membership(id(4), at(1))?.is_some());
    views.remove_local(id(4))?;
    // A hint for a view with no local member does not schedule a refresh.
    assert!(!views.observe_hint(id(2), 2)?);
    Ok(())
}

#[test]
fn stale_membership_or_presence_cannot_overwrite_hidden_invalidation() -> Result<(), ViewError> {
    let mut views = ready()?;
    let mut hidden = party(3, 2);
    hidden.members[1].presence = PartyPresence::Hidden;
    hidden.members[1].channel = None;
    hidden.members[1].presence_revision = 2;
    let ticket = views.begin_refresh()?;
    views.finish_refresh(ticket, vec![hidden.clone()], at(1))?;
    for stale in [party(3, 1), party(3, 2)] {
        let ticket = views.begin_refresh()?;
        assert_eq!(
            views.finish_refresh(ticket, vec![stale], at(2)),
            Err(ViewError::RevisionRegressed)
        );
        assert!(!views.ready(at(2)));
    }
    let mut substituted = hidden;
    substituted.members[1].presence = PartyPresence::Channel;
    substituted.members[1].channel = Some(id(5));
    let ticket = views.begin_refresh()?;
    assert_eq!(
        views.finish_refresh(ticket, vec![substituted], at(2)),
        Err(ViewError::RevisionRegressed)
    );
    Ok(())
}

#[test]
fn channel_entry_presence_can_advance_without_party_revision() -> Result<(), ViewError> {
    let mut views = ready()?;
    let mut changed = party(3, 1);
    changed.members[1].presence_revision = 2;
    changed.members[1].channel = Some(id(6));
    let ticket = views.begin_refresh()?;
    views.finish_refresh(ticket, vec![changed], at(5))?;
    let membership = views
        .membership(id(3), at(5))?
        .ok_or(ViewError::Unavailable)?;
    assert_eq!(membership.revision, 1);
    assert_eq!(membership.members, vec![id(3), id(4)]);
    // The projection's type contains no remote channel or vitals.
    Ok(())
}

#[test]
fn wrong_world_duplicate_locals_and_inconsistent_party_reads_fail_closed() -> Result<(), ViewError>
{
    let mut views = ready()?;
    let mut wrong_world = party(3, 1);
    wrong_world.world = id(9);
    let mut disagreement = party(4, 1);
    disagreement.members[1].presence_revision = 2;
    disagreement.members[1].channel = Some(id(6));
    for snapshots in [
        vec![wrong_world],
        vec![party(3, 1), party(3, 1)],
        vec![party(3, 1), disagreement],
    ] {
        let ticket = views.begin_refresh()?;
        assert_eq!(
            views.finish_refresh(ticket, snapshots, at(1)),
            Err(ViewError::InvalidSnapshot)
        );
        assert!(!views.ready(at(1)));
    }
    Ok(())
}

#[test]
fn malformed_membership_and_presence_shapes_never_become_ready() -> Result<(), ViewError> {
    let mut bad = Vec::new();
    let mut snapshot = party(3, 1);
    snapshot.members[1].character = id(3);
    bad.push(snapshot);
    let mut snapshot = party(3, 1);
    snapshot.leader = Some(id(8));
    bad.push(snapshot);
    let mut snapshot = party(8, 1);
    snapshot.character = id(8);
    bad.push(snapshot);
    let mut snapshot = party(3, 0);
    snapshot.revision = Some(0);
    bad.push(snapshot);
    let mut snapshot = party(3, 1);
    snapshot.members[1].presence = PartyPresence::Hidden;
    bad.push(snapshot);
    let mut snapshot = party(3, 1);
    snapshot.members[1].channel = None;
    bad.push(snapshot);
    let mut snapshot = solo(3);
    snapshot.shared_xp_enabled = true;
    bad.push(snapshot);
    for snapshot in bad {
        let mut views = PartyViews::new(id(1));
        let ticket = views.begin_refresh()?;
        assert_eq!(
            views.finish_refresh(ticket, vec![snapshot], at(0)),
            Err(ViewError::InvalidSnapshot)
        );
        assert!(!views.ready(at(0)));
    }
    Ok(())
}

#[test]
fn monotonic_clock_regression_fails_readiness_closed() -> Result<(), ViewError> {
    let mut views = PartyViews::new(id(1));
    let ticket = views.begin_refresh()?;
    views.finish_refresh(ticket, vec![party(3, 1)], at(10))?;
    assert!(!views.ready(at(9)));
    assert!(views.revision_check_due(at(9)));
    Ok(())
}

#[test]
fn full_refresh_rejects_mixed_time_solo_and_another_local_membership() -> Result<(), ViewError> {
    let mut views = ready()?;
    let ticket = views.begin_refresh()?;
    assert_eq!(
        views.finish_refresh(ticket, vec![solo(3), party(4, 1)], at(1)),
        Err(ViewError::InvalidSnapshot)
    );
    assert!(!views.ready(at(1)));
    Ok(())
}

#[test]
fn admission_cannot_leave_a_retained_party_claiming_the_departed_member() -> Result<(), ViewError> {
    for solo_admission in [false, true] {
        let mut views = ready()?;
        let ticket = views.begin_refresh()?;
        views.finish_refresh(ticket, vec![party(3, 1), party(4, 1)], at(1))?;
        let ticket = views.begin_admission(id(3))?;
        let mut new_party = party(3, 1);
        new_party.party_id = Some(id(8));
        new_party.members.retain(|member| member.character == id(3));
        let snapshot = if solo_admission { solo(3) } else { new_party };
        assert_eq!(
            views.finish_admission(ticket, snapshot),
            Err(ViewError::InvalidSnapshot)
        );
        assert_eq!(views.membership(id(4), at(1)), Err(ViewError::Unavailable));
    }
    Ok(())
}

#[test]
fn disconnect_supersedes_both_full_refresh_and_admission_completions() -> Result<(), ViewError> {
    let mut views = ready()?;
    let ticket = views.begin_refresh()?;
    views.remove_local(id(3))?;
    assert_eq!(
        views.finish_refresh(ticket, vec![party(3, 1)], at(1)),
        Err(ViewError::Superseded)
    );
    assert_eq!(views.membership(id(3), at(1)), Err(ViewError::Unavailable));
    let admission = views.begin_admission(id(3))?;
    views.remove_local(id(3))?;
    assert_eq!(
        views.finish_admission(admission, party(3, 1)),
        Err(ViewError::Superseded)
    );
    assert_eq!(views.membership(id(3), at(1)), Err(ViewError::Unavailable));
    Ok(())
}
