//! Pure rule cases use explicit data fixtures, not current-authority producers.
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "explicit data fixture failures"
)]
use super::*;
use crate::foundation::{MovementLocalPosition, PartyMemberSnapshot, PartyPresence};

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}
fn time(n: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(n)
}
fn channel() -> ChannelId {
    ChannelId::decode(&id(3)).expect("channel")
}
fn member(seed: u8, level: u32, x: i32, y: i32, floor: i16) -> SourcePartyXpMember {
    SourcePartyXpMember {
        character: id(seed),
        session: GameSessionId::decode(&id(seed + 50)).expect("session"),
        level,
        position: MovementLocalPosition { x, y, floor },
    }
}
fn source(members: &[SourcePartyXpMember]) -> PartySnapshot {
    PartySnapshot {
        world: id(1),
        character: id(10),
        party_id: Some(id(2)),
        revision: Some(1),
        leader: Some(id(10)),
        observed_database_micros: 1,
        members: members
            .iter()
            .enumerate()
            .map(|(i, m)| PartyMemberSnapshot {
                character: m.character,
                invitation_order: i as u64 + 1,
                presence_revision: 1,
                presence: PartyPresence::Channel,
                channel: Some(id(3)),
            })
            .collect(),
    }
}
fn active(members: &[SourcePartyXpMember], at: u64) -> Vec<PartyXpActivity> {
    members
        .iter()
        .map(|m| {
            let mut cell = PartyXpActivity::default();
            cell.record_members(
                &source(members),
                channel(),
                members,
                PartyActivityEvent::AttackedAggressiveMonster {
                    attacker: m.character,
                    session: m.session,
                },
                time(at),
            )
            .expect("activity");
            cell
        })
        .collect()
}

fn pz(leader: SourcePartyXpMember, now: u64, deadline: Option<u64>) -> LeaderPzObservation {
    LeaderPzObservation {
        character: leader.character,
        session: leader.session,
        observed_at: time(now),
        pz_until: deadline.map(time),
    }
}
fn decide(
    s: &PartySnapshot,
    members: &[SourcePartyXpMember],
    activity: &[PartyXpActivity],
    now: u64,
) -> Result<PartyXpDecision, PartyXpError> {
    evaluate(
        s,
        channel(),
        members,
        true,
        activity,
        Some(pz(members[0], now, None)),
        time(now),
        ExactI64::new(101),
    )
}
fn off(value: Result<PartyXpDecision, PartyXpError>, reason: SharingOff) {
    assert_eq!(value, Ok(PartyXpDecision::Off(reason)));
}

