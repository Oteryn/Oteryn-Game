#[test]
fn a_full_store_fails_the_forward_before_commit_and_unchanged_forwards_reserve_nothing()
-> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::new(1, 1)?)?;
    scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    let wall = PlacementKey::new(WALL_A)?;
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    assert_eq!(apply_plain(runtime, MEND)?, "COMMITTED");

    // Full store: the whole forward fails before commit.
    let occurrence = forward_child("oteryn:occurrence/wall/2", None)?;
    let action = LoweredActionId::new(WALL_ACTION)?;
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    let crack = operation(runtime, CRACK, runtime.revision())?;
    let full = driver.apply_forward(
        runtime,
        &occurrence,
        &revisions()?,
        &action,
        &crack,
        &BTreeSet::new(),
    );
    assert!(matches!(full, Err(RevertError::CapacityExceeded)));
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
    assert_eq!(driver.record_count(), 1);

    // An unchanged forward never reaches the capacity check: STALE_STATE, not a failure.
    let runtime = runtimes.get_mut(&wall).ok_or(fixture("runtime"))?;
    let stale = operation(runtime, CRACK, 0)?;
    let result = driver.apply_forward(
        runtime,
        &forward_child("oteryn:occurrence/wall/3", None)?,
        &revisions()?,
        &action,
        &stale,
        &BTreeSet::new(),
    )?;
    assert_eq!(result.outcome.disposition(), "STALE_STATE");
    assert_eq!(result.scheduled, None);
    assert_eq!(driver.record_count(), 1);
    Ok(())
}

#[test]
fn an_action_without_a_duration_schedules_nothing() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let outcome = forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        "oteryn:encounter/timed-wall/other/0",
        CRACK,
    )?;
    assert_eq!(outcome.outcome.disposition(), "COMMITTED");
    assert_eq!(outcome.scheduled, None);
    assert_eq!(driver.record_count(), 0);
    Ok(())
}

#[test]
fn two_anchors_fire_independently_in_scheduling_order() -> TestResult {
    let content = wall_content(
        &[(CRACK, WALL_ACTION, 1_000)],
        &[(WALL_A, 100), (WALL_B, 300)],
        "lock:r1",
    )?;
    let mut runtimes = walls(&content, &[WALL_A, WALL_B])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    // Equal deadlines and revisions; B is scheduled first but sorts last by identity.
    let on_b = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_B,
        "oteryn:occurrence/z",
        WALL_ACTION,
        CRACK,
    )?)?;
    let on_a = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/a",
        WALL_ACTION,
        CRACK,
    )?)?;
    assert!(on_a < on_b);
    clock.advance(millis(1_000))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    let order = report
        .fired
        .iter()
        .map(|(id, outcome)| (id.clone(), outcome.committed()))
        .collect::<Vec<_>>();
    assert_eq!(order, vec![(on_b, true), (on_a, true)]);
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
    assert_eq!(state_of(&runtimes, WALL_B)?, at(WALL_SEALED, 2));
    Ok(())
}

#[test]
fn a_mutually_timed_pair_fires_once_and_stops() -> TestResult {
    let content = wall_content(
        &[(CRACK, WALL_ACTION, 1_000), (MEND, WALL_ACTION, 1_000)],
        &[(WALL_A, 100)],
        "lock:r1",
    )?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    clock.advance(millis(1_000))?;
    let report = driver.wake(&mut runtimes, &BTreeSet::new())?;
    assert_eq!(report.fired.len(), 1);
    assert_eq!(driver.issuer().minted, 2);
    assert_eq!(driver.record_count(), 1);
    assert_eq!(driver.next_deadline(), None);
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_SEALED, 2));
    Ok(())
}

#[test]
fn an_occupied_revert_is_refused_once_and_never_retried() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let id = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    let occupied = runtimes
        .get(&PlacementKey::new(WALL_A)?)
        .ok_or(fixture("runtime"))?
        .collision_cells()
        .clone();
    clock.advance(millis(1_000))?;
    let report = driver.wake(&mut runtimes, &occupied)?;
    let [(_, outcome)] = report.fired.as_slice() else {
        return Err("expected one fired revert".into());
    };
    assert_eq!(disposition(outcome), Some("OCCUPIED"));
    assert!(
        driver
            .wake(&mut runtimes, &BTreeSet::new())?
            .fired
            .is_empty()
    );
    assert_eq!(
        driver.present(&id, &mut runtimes, &BTreeSet::new())?,
        RevertPresentation::Terminal(outcome.clone())
    );
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));
    Ok(())
}

#[test]
fn a_bounded_wake_leaves_the_remainder_for_a_later_wake() -> TestResult {
    let content = wall_content(
        &[(CRACK, WALL_ACTION, 1_000)],
        &[(WALL_A, 100), (WALL_B, 300)],
        "lock:r1",
    )?;
    let mut runtimes = walls(&content, &[WALL_A, WALL_B])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::new(4, 1)?)?;
    for (wall, occurrence) in [
        (WALL_A, "oteryn:occurrence/a"),
        (WALL_B, "oteryn:occurrence/b"),
    ] {
        scheduled(forward(
            &mut driver,
            &mut runtimes,
            wall,
            occurrence,
            WALL_ACTION,
            CRACK,
        )?)?;
    }
    clock.advance(millis(1_000))?;
    let first = driver.wake(&mut runtimes, &BTreeSet::new())?;
    assert_eq!((first.fired.len(), first.more_due), (1, true));
    let second = driver.wake(&mut runtimes, &BTreeSet::new())?;
    assert_eq!((second.fired.len(), second.more_due), (1, false));
    assert!(RevertDriverLimits::new(1, MAX_DUE_REVERTS_PER_WAKE + 1).is_err());
    assert!(RevertDriverLimits::new(MAX_REVERT_RECORDS_PER_SCOPE_GENERATION + 1, 1).is_err());
    Ok(())
}

