use super::*;

#[test]
fn unlock_requests_carry_the_seen_stage_or_the_servers_answer() {
    let view = cyclopedia();
    assert_eq!(
        view.unlock_request(index(3)),
        Ok(CharmUnlockStageIntent {
            charm: index(3),
            expected_stage: 1
        })
    );
    assert_eq!(
        view.unlock_request(index(1)),
        Ok(CharmUnlockStageIntent {
            charm: index(1),
            expected_stage: 0
        })
    );
    assert_eq!(
        view.unlock_request(index(2)),
        Err(CharmUnlockDisposition::NotEnoughMinorCharmEchoes)
    );
    assert_eq!(
        view.unlock_request(index(4)),
        Err(CharmUnlockDisposition::StageMismatch)
    );
    assert_eq!(
        view.unlock_request(index(5)),
        Err(CharmUnlockDisposition::UnknownCharm)
    );
    let mut poor = cyclopedia();
    poor.apply_charm_view(CharmView {
        charms: vec![charm(1, CharmKind::Major, 0, None)],
        charm_points_available: 99,
        minor_charm_echoes_available: 1000,
        assignment_slot_limit: None,
    });
    assert_eq!(
        poor.unlock_request(index(1)),
        Err(CharmUnlockDisposition::NotEnoughCharmPoints)
    );
}

#[test]
fn assign_requests_follow_the_stage_hint_and_never_unassign() {
    let view = cyclopedia();
    // Minor charm 2 needs stage 2: races 2 and 3.
    assert_eq!(
        view.assignable_races(index(2))
            .map(NonZeroU32::get)
            .collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(
        view.assign_request(index(2), index(2)),
        Ok(CharmAssignIntent {
            charm: index(2),
            race: index(2)
        })
    );
    assert_eq!(
        view.assign_request(index(2), index(1)),
        Err(CharmAssignDisposition::RaceStageTooLow)
    );
    // A race with no kills is not in the view.
    assert_eq!(
        view.assign_request(index(2), index(50)),
        Err(CharmAssignDisposition::RaceStageTooLow)
    );
    // Major charm 3 needs a complete entry, but it is already assigned (unassign is CHARM-6).
    assert_eq!(
        view.assign_request(index(3), index(3)),
        Err(CharmAssignDisposition::AlreadyAssigned)
    );
    assert_eq!(
        view.assign_request(index(1), index(3)),
        Err(CharmAssignDisposition::CharmLocked)
    );
    assert_eq!(
        view.assign_request(index(9), index(3)),
        Err(CharmAssignDisposition::UnknownCharm)
    );
    assert_eq!(view.assignable_races(index(9)).count(), 0);
}

#[test]
fn a_full_slot_limit_blocks_assign_and_no_limit_never_does() {
    let mut view = cyclopedia();
    let mut charms = CharmView {
        charms: view.charms.charms.clone(),
        charm_points_available: 150,
        minor_charm_echoes_available: 50,
        assignment_slot_limit: Some(index(1)),
    };
    view.apply_charm_view(charms.clone());
    assert_eq!(
        view.assign_request(index(2), index(2)),
        Err(CharmAssignDisposition::AssignmentSlotsFull)
    );
    assert!(view.charm_rows().all(|row| !row.can_assign));
    // The Charm Expansion: no limit.
    charms.assignment_slot_limit = None;
    view.apply_charm_view(charms);
    assert_eq!(
        view.assign_request(index(2), index(2)),
        Ok(CharmAssignIntent {
            charm: index(2),
            race: index(2)
        })
    );
}

#[test]
fn every_result_has_a_feedback_line() {
    let mut view = Cyclopedia::new();
    assert_eq!(view.feedback(), None);
    view.record_unlock(CharmUnlockDisposition::Rejected);
    assert_eq!(view.feedback(), Some("Charms unavailable"));
    view.record_assign(CharmAssignDisposition::Assigned);
    assert_eq!(view.feedback(), Some("Charm assigned"));
    for disposition in [
        CharmUnlockDisposition::Unlocked,
        CharmUnlockDisposition::NotEnoughCharmPoints,
        CharmUnlockDisposition::NotEnoughMinorCharmEchoes,
        CharmUnlockDisposition::StageMismatch,
        CharmUnlockDisposition::UnknownCharm,
        CharmUnlockDisposition::Rejected,
    ] {
        assert!(!unlock_feedback_text(disposition).is_empty());
    }
    for disposition in [
        CharmAssignDisposition::Assigned,
        CharmAssignDisposition::CharmLocked,
        CharmAssignDisposition::AlreadyAssigned,
        CharmAssignDisposition::RaceStageTooLow,
        CharmAssignDisposition::RaceCharmLimit,
        CharmAssignDisposition::UnknownCharm,
        CharmAssignDisposition::UnknownRace,
        CharmAssignDisposition::Rejected,
        CharmAssignDisposition::AssignmentSlotsFull,
    ] {
        assert!(!assign_feedback_text(disposition).is_empty());
    }
}