#[test]
fn party_xp_range_any_floor_and_exact_two_thirds_boundary() {
    let members = [member(10, 300, 0, 0, 0), member(11, 200, 30, -30, 15)];
    let s = source(&members);
    let activity = active(&members, 0);
    let PartyXpDecision::Shares(shares) = decide(&s, &members, &activity, 0).expect("plan") else {
        panic!("eligible")
    };
    assert_eq!(
        shares.iter().map(|s| s.amount.get()).collect::<Vec<_>>(),
        [51, 51]
    );
    let mut outside = members;
    outside[1].position.x = 31;
    off(decide(&s, &outside, &activity, 0), SharingOff::Range);
    let mut low = members;
    low[1].level = 199;
    off(decide(&s, &low, &activity, 0), SharingOff::Level);
}
#[test]
fn party_xp_activity_two_minutes_is_inclusive_then_expires() {
    let members = [member(10, 100, 0, 0, 0)];
    let s = source(&members);
    let activity = active(&members, 1);
    assert!(matches!(
        decide(&s, &members, &activity, 120_000_001),
        Ok(PartyXpDecision::Shares(_))
    ));
    off(
        decide(&s, &members, &activity, 120_000_002),
        SharingOff::Inactive,
    );
    assert_eq!(
        decide(&s, &members, &activity, 0),
        Err(PartyXpError::ClockRegressed)
    );
}
#[test]
fn party_xp_heal_is_bound_to_current_member_and_session_without_state_change_on_refusal() {
    let members = [member(10, 100, 0, 0, 0), member(11, 100, 1, 1, 0)];
    let mut activity = active(&members, 1);
    activity[0]
        .record_members(
            &source(&members),
            channel(),
            &members,
            PartyActivityEvent::HealedMember {
                healer: members[0].character,
                session: members[0].session,
                healed: members[1].character,
            },
            time(2),
        )
        .expect("heal");
    let before = activity.clone();
    assert_eq!(
        activity[0].record_members(
            &source(&members),
            channel(),
            &members,
            PartyActivityEvent::HealedMember {
                healer: members[0].character,
                session: members[0].session,
                healed: id(99)
            },
            time(3)
        ),
        Err(PartyXpError::InvalidRoster)
    );
    assert_eq!(activity, before);
    assert_eq!(
        activity[0].record_members(
            &source(&members),
            channel(),
            &members,
            PartyActivityEvent::AttackedAggressiveMonster {
                attacker: members[0].character,
                session: members[1].session
            },
            time(3)
        ),
        Err(PartyXpError::InvalidRoster)
    );
    assert_eq!(activity, before);
    assert_eq!(
        activity[0].record_members(
            &source(&members),
            channel(),
            &members,
            PartyActivityEvent::AttackedAggressiveMonster {
                attacker: members[0].character,
                session: members[0].session
            },
            time(1)
        ),
        Err(PartyXpError::ClockRegressed)
    );
    assert_eq!(activity, before);
}
#[test]
fn party_xp_replaced_session_loses_activity() {
    let mut members = [member(10, 100, 0, 0, 0), member(11, 100, 1, 1, 0)];
    let mut activity = active(&members, 1);
    members[1].session = GameSessionId::decode(&id(99)).expect("new session");
    assert_eq!(
        decide(&source(&members), &members, &activity, 1),
        Err(PartyXpError::ActivityScopeChanged)
    );
    let before = activity[1];
    assert_eq!(
        activity[1].record_members(
            &source(&members),
            channel(),
            &members,
            PartyActivityEvent::AttackedAggressiveMonster {
                attacker: members[1].character,
                session: members[1].session
            },
            time(2)
        ),
        Err(PartyXpError::ActivityScopeChanged)
    );
    assert_eq!(activity[1], before);
    activity[1] = PartyXpActivity::default();
    activity[1]
        .record_members(
            &source(&members),
            channel(),
            &members,
            PartyActivityEvent::AttackedAggressiveMonster {
                attacker: members[1].character,
                session: members[1].session,
            },
            time(2),
        )
        .expect("new event");
    assert_eq!(activity.len(), 2);
    assert!(matches!(
        decide(&source(&members), &members, &activity, 2),
        Ok(PartyXpDecision::Shares(_))
    ));
}
#[test]
fn party_xp_foreign_channel_member_does_not_disable_local_sharing() {
    let members = [member(10, 100, 0, 0, 0), member(11, 100, 1, 1, 0)];
    let mut s = source(&members);
    s.members.push(PartyMemberSnapshot {
        character: id(99),
        invitation_order: 3,
        presence_revision: 1,
        presence: PartyPresence::Channel,
        channel: Some(id(4)),
    });
    let PartyXpDecision::Shares(shares) =
        decide(&s, &members, &active(&members, 0), 0).expect("plan")
    else {
        panic!("eligible")
    };
    assert_eq!(shares.len(), 2);
    assert!(shares.iter().all(|s| s.character != id(99)));
    s.leader = Some(id(99));
    off(
        decide(&s, &members, &active(&members, 0), 0),
        SharingOff::LeaderAbsent,
    );
}
#[test]
fn party_xp_one_inactive_participant_disables_whole_kill() {
    let members = [member(10, 100, 0, 0, 0), member(11, 100, 1, 1, 0)];
    let activity = active(&members[..1], 0);
    off(
        decide(&source(&members), &members, &activity, 0),
        SharingOff::Inactive,
    );
}
#[test]
fn party_xp_pz_deadline_is_not_logout_block_and_requires_current_sample() {
    let members = [member(10, 100, 0, 0, 0)];
    let s = source(&members);
    let activity = active(&members, 0);
    let eval = |sample| {
        evaluate(
            &s,
            channel(),
            &members,
            true,
            &activity,
            sample,
            time(10),
            ExactI64::new(20),
        )
    };
    off(
        eval(Some(pz(members[0], 10, Some(11)))),
        SharingOff::LeaderPzBlock,
    );
    assert!(matches!(
        eval(Some(pz(members[0], 10, Some(10)))),
        Ok(PartyXpDecision::Shares(_))
    ));
    assert_eq!(eval(None), Err(PartyXpError::PzObservationUnavailable));
    assert_eq!(
        eval(Some(pz(members[0], 9, None))),
        Err(PartyXpError::PzObservationUnavailable)
    );
    let mut sample = pz(members[0], 10, None);
    sample.session = GameSessionId::decode(&id(99)).expect("different");
    assert_eq!(
        eval(Some(sample)),
        Err(PartyXpError::PzObservationUnavailable)
    );
}
#[test]
fn party_xp_toggle_solo_and_activity_scope() {
    let members = [member(10, 100, 0, 0, 0)];
    let mut s = source(&members);
    let mut activity = active(&members, 0);
    off(
        evaluate(
            &s,
            channel(),
            &members,
            false,
            &activity,
            None,
            time(0),
            ExactI64::new(0),
        ),
        SharingOff::Disabled,
    );
    s.party_id = None;
    off(decide(&s, &members, &activity, 0), SharingOff::Solo);
    s.party_id = Some(id(2));
    activity[0].stamp.as_mut().expect("stamp").channel =
        ChannelId::decode(&id(4)).expect("other channel");
    assert_eq!(
        decide(&s, &members, &activity, 0),
        Err(PartyXpError::ActivityScopeChanged)
    );
}
#[test]
fn party_xp_roster_and_fifty_principal_bound() {
    let members: Vec<_> = (10..60)
        .map(|seed| member(seed, u32::MAX, 0, 0, 0))
        .collect();
    let PartyXpDecision::Shares(shares) =
        decide(&source(&members), &members, &active(&members, 0), 0).expect("plan")
    else {
        panic!("eligible")
    };
    assert_eq!(shares.len(), 50);
    assert!(shares.iter().all(|s| s.amount.get() == 3));
    let mut extra = members.clone();
    extra.push(member(60, u32::MAX, 0, 0, 0));
    assert_eq!(
        decide(&source(&extra), &extra, &active(&members, 0), 0),
        Err(PartyXpError::InvalidRoster)
    );
    let mut duplicate = members.clone();
    duplicate[1] = duplicate[0];
    assert_eq!(
        decide(&source(&members), &duplicate, &active(&members, 0), 0),
        Err(PartyXpError::InvalidRoster)
    );
    let mut unknown = members.clone();
    unknown[1].character = id(99);
    assert_eq!(
        decide(&source(&members), &unknown, &active(&members, 0), 0),
        Err(PartyXpError::InvalidRoster)
    );
}
#[test]
fn party_xp_equal_rounding_has_no_addition_overflow_or_redistribution() {
    let members = [member(10, 100, 0, 0, 0), member(11, 100, 0, 0, 0)];
    let s = source(&members);
    let activity = active(&members, 0);
    for (pool, each) in [
        (0, 0),
        (1, 1),
        (100, 50),
        (101, 51),
        (i64::MAX, i64::MAX / 2 + 1),
    ] {
        let PartyXpDecision::Shares(shares) = evaluate(
            &s,
            channel(),
            &members,
            true,
            &activity,
            Some(pz(members[0], 0, None)),
            time(0),
            ExactI64::new(pool),
        )
        .expect("plan") else {
            panic!("eligible")
        };
        assert_eq!(shares[0].amount.get(), each);
        assert_eq!(shares[1].amount.get(), each);
    }
    assert_eq!(
        evaluate(
            &s,
            channel(),
            &members,
            true,
            &activity,
            Some(pz(members[0], 0, None)),
            time(0),
            ExactI64::new(-1)
        ),
        Err(PartyXpError::InvalidPool)
    );
}