#[test]
fn only_the_drivers_own_clock_decides_what_is_due() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let unrelated = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    // No driver call takes a clock: another clock's origin cannot make the record due.
    unrelated.advance(millis(1_000_000))?;
    assert!(
        driver
            .wake(&mut runtimes, &BTreeSet::new())?
            .fired
            .is_empty()
    );
    clock.advance(millis(1_000))?;
    assert_eq!(driver.wake(&mut runtimes, &BTreeSet::new())?.fired.len(), 1);
    Ok(())
}

#[test]
fn ordinal_exhaustion_is_scope_terminal_and_leaves_the_record_pending() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut driver = driver(&clock, RevertDriverLimits::registered())?;
    let id = scheduled(forward(
        &mut driver,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    driver.issuer_mut_for_test().next = None;
    clock.advance(millis(1_000))?;
    assert!(matches!(
        driver.wake(&mut runtimes, &BTreeSet::new()),
        Err(RevertError::ScopeTerminal)
    ));
    assert!(matches!(
        driver.lifecycle(&id),
        Some(RevertLifecycle::Pending(_))
    ));
    assert_eq!(state_of(&runtimes, WALL_A)?, at(WALL_CRACKED, 1));
    assert!(matches!(
        driver.wake(&mut runtimes, &BTreeSet::new()),
        Err(RevertError::ScopeTerminal)
    ));
    Ok(())
}

#[test]
fn a_fresh_driver_after_scope_restart_holds_no_records() -> TestResult {
    let content = wall_content(&[(CRACK, WALL_ACTION, 1_000)], &[(WALL_A, 100)], "lock:r1")?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    let clock = ManualClock::new(Moment::ZERO);
    let mut first = driver(&clock, RevertDriverLimits::registered())?;
    let id = scheduled(forward(
        &mut first,
        &mut runtimes,
        WALL_A,
        "oteryn:occurrence/wall/1",
        WALL_ACTION,
        CRACK,
    )?)?;
    drop(first);
    let mut restarted = driver(&clock, RevertDriverLimits::registered())?;
    let mut runtimes = walls(&content, &[WALL_A])?;
    assert_eq!(restarted.lifecycle(&id), None);
    assert_eq!(restarted.next_deadline(), None);
    clock.advance(millis(1_000))?;
    assert!(
        restarted
            .wake(&mut runtimes, &BTreeSet::new())?
            .fired
            .is_empty()
    );
    assert!(matches!(
        restarted.present(&id, &mut runtimes, &BTreeSet::new()),
        Err(RevertError::UnknownOccurrence)
    ));
    Ok(())
}

#[test]
fn a_destination_must_resolve_to_exactly_one_placement_in_the_bound_world() -> TestResult {
    // #1133 carry-over (a).
    let (content, revert) = duke_content(DUKE)?;
    let anchor = duke_anchor()?;
    let rearm = duke_rearm();
    let transitions = [OPEN_TELEPORTER, &rearm, revert.as_str()];
    let reward = duke_destination("reward_destination")?;

    let mut duplicate = content.clone();
    duplicate.placements.push(placement_at(
        &content,
        reward.clone(),
        marker_definition()?,
        400,
    )?);
    assert!(matches!(
        bind_at(&duplicate, &anchor, &transitions, 1),
        Err(WorldRuntimeError::Content(ContentError::DuplicateKey(key))) if key == reward.as_str()
    ));

    let mut cross_world = content.clone();
    let marker = cross_world
        .placements
        .iter_mut()
        .find(|placement| placement.key == reward)
        .ok_or(fixture("reward marker"))?;
    marker.address.world_id = world(2)?;
    assert!(matches!(
        bind_at(&cross_world, &anchor, &transitions, 1),
        Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(_)))
    ));

    let mut cross_frame = content.clone();
    let marker = cross_frame
        .placements
        .iter_mut()
        .find(|placement| placement.key == reward)
        .ok_or(fixture("reward marker"))?;
    marker.address.coordinate_frame = CoordinateFrameRef::new("another-frame")?;
    assert!(matches!(
        bind_at(&cross_frame, &anchor, &transitions, 1),
        Err(WorldRuntimeError::Content(ContentError::InvalidArtifact(_)))
    ));

    assert!(bind_at(&content, &anchor, &transitions, 1).is_ok());
    Ok(())
}

#[test]
fn two_revert_destination_occurrences_on_one_forward_stay_rejected() -> TestResult {
    // #1133 carry-over (c): each occurrence gets its own qualifying inverse, so `bind`
    // finds two and rejects the placement as ambiguous, fail-closed.
    let mut encounter: Value = serde_json::from_str(DUKE)?;
    let actions = encounter
        .pointer_mut("/rules/0/actions")
        .and_then(Value::as_array_mut)
        .ok_or(fixture("duke actions"))?;
    let first = actions.first().cloned().ok_or(fixture("duke action"))?;
    actions.push(first);
    let (content, revert) = duke_content(&serde_json::to_string(&encounter)?)?;
    let second_revert = format!("{}/1/revert", DUKE_ACTION.trim_end_matches("/0"));
    let second_rearm = format!("{}/1/rearm", DUKE_ACTION.trim_end_matches("/0"));
    assert!(matches!(
        bind_at(
            &content,
            &duke_anchor()?,
            &[
                OPEN_TELEPORTER,
                &duke_rearm(),
                &second_rearm,
                revert.as_str(),
                &second_revert,
            ],
            1
        ),
        Err(WorldRuntimeError::InvalidBinding(
            "revert_after_ms transition has an ambiguous bound inverse at this placement"
        ))
    ));
    Ok(())
}