#[test]
fn party_xp_restarted_activity_does_not_grant_sharing() {
    let members = [member(10, 100, 0, 0, 0)];
    off(
        decide(
            &source(&members),
            &members,
            &[PartyXpActivity::default()],
            0,
        ),
        SharingOff::Inactive,
    );
}

#[test]
fn party_xp_solo_attack_survives_join_and_party_change_without_fabricated_membership() {
    let members = [member(10, 100, 0, 0, 0)];
    let mut solo = source(&members);
    solo.party_id = None;
    solo.revision = None;
    solo.leader = None;
    solo.members.clear();
    let mut cell = PartyXpActivity::default();
    cell.record_members(
        &solo,
        channel(),
        &members,
        PartyActivityEvent::AttackedAggressiveMonster {
            attacker: members[0].character,
            session: members[0].session,
        },
        time(1),
    )
    .expect("actual qualified solo attack data");
    let joined = source(&members);
    assert!(matches!(
        decide(&joined, &members, &[cell], 2),
        Ok(PartyXpDecision::Shares(_))
    ));
    let mut changed_party = joined;
    changed_party.party_id = Some(id(9));
    changed_party.revision = Some(7);
    assert!(matches!(
        decide(&changed_party, &members, &[cell], 3),
        Ok(PartyXpDecision::Shares(_))
    ));
    let before = cell;
    assert_eq!(
        cell.record_members(
            &solo,
            channel(),
            &members,
            PartyActivityEvent::HealedMember {
                healer: members[0].character,
                session: members[0].session,
                healed: members[0].character
            },
            time(4)
        ),
        Err(PartyXpError::InvalidRoster)
    );
    assert_eq!(cell, before);
}

#[test]
fn party_xp_cell_cannot_rebind_world_channel_or_session_and_duplicates_refuse() {
    let members = [member(10, 100, 0, 0, 0)];
    let s = source(&members);
    let mut cell = active(&members, 1)[0];
    let event = PartyActivityEvent::AttackedAggressiveMonster {
        attacker: members[0].character,
        session: members[0].session,
    };
    let before = cell;
    assert_eq!(
        cell.record_members(
            &s,
            ChannelId::decode(&id(4)).expect("other channel"),
            &members,
            event,
            time(2)
        ),
        Err(PartyXpError::ActivityScopeChanged)
    );
    assert_eq!(cell, before);
    let mut other = s;
    other.world = id(5);
    assert_eq!(
        cell.record_members(&other, channel(), &members, event, time(2)),
        Err(PartyXpError::ActivityScopeChanged)
    );
    assert_eq!(cell, before);
    assert_eq!(
        decide(&source(&members), &members, &[cell, cell], 2),
        Err(PartyXpError::InvalidRoster)
    );
}
